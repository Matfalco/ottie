//! `ottie open`: the web UI in your browser, already signed in.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use tokio::net::TcpStream;

use crate::config::{self, CONFIG_FILE, DaemonArgs, Dirs, FileConfig};
use crate::token;

/// Rewritten on every `ottie open`; readable only by you, like the token.
const REDIRECT_PAGE: &str = "open.html";

pub async fn run(args: &DaemonArgs) -> Result<()> {
    let dirs = Dirs::resolve(&args.dirs)?;
    let file = FileConfig::load(&dirs.config.join(CONFIG_FILE))?;
    let addr = config::listen_addr(args.address, args.port, &file);
    let local = config::local(addr);

    let probe = tokio::time::timeout(Duration::from_secs(2), TcpStream::connect(local)).await;
    if !matches!(probe, Ok(Ok(_))) {
        bail!("Ottie isn't running at {local}: start it with `ottie serve`");
    }

    // The link holds the token. Handing it to the browser on its command
    // line would show it to other users of this computer (in the process
    // list), so the browser opens a private page that redirects instead.
    let token = token::load_or_create(&dirs.config)?;
    let page = dirs.config.join(REDIRECT_PAGE);
    token::write_private(&page, &redirect(&config::ui_url(addr, &token)), true)
        .with_context(|| format!("can't write {}", page.display()))?;
    open::that_detached(&page).context("can't start a browser")?;
    println!("Opening Ottie in your browser…");
    Ok(())
}

fn redirect(url: &str) -> String {
    format!(
        "<!doctype html>\n<meta charset=\"utf-8\">\n\
         <meta http-equiv=\"refresh\" content=\"0;url={url}\">\n\
         <title>Ottie</title>\n<a href=\"{url}\">Open Ottie</a>\n"
    )
}
