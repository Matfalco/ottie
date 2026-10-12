//! `ottie serve`: the daemon.

use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;
use directories::UserDirs;
use ottie_api::{Token, UiSource};
use ottie_core::{CategoryId, HttpBackend, Manager, Store};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

use crate::config::{self, CONFIG_FILE, DATABASE_FILE, DaemonArgs, Dirs, FileConfig};
use crate::token;

#[derive(Args, Debug)]
pub struct ServeArgs {
    #[command(flatten)]
    pub daemon: DaemonArgs,
    /// Serve the web UI from this folder instead of the one built into Ottie
    #[arg(long, env = "OTTIE_UI_DIR", value_name = "FOLDER")]
    pub ui_dir: Option<PathBuf>,
    /// For a program that starts Ottie (the desktop app): print `ready` once
    /// listening, and stop when standard input closes.
    #[arg(long, hide = true)]
    pub managed: bool,
}

pub async fn run(args: ServeArgs) -> Result<()> {
    // Watched from the start: if the app quits while we start, we stop too.
    let parent_gone = args.managed.then(stdin_closed);
    // Log to stderr; `OTTIE_LOG=debug` (or `ottie_core=debug`…) shows more.
    let filter = EnvFilter::try_from_env("OTTIE_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    let daemon = &args.daemon;
    let dirs = Dirs::resolve(&daemon.dirs)?;
    let file = FileConfig::load(&dirs.config.join(CONFIG_FILE))?;
    let addr = config::listen_addr(daemon.address, daemon.port, &file);
    let token = token::load_or_create(&dirs.config)?;

    // The usual failure: another Ottie already has the port. Checked before
    // the database is opened, or this one would resume its downloads too.
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("can't listen on {addr} (is Ottie already running?)"))?;

    std::fs::create_dir_all(&dirs.data)
        .with_context(|| format!("can't create {}", dirs.data.display()))?;
    let database = dirs.data.join(DATABASE_FILE);
    let store = Store::open(&database)
        .await
        .with_context(|| format!("can't open the database {}", database.display()))?;
    let manager = Manager::start(HttpBackend::new()?, store).await?;
    default_general_folder(&manager, downloads_folder()).await?;

    if !addr.ip().is_loopback() {
        tracing::warn!(%addr, "the API is open to other computers: keep it behind a VPN or reverse proxy");
    }
    tracing::info!(%addr, data = %dirs.data.display(), "Ottie {} is running", env!("CARGO_PKG_VERSION"));
    // Only for a person at a terminal; never into logs (systemd, Docker).
    if std::io::stdout().is_terminal() {
        println!(
            "Ottie is running. Open it, already signed in:\n  {}\n\
             (or run `ottie open`). Ctrl+C stops Ottie; downloads continue at the next start.",
            config::ui_url(addr, &token)
        );
    }

    if args.managed {
        // Connections already wait in the listener's queue: the app can open
        // its window now.
        println!("ready");
    }

    let ui = match args.ui_dir {
        Some(folder) => UiSource::Folder(folder),
        None => UiSource::Embedded,
    };
    let shutdown = CancellationToken::new();
    let app = ottie_api::router(manager.clone(), Token::new(token), shutdown.clone(), ui);
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            stop_requested(parent_gone).await;
            // Close event streams, or the server would wait for them.
            shutdown.cancel();
        })
        .await
        .context("the API server failed")?;

    tracing::info!("stopping: saving downloads");
    manager.shutdown().await?;
    Ok(())
}

/// Until the user picks a folder for General, it's the OS's Downloads
/// folder, so a download can be added with just a link.
async fn default_general_folder(manager: &Manager, downloads: Option<PathBuf>) -> Result<()> {
    let categories = manager.categories().await?;
    let Some(general) = categories.into_iter().find(|c| c.id == CategoryId::GENERAL) else {
        return Ok(());
    };
    if general.settings.folder.is_some() {
        return Ok(());
    }
    let Some(downloads) = downloads else {
        tracing::warn!("no Downloads folder found: every download must say where to go");
        return Ok(());
    };
    tracing::info!(folder = %downloads.display(), "downloads go to this folder by default");
    let mut settings = general.settings;
    settings.folder = Some(downloads);
    manager
        .update_category(CategoryId::GENERAL, settings)
        .await?;
    Ok(())
}

/// The user's Downloads folder, or `~/Downloads` where the OS names none
/// (e.g. a Linux server without desktop folders).
fn downloads_folder() -> Option<PathBuf> {
    let user = UserDirs::new()?;
    let named = user.download_dir().map(Path::to_owned);
    Some(named.unwrap_or_else(|| user.home_dir().join("Downloads")))
}

/// `--managed`: cancelled once standard input closes, i.e. the program that
/// started us quit (or crashed: the OS closes its end of the pipe then too).
fn stdin_closed() -> CancellationToken {
    let closed = CancellationToken::new();
    let signal = closed.clone();
    // A plain thread, not a tokio task: a read that's still waiting would
    // hold up the runtime's shutdown after Ctrl+C.
    std::thread::spawn(move || {
        let _ = std::io::copy(&mut std::io::stdin().lock(), &mut std::io::sink());
        signal.cancel();
    });
    closed
}

/// Ctrl+C, (Unix) the SIGTERM that systemd and Docker send, or (`--managed`)
/// standard input closing.
async fn stop_requested(parent_gone: Option<CancellationToken>) {
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            // Without a handler we can't stop gracefully; keep running.
            tracing::warn!(error = %err, "can't listen for Ctrl+C");
            std::future::pending::<()>().await;
        }
    };
    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Ok(mut sigterm) => {
                sigterm.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    let parent_gone = async {
        match parent_gone {
            Some(closed) => closed.cancelled().await,
            None => std::future::pending().await,
        }
    };
    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
        () = parent_gone => tracing::info!("the program that started Ottie has quit"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn general_gets_the_downloads_folder_once() {
        let store = Store::open_in_memory().await.unwrap();
        let manager = Manager::start(HttpBackend::new().unwrap(), store)
            .await
            .unwrap();
        let folder = |m: Vec<ottie_core::Category>| m[0].settings.folder.clone();

        default_general_folder(&manager, Some("/dl".into()))
            .await
            .unwrap();
        assert_eq!(
            folder(manager.categories().await.unwrap()),
            Some("/dl".into())
        );
        // A folder already set (by the user, later) is kept.
        default_general_folder(&manager, Some("/other".into()))
            .await
            .unwrap();
        assert_eq!(
            folder(manager.categories().await.unwrap()),
            Some("/dl".into())
        );
    }
}
