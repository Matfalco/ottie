//! `ottie serve`: the daemon.

use std::io::IsTerminal;
use std::net::IpAddr;

use anyhow::{Context, Result};
use clap::Args;
use ottie_api::Token;
use ottie_core::{HttpBackend, Manager, Store};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

use crate::config::{self, CONFIG_FILE, DATABASE_FILE, DirArgs, Dirs, FileConfig};
use crate::token;

#[derive(Args, Debug)]
pub struct ServeArgs {
    #[command(flatten)]
    pub dirs: DirArgs,
    /// Address to listen on; 0.0.0.0 opens the API to other computers [default: 127.0.0.1]
    #[arg(long, env = "OTTIE_ADDRESS")]
    pub address: Option<IpAddr>,
    /// Port to listen on [default: 16889]
    #[arg(long, env = "OTTIE_PORT")]
    pub port: Option<u16>,
}

pub async fn run(args: ServeArgs) -> Result<()> {
    // Log to stderr; `OTTIE_LOG=debug` (or `ottie_core=debug`…) shows more.
    let filter = EnvFilter::try_from_env("OTTIE_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    let dirs = Dirs::resolve(&args.dirs)?;
    let file = FileConfig::load(&dirs.config.join(CONFIG_FILE))?;
    let addr = config::listen_addr(args.address, args.port, &file);
    let token = token::load_or_create(&dirs.config)?;
    std::fs::create_dir_all(&dirs.data)
        .with_context(|| format!("can't create {}", dirs.data.display()))?;
    let database = dirs.data.join(DATABASE_FILE);
    let store = Store::open(&database)
        .await
        .with_context(|| format!("can't open the database {}", database.display()))?;
    let manager = Manager::start(HttpBackend::new()?, store).await?;

    // The usual failure: another Ottie already has the port.
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("can't listen on {addr} (is Ottie already running?)"))?;
    if !addr.ip().is_loopback() {
        tracing::warn!(%addr, "the API is open to other computers: keep it behind a VPN or reverse proxy");
    }
    tracing::info!(%addr, data = %dirs.data.display(), "Ottie {} is running", env!("CARGO_PKG_VERSION"));
    // Only for a person at a terminal; never into logs (systemd, Docker).
    if std::io::stdout().is_terminal() {
        println!(
            "Ottie is running. API: http://{addr}/api/v1\n\
             Token: `ottie token` prints it (it's in {}).\n\
             Ctrl+C stops Ottie; downloads continue at the next start.",
            token::path(&dirs.config).display()
        );
    }

    let shutdown = CancellationToken::new();
    let app = ottie_api::router(manager.clone(), Token::new(token), shutdown.clone());
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            stop_requested().await;
            // Close event streams, or the server would wait for them.
            shutdown.cancel();
        })
        .await
        .context("the API server failed")?;

    tracing::info!("stopping: saving downloads");
    manager.shutdown().await?;
    Ok(())
}

/// Ctrl+C, or (Unix) the SIGTERM that systemd and Docker send.
async fn stop_requested() {
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
    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
}
