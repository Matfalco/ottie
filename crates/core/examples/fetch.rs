//! Try the engine by hand: `cargo run -p ottie-core --example fetch -- <url> [folder]`
//! Ctrl+C pauses, Enter resumes.

use std::error::Error;
use std::io::Write;
use std::time::{Duration, Instant};

use ottie_core::{Download, Event, HttpConfig, Outcome, Throttle, build_client, run};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let url = args.next().ok_or("usage: fetch <url> [folder]")?.parse()?;
    let folder = args.next().unwrap_or_else(|| ".".to_owned());

    let client = build_client()?;
    let config = HttpConfig::default();
    let throttle = Throttle::unlimited();
    let mut dl = Download::new(url, folder)?;
    let mut stdin = BufReader::new(tokio::io::stdin()).lines();

    loop {
        let cancel = CancellationToken::new();
        let on_ctrl_c = cancel.clone();
        let ctrl_c = tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                on_ctrl_c.cancel();
            }
        });

        let mut last_print = Instant::now() - Duration::from_secs(1);
        let outcome = run(
            &client,
            &config,
            &mut dl,
            &throttle,
            &cancel,
            |event| match event {
                Event::Progress { downloaded, total } => {
                    if last_print.elapsed() >= Duration::from_millis(200) {
                        last_print = Instant::now();
                        print_progress(downloaded, total);
                    }
                }
                Event::Started { file_name, .. } => println!("downloading {file_name}"),
                Event::Restarted(reason) => println!("\nrestarting from zero: {reason:?}"),
                Event::Checkpoint { .. } => {}
            },
        )
        .await;
        ctrl_c.abort();

        match outcome? {
            Outcome::Completed { path } => {
                print_progress(dl.downloaded, dl.total);
                println!("\nsaved to {}", path.display());
                return Ok(());
            }
            Outcome::Paused => {
                let note = if dl.resumable {
                    ""
                } else {
                    " (not resumable: will restart)"
                };
                println!("\npaused{note}. Press Enter to resume.");
                if stdin.next_line().await?.is_none() {
                    return Ok(());
                }
            }
        }
    }
}

fn print_progress(downloaded: u64, total: Option<u64>) {
    let mib = |b: u64| b as f64 / (1024.0 * 1024.0);
    match total {
        Some(t) if t > 0 => print!(
            "\r{:.1} / {:.1} MiB ({:.0}%)   ",
            mib(downloaded),
            mib(t),
            downloaded as f64 * 100.0 / t as f64
        ),
        _ => print!("\r{:.1} MiB   ", mib(downloaded)),
    }
    let _ = std::io::stdout().flush();
}
