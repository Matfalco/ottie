#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::time::Duration;

use dm_core::{Download, Error, Event, Outcome, RestartReason, build_client, run};
use reqwest::Client;
use support::{TestServer, test_data};
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

const SIZE: usize = 1024 * 1024;
const PAUSE_AT: u64 = 400 * 1024;

struct Setup {
    server: TestServer,
    dir: TempDir,
    client: Client,
    data: Vec<u8>,
}

async fn setup() -> Setup {
    let data = test_data(SIZE, 1);
    let server = TestServer::start(data.clone()).await;
    // Trickle the body so pausing lands mid-file instead of after it.
    server.configure(|b| b.chunk_delay = Some(Duration::from_millis(1)));
    Setup {
        server,
        dir: tempfile::tempdir().unwrap(),
        client: build_client().unwrap(),
        data,
    }
}

impl Setup {
    fn download(&self, path: &str) -> Download {
        Download::new(self.server.url(path), self.dir.path()).unwrap()
    }
}

/// Runs until done (or until `pause_at` bytes, if given), collecting events.
async fn run_collect(
    client: &Client,
    dl: &mut Download,
    pause_at: Option<u64>,
) -> (Result<Outcome, Error>, Vec<Event>) {
    let cancel = CancellationToken::new();
    let mut events = Vec::new();
    let result = run(client, dl, &cancel, |event| {
        if let (Some(limit), Event::Progress { downloaded, .. }) = (pause_at, &event)
            && *downloaded >= limit
        {
            cancel.cancel();
        }
        events.push(event);
    })
    .await;
    (result, events)
}

fn restarts(events: &[Event]) -> Vec<RestartReason> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Restarted(reason) => Some(*reason),
            Event::Progress { .. } => None,
        })
        .collect()
}

/// Pauses partway and checks the `.part` file matches what `dl` claims.
async fn pause_midway(s: &Setup, dl: &mut Download) {
    let (result, _) = run_collect(&s.client, dl, Some(PAUSE_AT)).await;
    assert_eq!(result.unwrap(), Outcome::Paused);
    assert!(dl.downloaded >= PAUSE_AT && dl.downloaded < SIZE as u64);
    let part_len = std::fs::metadata(dl.part_path().unwrap()).unwrap().len();
    assert_eq!(part_len, dl.downloaded);
}

fn assert_completed_with(outcome: Outcome, expected: &[u8]) {
    let Outcome::Completed { path } = outcome else {
        panic!("expected completion, got {outcome:?}");
    };
    // Byte-for-byte comparison: stricter than comparing hashes.
    assert!(std::fs::read(&path).unwrap() == expected, "content differs");
    assert!(
        !path
            .with_file_name(format!("{}.part", path.file_name().unwrap().display()))
            .exists()
    );
}

#[tokio::test]
async fn downloads_whole_file() {
    let s = setup().await;
    let mut dl = s.download("/files/data.bin");
    let (result, events) = run_collect(&s.client, &mut dl, None).await;
    let outcome = result.unwrap();
    assert_eq!(
        outcome,
        Outcome::Completed {
            path: s.dir.path().join("data.bin")
        }
    );
    assert_completed_with(outcome, &s.data);
    assert_eq!(dl.total, Some(SIZE as u64));
    assert!(dl.resumable);
    assert_eq!(
        events.last(),
        Some(&Event::Progress {
            downloaded: SIZE as u64,
            total: Some(SIZE as u64)
        })
    );
}

#[tokio::test]
async fn pause_then_resume_is_byte_identical() {
    let s = setup().await;
    let mut dl = s.download("/files/data.bin");
    pause_midway(&s, &mut dl).await;
    let offset = dl.downloaded;

    let (result, events) = run_collect(&s.client, &mut dl, None).await;
    assert_completed_with(result.unwrap(), &s.data);
    assert!(restarts(&events).is_empty());

    let resume = &s.server.requests()[1];
    assert_eq!(
        resume.range.as_deref(),
        Some(format!("bytes={offset}-").as_str())
    );
    assert_eq!(resume.if_range.as_deref(), Some("\"v1\""));
}

#[tokio::test]
async fn resume_restarts_when_remote_file_changed() {
    let s = setup().await;
    let mut dl = s.download("/files/data.bin");
    pause_midway(&s, &mut dl).await;

    let new_data = test_data(SIZE + 1000, 2);
    s.server.configure(|b| {
        b.body = new_data.clone().into();
        b.etag = Some("\"v2\"".to_owned());
    });
    let (result, events) = run_collect(&s.client, &mut dl, None).await;
    assert_completed_with(result.unwrap(), &new_data);
    assert_eq!(restarts(&events), [RestartReason::RemoteChanged]);
}

#[tokio::test]
async fn resume_restarts_when_server_ignores_ranges() {
    let s = setup().await;
    let mut dl = s.download("/files/data.bin");
    pause_midway(&s, &mut dl).await;

    s.server.configure(|b| b.ranges = false);
    let (result, events) = run_collect(&s.client, &mut dl, None).await;
    assert_completed_with(result.unwrap(), &s.data);
    assert_eq!(restarts(&events), [RestartReason::RangesIgnored]);
    assert!(!dl.resumable, "server now says Accept-Ranges: none");
}

#[tokio::test]
async fn without_validator_resume_starts_over() {
    let s = setup().await;
    s.server.configure(|b| b.etag = None);
    let mut dl = s.download("/files/data.bin");
    pause_midway(&s, &mut dl).await;
    assert!(!dl.resumable);

    let (result, events) = run_collect(&s.client, &mut dl, None).await;
    assert_completed_with(result.unwrap(), &s.data);
    assert_eq!(restarts(&events), [RestartReason::NotResumable]);
    assert_eq!(s.server.requests()[1].range, None);
}

#[tokio::test]
async fn dropped_connection_can_be_resumed() {
    let s = setup().await;
    s.server.configure(|b| b.drop_after = Some(300_000));
    let mut dl = s.download("/files/data.bin");

    let (result, _) = run_collect(&s.client, &mut dl, None).await;
    assert!(matches!(result, Err(Error::Http(_))), "got {result:?}");
    // Bytes in flight when the connection died are lost, so expect up to 300 000.
    let offset = dl.downloaded;
    assert!(offset > 0 && offset <= 300_000, "downloaded {offset}");
    let part_len = std::fs::metadata(dl.part_path().unwrap()).unwrap().len();
    assert_eq!(part_len, offset);

    let (result, events) = run_collect(&s.client, &mut dl, None).await;
    assert_completed_with(result.unwrap(), &s.data);
    assert!(restarts(&events).is_empty());
    let resume = &s.server.requests()[1];
    assert_eq!(
        resume.range.as_deref(),
        Some(format!("bytes={offset}-").as_str())
    );
}

#[tokio::test]
async fn deleted_part_file_restarts() {
    let s = setup().await;
    let mut dl = s.download("/files/data.bin");
    pause_midway(&s, &mut dl).await;
    std::fs::remove_file(dl.part_path().unwrap()).unwrap();

    let (result, events) = run_collect(&s.client, &mut dl, None).await;
    assert_completed_with(result.unwrap(), &s.data);
    assert_eq!(restarts(&events), [RestartReason::PartFileMissing]);
}

#[tokio::test]
async fn pause_is_prompt_on_a_slow_server() {
    let s = setup().await;
    s.server
        .configure(|b| b.chunk_delay = Some(Duration::from_secs(30)));
    let mut dl = s.download("/files/data.bin");
    let cancel = CancellationToken::new();
    let pauser = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        pauser.cancel();
    });

    let result = tokio::time::timeout(
        Duration::from_secs(5),
        run(&s.client, &mut dl, &cancel, |_| {}),
    )
    .await
    .expect("pause took too long");
    assert_eq!(result.unwrap(), Outcome::Paused);
}

#[tokio::test]
async fn existing_file_gets_a_numbered_name() {
    let s = setup().await;
    std::fs::write(s.dir.path().join("data.bin"), b"mine").unwrap();
    let mut dl = s.download("/files/data.bin");

    let (result, _) = run_collect(&s.client, &mut dl, None).await;
    let outcome = result.unwrap();
    assert_eq!(
        outcome,
        Outcome::Completed {
            path: s.dir.path().join("data (1).bin")
        }
    );
    assert_completed_with(outcome, &s.data);
    assert_eq!(
        std::fs::read(s.dir.path().join("data.bin")).unwrap(),
        b"mine"
    );
}

#[tokio::test]
async fn redirect_to_another_scheme_is_rejected() {
    let s = setup().await;
    let mut dl = s.download("/redirect-to-ftp");
    let (result, _) = run_collect(&s.client, &mut dl, None).await;
    assert!(
        matches!(&result, Err(Error::Http(e)) if e.is_redirect()),
        "got {result:?}"
    );
    assert_eq!(std::fs::read_dir(s.dir.path()).unwrap().count(), 0);
}

#[test]
fn only_http_urls_are_accepted() {
    let url = reqwest::Url::parse("file:///etc/passwd").unwrap();
    assert!(matches!(
        Download::new(url, "."),
        Err(Error::UnsupportedScheme(s)) if s == "file"
    ));
}
