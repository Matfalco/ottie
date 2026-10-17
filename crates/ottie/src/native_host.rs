//! The browser extension's native messaging host (docs/decisions/0055): a
//! browser starts `ottie` itself when the extension asks, with the
//! extension's address as its argument, and they exchange one JSON message
//! each over standard input and output (each after its length, 4 bytes in
//! this computer's byte order).
//!
//! It signs the browser in to Ottie on this computer (a device token of its
//! own, named after the browser) and starts Ottie when it isn't running.
//! Only the extensions named in the host's manifest (`extension.rs`) can
//! start it; anyone who can run `ottie` here can read the token file anyway.

use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use clap::Args;
use ottie_api::{AddDevice, AddedDevice};
use serde::{Deserialize, Serialize};

use crate::config::{self, CONFIG_FILE, DaemonArgs, Dirs, FileConfig};
use crate::device::Client;

/// What the host is registered as, in every browser.
pub const HOST_NAME: &str = "io.github.matfalco.ottie";
/// Firefox's ID for the extension (`extension/src/manifest.ts`).
pub const GECKO_ID: &str = "ottie@matfalco.github.io";

/// Messages from the extension are small; anything bigger isn't one.
const MOST_BYTES: u32 = 64 * 1024;
/// How long Ottie may take to start answering.
const STARTUP: Duration = Duration::from_secs(20);

#[derive(Args, Debug)]
pub struct NativeHostArgs {
    #[command(flatten)]
    daemon: DaemonArgs,
}

/// Whether a browser started `ottie` as the host: Chromium-based browsers
/// pass `chrome-extension://<id>/` (and on Windows `--parent-window=…`),
/// Firefox the manifest's path and the extension's ID.
pub fn started_by_browser(args: &[String]) -> bool {
    args.get(1)
        .is_some_and(|first| first.starts_with("chrome-extension://"))
        || args.get(2).is_some_and(|second| second == GECKO_ID)
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Request {
    /// A token of the browser's own, after starting Ottie if needed.
    Pair { browser: String },
    /// Start Ottie if it isn't running.
    Start,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Answer {
    Paired {
        address: String,
        token: String,
    },
    Started {
        address: String,
    },
    /// English, for the extension's console; the extension says it its way.
    Error {
        message: String,
    },
}

/// Answers one message, then ends (the extension sends one per start).
pub async fn run(args: NativeHostArgs) -> Result<()> {
    let mut stdin = io::stdin().lock();
    let request = read_message(&mut stdin)?;
    let answer = match answer(&args.daemon, request).await {
        Ok(answer) => answer,
        Err(err) => Answer::Error {
            message: format!("{err:#}"),
        },
    };
    write_message(&mut io::stdout().lock(), &answer)
}

async fn answer(daemon: &DaemonArgs, request: Request) -> Result<Answer> {
    let addr = ensure_running(daemon)?;
    let address = format!("http://{}", config::local(addr));
    match request {
        Request::Start => Ok(Answer::Started { address }),
        Request::Pair { browser } => {
            let client = Client::new(daemon)?;
            let body = AddDevice {
                name: device_name(&browser),
                read_only: Some(false),
            };
            let request = client.http.post(client.url("/access/devices")).json(&body);
            let added: AddedDevice = client.send(request).await?.json().await?;
            Ok(Answer::Paired {
                address,
                token: added.token,
            })
        }
    }
}

/// The device's name in Settings → Connection: the browser's ("Chrome",
/// "Firefox"), 1 to 60 characters.
fn device_name(browser: &str) -> String {
    let name: String = browser
        .chars()
        .filter(|c| !c.is_control())
        .take(60)
        .collect();
    let name = name.trim();
    if name.is_empty() {
        "Browser".to_owned()
    } else {
        name.to_owned()
    }
}

/// Where Ottie on this computer listens; started first if nothing answers.
fn ensure_running(daemon: &DaemonArgs) -> Result<SocketAddr> {
    let dirs = Dirs::resolve(&daemon.dirs)?;
    let file = FileConfig::load(&dirs.config.join(CONFIG_FILE))?;
    let addr = config::listen_addr(daemon.address, daemon.port, &file);
    if answers(addr) {
        return Ok(addr);
    }
    let exe = std::env::current_exe().context("can't find ottie itself")?;
    let log = dirs.data.join("ottie.log");
    start_alone(&exe, &log)?;
    let start = Instant::now();
    while start.elapsed() < STARTUP {
        if answers(addr) {
            return Ok(addr);
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    bail!(
        "Ottie didn't start within {} s (its log: {})",
        STARTUP.as_secs(),
        log.display()
    )
}

fn answers(addr: SocketAddr) -> bool {
    TcpStream::connect_timeout(&config::local(addr), Duration::from_secs(1)).is_ok()
}

/// `ottie serve`, on its own: it outlives the host and the browser. Its
/// standard input and output are never the host's (those are the browser's
/// messages).
/// It gets the host's environment, so the same `OTTIE_*` folders and port.
fn start_alone(exe: &Path, log: &Path) -> Result<()> {
    if let Some(folder) = log.parent() {
        std::fs::create_dir_all(folder)
            .with_context(|| format!("can't create {}", folder.display()))?;
    }
    let mut command = Command::new(exe);
    command.arg("serve").arg("--log-file").arg(log);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    detach(&mut command);
    match command.spawn() {
        Ok(_) => Ok(()),
        #[cfg(windows)]
        Err(_) => {
            // The browser's job doesn't let children leave it: start it
            // inside, without a window (it may stop with the browser).
            use std::os::windows::process::CommandExt;
            command.creation_flags(CREATE_NO_WINDOW);
            command
                .spawn()
                .map(|_| ())
                .with_context(|| format!("can't start {}", exe.display()))
        }
        #[cfg(not(windows))]
        Err(err) => Err(err).with_context(|| format!("can't start {}", exe.display())),
    }
}

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Out of the browser's reach: its own process group on Unix (a signal to
/// the host's group doesn't stop it); on Windows out of the job object the
/// browser puts its hosts in, which Firefox allows, without a window.
fn detach(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
        command.creation_flags(CREATE_NO_WINDOW | CREATE_BREAKAWAY_FROM_JOB);
    }
}

fn read_message(input: &mut impl Read) -> Result<Request> {
    let mut length = [0; 4];
    input
        .read_exact(&mut length)
        .context("no message from the browser")?;
    let length = u32::from_ne_bytes(length);
    if length > MOST_BYTES {
        bail!("a message of {length} bytes from the browser");
    }
    let mut body = vec![0; length as usize];
    input.read_exact(&mut body).context("a message cut short")?;
    serde_json::from_slice(&body).context("not a message Ottie knows")
}

fn write_message(output: &mut impl Write, answer: &Answer) -> Result<()> {
    let body = serde_json::to_vec(answer)?;
    let length = u32::try_from(body.len()).context("an answer too long")?;
    output.write_all(&length.to_ne_bytes())?;
    output.write_all(&body)?;
    output.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn framed(json: &str) -> Vec<u8> {
        let mut bytes = u32::try_from(json.len()).unwrap().to_ne_bytes().to_vec();
        bytes.extend_from_slice(json.as_bytes());
        bytes
    }

    #[test]
    fn browsers_are_recognised_by_what_they_pass() {
        let args = |list: &[&str]| list.iter().map(|&a| a.to_owned()).collect::<Vec<_>>();
        assert!(started_by_browser(&args(&[
            "ottie",
            "chrome-extension://njjhemepabjjkdpcdplfcgjiooggdpol/",
            "--parent-window=0"
        ])));
        assert!(started_by_browser(&args(&[
            "ottie",
            "/home/me/.mozilla/native-messaging-hosts/io.github.matfalco.ottie.json",
            GECKO_ID
        ])));
        assert!(!started_by_browser(&args(&["ottie", "serve"])));
        assert!(!started_by_browser(&args(&["ottie"])));
    }

    #[test]
    fn messages_go_after_their_length() {
        let mut input = io::Cursor::new(framed(r#"{"type":"pair","browser":"Chrome"}"#));
        assert_eq!(
            read_message(&mut input).unwrap(),
            Request::Pair {
                browser: "Chrome".to_owned()
            }
        );
        let mut input = io::Cursor::new(framed(r#"{"type":"start"}"#));
        assert_eq!(read_message(&mut input).unwrap(), Request::Start);

        let mut output = Vec::new();
        let answer = Answer::Started {
            address: "http://127.0.0.1:24068".to_owned(),
        };
        write_message(&mut output, &answer).unwrap();
        assert_eq!(
            output,
            framed(r#"{"type":"started","address":"http://127.0.0.1:24068"}"#)
        );
    }

    #[test]
    fn odd_messages_are_refused() {
        for bytes in [
            framed(r#"{"type":"steal"}"#),
            framed("not json"),
            vec![1, 2],
            (MOST_BYTES + 1).to_ne_bytes().to_vec(),
        ] {
            assert!(read_message(&mut io::Cursor::new(bytes)).is_err());
        }
    }

    #[test]
    fn a_device_is_named_after_its_browser() {
        assert_eq!(device_name("Firefox"), "Firefox");
        assert_eq!(device_name("  \n"), "Browser");
        assert_eq!(device_name(&"x".repeat(80)).chars().count(), 60);
    }
}
