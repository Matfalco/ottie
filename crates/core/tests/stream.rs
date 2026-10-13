#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::time::Duration;

use ottie_core::{
    Error, HttpBackend, HttpConfig, JobEvent, JobId, JobState, Manager, NewJob, Reader, Store,
};
use support::{TestServer, test_data};
use tempfile::TempDir;
use tokio::sync::broadcast::Receiver;

const TIMEOUT: Duration = Duration::from_secs(20);
const READ: usize = 64 * 1024;

struct Setup {
    server: TestServer,
    dir: TempDir,
    manager: Manager,
    events: Receiver<JobEvent>,
    data: Vec<u8>,
}

/// `chunk_delay_ms` before each 16 KiB chunk keeps downloads running while
/// they're read.
async fn setup(size: usize, connections: usize, chunk_delay_ms: u64) -> Setup {
    let data = test_data(size, 3);
    let server = TestServer::start(data.clone()).await;
    server.configure(|b| b.chunk_delay = Some(Duration::from_millis(chunk_delay_ms)));
    let config = HttpConfig {
        connections,
        min_split: 64 * 1024,
        ..HttpConfig::default()
    };
    let store = Store::open_in_memory().await.unwrap();
    let manager = Manager::start(HttpBackend::with_config(config).unwrap(), store)
        .await
        .unwrap();
    let events = manager.subscribe();
    Setup {
        server,
        dir: tempfile::tempdir().unwrap(),
        manager,
        events,
        data,
    }
}

impl Setup {
    async fn add(&self) -> JobId {
        let url = self.server.url("/files/movie.mp4");
        self.manager
            .add(NewJob::new(url).in_dir(self.dir.path()))
            .await
            .unwrap()
    }

    async fn wait_for(&mut self, id: JobId, state: JobState) {
        tokio::time::timeout(TIMEOUT, async {
            loop {
                if let JobEvent::Changed(job) = self.events.recv().await.unwrap()
                    && job.id == id
                    && job.state == state
                {
                    return;
                }
            }
        })
        .await
        .unwrap_or_else(|_| panic!("{id} never became {state:?}"));
    }
}

/// Reads from `offset` to the end of the file.
async fn read_rest(reader: &mut Reader, mut offset: u64) -> Vec<u8> {
    let mut all = Vec::new();
    tokio::time::timeout(TIMEOUT, async {
        loop {
            let bytes = reader.read(offset, READ).await.unwrap();
            if bytes.is_empty() {
                return;
            }
            offset += bytes.len() as u64;
            all.extend(bytes);
        }
    })
    .await
    .expect("reading took too long");
    all
}

#[tokio::test]
async fn reads_a_single_connection_download_as_it_arrives() {
    let s = setup(512 * 1024, 1, 10).await;
    let id = s.add().await;
    let (mut reader, opened) = Reader::open(&s.manager, id).await.unwrap();
    assert_eq!(opened.file_name, "movie.mp4");
    assert_eq!(opened.total, Some(512 * 1024));

    let first = reader.read(0, READ).await.unwrap();
    assert!(!first.is_empty());
    let job = s.manager.job(id).await.unwrap();
    assert_ne!(job.state, JobState::Completed, "read while downloading");

    // The reader keeps its file open while the download completes, which
    // renames it (Windows must allow that).
    let rest = read_rest(&mut reader, first.len() as u64).await;
    assert!([first, rest].concat() == s.data, "content differs");
    let job = s.manager.job(id).await.unwrap();
    assert_eq!(job.state, JobState::Completed);
}

#[tokio::test]
async fn reads_a_multi_connection_download_as_it_arrives() {
    let size = 2 * 1024 * 1024;
    let s = setup(size, 4, 100).await;
    let id = s.add().await;
    let (mut reader, _) = Reader::open(&s.manager, id).await.unwrap();

    // Each connection writes its own range: the start of the third one comes
    // long before the bytes before it.
    let middle = size as u64 / 2 + 123;
    let first = reader.read(middle, READ).await.unwrap();
    let job = s.manager.job(id).await.unwrap();
    assert_ne!(job.state, JobState::Completed, "read while downloading");
    let rest = read_rest(&mut reader, middle + first.len() as u64).await;
    assert!(
        [first, rest].concat() == s.data[size / 2 + 123..],
        "content differs"
    );

    let whole = read_rest(&mut reader, 0).await;
    assert!(whole == s.data, "content differs");
}

#[tokio::test]
async fn reads_a_finished_download() {
    let mut s = setup(256 * 1024, 1, 0).await;
    let id = s.add().await;
    s.wait_for(id, JobState::Completed).await;
    let (mut reader, opened) = Reader::open(&s.manager, id).await.unwrap();
    assert_eq!(opened.total, Some(256 * 1024));
    assert!(read_rest(&mut reader, 0).await == s.data, "content differs");
    assert!(reader.read(256 * 1024, READ).await.unwrap().is_empty());
}

#[tokio::test]
async fn a_paused_download_keeps_the_reader_waiting() {
    let s = setup(512 * 1024, 1, 10).await;
    let id = s.add().await;
    let (mut reader, _) = Reader::open(&s.manager, id).await.unwrap();
    reader.read(0, READ).await.unwrap();
    s.manager.pause(id).await.unwrap();

    let reading = tokio::spawn(async move {
        let rest = read_rest(&mut reader, 0).await;
        (reader, rest)
    });
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert!(!reading.is_finished(), "waits for the bytes to come");
    s.manager.resume(id).await.unwrap();
    let (_, all) = reading.await.unwrap();
    assert!(all == s.data, "content differs");
}

#[tokio::test]
async fn a_cancelled_download_stops_the_reader() {
    let s = setup(512 * 1024, 1, 20).await;
    let id = s.add().await;
    let (mut reader, _) = Reader::open(&s.manager, id).await.unwrap();
    let first = reader.read(0, READ).await.unwrap();
    s.manager.cancel(id).await.unwrap();
    let err = reader.read(first.len() as u64, READ).await.unwrap_err();
    assert!(matches!(err, Error::DownloadStopped(stopped) if stopped == id));
}

#[tokio::test]
async fn a_removed_download_stops_the_reader() {
    let s = setup(512 * 1024, 1, 20).await;
    let id = s.add().await;
    let (mut reader, _) = Reader::open(&s.manager, id).await.unwrap();
    let first = reader.read(0, READ).await.unwrap();
    let waiting = tokio::spawn(async move { reader.read(512 * 1024 - 1, READ).await });
    s.manager.remove(id).await.unwrap();
    let err = waiting.await.unwrap().unwrap_err();
    assert!(matches!(err, Error::JobNotFound(gone) if gone == id));
    assert!(!first.is_empty());
}

impl Setup {
    /// Whether a request for a range starting at `start` reached the server.
    fn requested_from(&self, start: u64) -> bool {
        let wanted = format!("bytes={start}-");
        self.server
            .requests()
            .iter()
            .any(|r| r.range.as_deref().is_some_and(|r| r.starts_with(&wanted)))
    }
}

#[tokio::test]
async fn a_player_far_ahead_gets_a_connection_of_its_own() {
    // Four ranges of 1 MiB, each arriving at 80 KiB/s.
    let size = 4 * 1024 * 1024;
    let s = setup(size, 4, 200).await;
    let id = s.add().await;
    let (mut reader, _) = Reader::open(&s.manager, id).await.unwrap();

    // The second connection would need ~12 s to get here by itself.
    let seek = 1024 * 1024 + 960 * 1024;
    let started = std::time::Instant::now();
    let bytes = reader.read(seek, READ).await.unwrap();
    let waited = started.elapsed();
    assert!(waited < Duration::from_secs(5), "waited {waited:?}");
    assert!(s.requested_from(seek), "a connection went there");
    let seek = usize::try_from(seek).unwrap();
    assert!(bytes == s.data[seek..seek + bytes.len()], "content differs");
}

#[tokio::test]
async fn the_end_of_the_file_comes_early_while_someone_reads() {
    // Two ranges of 2 MiB; the player reads from the start.
    let size = 4 * 1024 * 1024;
    let s = setup(size, 2, 200).await;
    let id = s.add().await;
    let (mut reader, _) = Reader::open(&s.manager, id).await.unwrap();
    reader.read(0, READ).await.unwrap();

    let tail = size as u64 - 1024 * 1024;
    tokio::time::timeout(Duration::from_secs(5), async {
        while !s.requested_from(tail) {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("the last MiB was never asked for");
}

#[tokio::test]
async fn following_players_keeps_the_file_intact() {
    let size = 2 * 1024 * 1024;
    let mut s = setup(size, 4, 10).await;
    let id = s.add().await;
    let (mut ahead, _) = Reader::open(&s.manager, id).await.unwrap();
    let (mut behind, _) = Reader::open(&s.manager, id).await.unwrap();

    // Two players far from every connection, each reading on to the end.
    let reading_ahead = read_rest(&mut ahead, 300 * 1024 + 7);
    let reading_behind = read_rest(&mut behind, 1300 * 1024 + 7);
    let (from_ahead, from_behind) = tokio::join!(reading_ahead, reading_behind);
    assert!(from_ahead == s.data[300 * 1024 + 7..], "content differs");
    assert!(from_behind == s.data[1300 * 1024 + 7..], "content differs");

    s.wait_for(id, JobState::Completed).await;
    let file = std::fs::read(s.dir.path().join("movie.mp4")).unwrap();
    assert!(file == s.data, "the finished file differs");
}
