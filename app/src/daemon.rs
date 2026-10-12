//! The daemon this window shows: the one already running on this computer,
//! or `ottie serve --managed` started for the app (docs/decisions/0017).

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Stdio};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use reqwest::StatusCode;
use tauri::Url;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::{Child, ChildStdin, Command};

/// A first start creates the database; a large one takes a moment to load.
const STARTUP: Duration = Duration::from_secs(30);
/// Stopping saves every running download first.
const SHUTDOWN: Duration = Duration::from_secs(10);
/// The end of the daemon's log, kept to explain a failed start.
const KEPT_LINES: usize = 20;

/// Windows: start `ottie` without a console window of its own.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub struct Connection {
    /// The web UI, signed in: `http://127.0.0.1:16889/#token=…`.
    pub link: Url,
    /// `None` when a daemon was already running: not ours to stop.
    pub started: Option<Started>,
}

/// `ottie` sits next to the app: put there by the installer (Tauri's
/// `externalBin`), or built into the same `target/` folder in development.
pub fn sidecar() -> Result<PathBuf> {
    let app = std::env::current_exe().context("can't find where the app is")?;
    let ottie = app.with_file_name(format!("ottie{}", std::env::consts::EXE_SUFFIX));
    if !ottie.is_file() {
        bail!(
            "{} is missing (in development: cargo build -p ottie)",
            ottie.display()
        );
    }
    Ok(ottie)
}

pub async fn connect(ottie: &Path) -> Result<Connection> {
    let link = link(ottie).await?;
    // Starting first, not probing first: on Windows a connection to a port
    // nobody listens on takes 2 s to fail. `ottie serve` checks the port
    // before anything else, so a second one gives up at once.
    let failure = match Started::spawn(ottie).await {
        Ok(started) => {
            return Ok(Connection {
                link,
                started: Some(started),
            });
        }
        Err(failure) => failure,
    };
    // Usually: Ottie was already running (from a terminal, or as a service).
    if answers(&link).await {
        return Ok(Connection {
            link,
            started: None,
        });
    }
    Err(failure)
}

/// `ottie link`: the address and token, as `ottie serve` will use them.
async fn link(ottie: &Path) -> Result<Url> {
    let output = command(ottie)
        .arg("link")
        .stdin(Stdio::null())
        .output()
        .await
        .with_context(|| format!("can't run {}", ottie.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(reason(stderr.lines(), output.status));
    }
    let printed = String::from_utf8_lossy(&output.stdout);
    Url::parse(printed.trim()).with_context(|| format!("`ottie link` printed {printed:?}"))
}

/// Whether an Ottie daemon answers at the link's address. Refusing our token
/// counts: it's Ottie started with other folders, and its page asks for one.
async fn answers(link: &Url) -> bool {
    let token = link
        .fragment()
        .and_then(|f| f.strip_prefix("token="))
        .unwrap_or_default();
    let Ok(jobs) = link.join("/api/v1/jobs") else {
        return false;
    };
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build();
    let Ok(client) = client else {
        return false;
    };
    match client.get(jobs.as_str()).bearer_auth(token).send().await {
        Ok(answer) => matches!(answer.status(), StatusCode::OK | StatusCode::UNAUTHORIZED),
        Err(_) => false,
    }
}

fn command(ottie: &Path) -> Command {
    let mut command = Command::new(ottie);
    // Its log lines without colour codes.
    command.env("NO_COLOR", "1");
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

/// A daemon this app started. Closing its stdin tells it to stop; if the app
/// crashes, the OS closes it, so the daemon never outlives the app.
pub struct Started {
    child: Child,
    stdin: Option<ChildStdin>,
}

impl Started {
    async fn spawn(ottie: &Path) -> Result<Started> {
        let mut child = command(ottie)
            .args(["serve", "--managed"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // If we give up on it below.
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!("can't start {}", ottie.display()))?;
        let stdin = child.stdin.take();
        let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) else {
            bail!("no pipes to ottie serve");
        };
        let log = tauri::async_runtime::spawn(forward_log(stderr));

        let mut stdout = BufReader::new(stdout).lines();
        match tokio::time::timeout(STARTUP, stdout.next_line()).await {
            Ok(Ok(Some(line))) if line == "ready" => {
                // Keep reading: a daemon that writes to a closed pipe fails.
                tauri::async_runtime::spawn(async move {
                    while let Ok(Some(_)) = stdout.next_line().await {}
                });
                Ok(Started { child, stdin })
            }
            Ok(_) => {
                // It stopped: its last log lines say why.
                let status = child.wait().await?;
                let lines = log.await.unwrap_or_default();
                Err(anyhow!(reason(lines.iter().map(String::as_str), status)))
            }
            Err(_) => bail!("Ottie didn't start within {} s", STARTUP.as_secs()),
        }
    }

    /// Asks it to stop, which saves its downloads; kills it if that takes too
    /// long (the next start resumes from the files anyway).
    pub async fn stop(mut self) {
        drop(self.stdin.take());
        if tokio::time::timeout(SHUTDOWN, self.child.wait())
            .await
            .is_err()
        {
            let _ = self.child.kill().await;
        }
    }
}

/// The daemon's log goes on to the app's own stderr (seen in development);
/// the last lines are returned once it closes.
async fn forward_log(stderr: impl AsyncRead + Unpin) -> VecDeque<String> {
    let mut kept = VecDeque::with_capacity(KEPT_LINES);
    let mut lines = BufReader::new(stderr).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        eprintln!("{line}");
        if kept.len() == KEPT_LINES {
            kept.pop_front();
        }
        kept.push_back(line);
    }
    kept
}

/// Why `ottie` stopped: the error it printed last (`ottie: …`), else its exit status.
fn reason<'a>(stderr: impl DoubleEndedIterator<Item = &'a str>, status: ExitStatus) -> String {
    stderr
        .rev()
        .find_map(|line| line.strip_prefix("ottie: "))
        .map_or_else(|| format!("Ottie stopped ({status})"), str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reason_is_the_last_error_line() {
        let failed = failure_status();
        let log = "INFO starting\nottie: first\nottie: can't listen on 127.0.0.1:16889\nmore";
        assert_eq!(
            reason(log.lines(), failed),
            "can't listen on 127.0.0.1:16889"
        );
        assert!(reason("INFO only logs".lines(), failed).starts_with("Ottie stopped ("));
    }

    /// Exit code 1 on Unix (code × 256), 256 on Windows: a failure either way.
    fn failure_status() -> ExitStatus {
        #[cfg(unix)]
        use std::os::unix::process::ExitStatusExt;
        #[cfg(windows)]
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(256)
    }
}
