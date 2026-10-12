#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The real `ottie` binary, started the way a user would.

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use reqwest::StatusCode;

/// `ottie` with its folders inside `dir`.
fn ottie(dir: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ottie"));
    command
        .env("OTTIE_CONFIG_DIR", dir.join("config"))
        .env("OTTIE_DATA_DIR", dir.join("data"))
        .env_remove("OTTIE_ADDRESS")
        .env_remove("OTTIE_PORT");
    command
}

/// Stops the daemon even if the test fails.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

#[test]
fn the_token_is_created_once() {
    let dir = tempfile::tempdir().unwrap();
    let print = || {
        let output = ottie(dir.path()).arg("token").output().unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    };
    let token = print();
    assert_eq!(token.len(), 64);
    assert_eq!(print(), token);
    assert!(dir.path().join("config").join("token").is_file());
}

#[test]
fn a_bad_config_file_stops_the_daemon_with_a_reason() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("config")).unwrap();
    std::fs::write(dir.path().join("config").join("config.toml"), "prot = 1").unwrap();
    let output = ottie(dir.path()).arg("serve").output().unwrap();
    assert!(!output.status.success());
    let message = String::from_utf8(output.stderr).unwrap();
    assert!(message.contains("config.toml is not valid"), "{message}");
}

#[tokio::test]
async fn serve_answers_requests_that_carry_the_token() {
    let dir = tempfile::tempdir().unwrap();
    let port = free_port();
    let child = ottie(dir.path())
        .args(["serve", "--port", &port.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let _running = Running(child);

    let url = format!("http://127.0.0.1:{port}/api/v1/jobs");
    let token_file = dir.path().join("config").join("token");
    let client = reqwest::Client::new();
    let started = Instant::now();
    let token = loop {
        assert!(
            started.elapsed() < Duration::from_secs(15),
            "never answered"
        );
        if let Ok(token) = std::fs::read_to_string(&token_file) {
            let token = token.trim().to_owned();
            let answer = client.get(&url).bearer_auth(&token).send().await;
            if answer.is_ok_and(|a| a.status() == StatusCode::OK) {
                break token;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    };

    let anonymous = client.get(&url).send().await.unwrap();
    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);
    let jobs = client.get(&url).bearer_auth(&token).send().await.unwrap();
    assert_eq!(jobs.text().await.unwrap(), "[]");
    assert!(dir.path().join("data").join("ottie.db").is_file());
}

#[test]
fn open_explains_when_ottie_is_not_running() {
    let dir = tempfile::tempdir().unwrap();
    let port = free_port().to_string();
    let output = ottie(dir.path())
        .args(["open", "--port", &port])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let message = String::from_utf8(output.stderr).unwrap();
    assert!(message.contains("isn't running"), "{message}");
    assert!(message.contains("ottie serve"), "{message}");
}

#[test]
fn a_taken_port_stops_serve_before_it_opens_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = taken.local_addr().unwrap().port().to_string();
    let output = ottie(dir.path())
        .args(["serve", "--port", &port])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let message = String::from_utf8(output.stderr).unwrap();
    assert!(message.contains("already running"), "{message}");
    // Otherwise it would have resumed the other daemon's downloads.
    assert!(!dir.path().join("data").join("ottie.db").exists());
}

#[test]
fn link_prints_the_signed_in_address() {
    let dir = tempfile::tempdir().unwrap();
    let output = ottie(dir.path())
        .args(["link", "--port", "9123"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let token = std::fs::read_to_string(dir.path().join("config").join("token")).unwrap();
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("http://127.0.0.1:9123/#token={}\n", token.trim())
    );
}

/// What the desktop app does: wait for `ready`, then close stdin to stop it.
#[test]
fn managed_serve_says_ready_and_stops_when_stdin_closes() {
    use std::io::{BufRead, BufReader};

    let dir = tempfile::tempdir().unwrap();
    let port = free_port().to_string();
    let mut child = ottie(dir.path())
        .args(["serve", "--managed", "--port", &port])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let mut running = Running(child);

    let (lines, ready) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let _ = lines.send(line.unwrap());
        }
    });
    let first = ready.recv_timeout(Duration::from_secs(15)).unwrap();
    assert_eq!(first, "ready");
    // It answers right away.
    let answer = std::net::TcpStream::connect(format!("127.0.0.1:{port}"));
    assert!(answer.is_ok());

    drop(stdin);
    let started = Instant::now();
    let status = loop {
        if let Some(status) = running.0.try_wait().unwrap() {
            break status;
        }
        assert!(started.elapsed() < Duration::from_secs(15), "still running");
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(status.success());
    let logs = std::io::read_to_string(stderr).unwrap();
    assert!(
        logs.contains("the program that started Ottie has quit"),
        "{logs}"
    );
    assert!(logs.contains("saving downloads"), "{logs}");
}
