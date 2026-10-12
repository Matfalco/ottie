#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::path::PathBuf;
use std::time::Duration;

use ottie_core::{
    Error, HttpBackend, JobId, JobSnapshot, JobState, Manager, ManagerConfig, PART_SUFFIX, Store,
};
use support::{TestServer, test_data};
use tempfile::TempDir;

const SIZE: usize = 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(10);

struct Setup {
    server: TestServer,
    _dir: TempDir,
    db: PathBuf,
    downloads: PathBuf,
    data: Vec<u8>,
}

async fn setup() -> Setup {
    let data = test_data(SIZE, 1);
    let server = TestServer::start(data.clone()).await;
    // Slow enough (about 0.3 s per file) to stop a download halfway.
    server.configure(|b| b.chunk_delay = Some(Duration::from_millis(5)));
    let dir = tempfile::tempdir().unwrap();
    let downloads = dir.path().join("downloads");
    std::fs::create_dir(&downloads).unwrap();
    Setup {
        server,
        db: dir.path().join("ottie.db"),
        _dir: dir,
        downloads,
        data,
    }
}

impl Setup {
    /// Starts a manager on the test database, as Ottie does at launch.
    async fn launch(&self) -> Manager {
        let store = Store::open(&self.db).await.unwrap();
        Manager::start(HttpBackend::new().unwrap(), store, ManagerConfig::default())
            .await
            .unwrap()
    }

    async fn add(&self, manager: &Manager) -> JobId {
        let url = self.server.url("/files/data.bin");
        manager.add(url, &self.downloads).await.unwrap()
    }

    fn part(&self) -> PathBuf {
        self.downloads.join(format!("data.bin{PART_SUFFIX}"))
    }

    fn part_len(&self) -> u64 {
        std::fs::metadata(self.part()).unwrap().len()
    }

    fn assert_complete(&self) {
        let file = self.downloads.join("data.bin");
        assert!(std::fs::read(file).unwrap() == self.data, "content differs");
        assert!(!self.part().exists());
    }
}

/// Polls until the job reaches `state`. (Polling rather than events, because a
/// freshly started manager may change state before we can subscribe.)
async fn wait_until(manager: &Manager, id: JobId, state: JobState) -> JobSnapshot {
    tokio::time::timeout(TIMEOUT, async {
        loop {
            let jobs = manager.list().await.unwrap();
            if let Some(job) = jobs.into_iter().find(|j| j.id == id && j.state == state) {
                return job;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("job {id} never became {state:?}"))
}

#[tokio::test]
async fn running_jobs_continue_after_a_restart() {
    let s = setup().await;
    let manager = s.launch().await;
    let id = s.add(&manager).await;
    wait_until(&manager, id, JobState::Downloading).await;
    manager.shutdown().await.unwrap();

    let saved = s.part_len();
    assert!(saved > 0 && saved < SIZE as u64, "stopped at {saved}");

    let manager = s.launch().await;
    wait_until(&manager, id, JobState::Completed).await;
    s.assert_complete();
    let resumed = &s.server.requests()[1];
    assert_eq!(resumed.range, Some(format!("bytes={saved}-")));
    assert_eq!(resumed.if_range.as_deref(), Some("\"v1\""));
}

#[tokio::test]
async fn paused_jobs_stay_paused_after_a_restart() {
    let s = setup().await;
    let manager = s.launch().await;
    let id = s.add(&manager).await;
    wait_until(&manager, id, JobState::Downloading).await;
    manager.pause(id).await.unwrap();
    manager.shutdown().await.unwrap();

    let manager = s.launch().await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let job = wait_until(&manager, id, JobState::Paused).await;
    assert_eq!(job.downloaded, s.part_len());
    assert_eq!(s.server.requests().len(), 1, "a paused job must not start");

    manager.resume(id).await.unwrap();
    wait_until(&manager, id, JobState::Completed).await;
    s.assert_complete();
}

#[tokio::test]
async fn crash_with_stale_progress_resumes_from_the_file() {
    let s = setup().await;
    let manager = s.launch().await;
    let id = s.add(&manager).await;
    wait_until(&manager, id, JobState::Downloading).await;
    manager.pause(id).await.unwrap();
    manager.shutdown().await.unwrap();
    let on_disk = s.part_len();

    // Make the database look like Ottie crashed mid-download, before its
    // last progress save: still "downloading", with an outdated byte count.
    let db = rusqlite::Connection::open(&s.db).unwrap();
    db.execute("UPDATE jobs SET state = 'downloading', downloaded = 0", [])
        .unwrap();
    drop(db);

    let manager = s.launch().await;
    wait_until(&manager, id, JobState::Completed).await;
    s.assert_complete();
    let resumed = &s.server.requests()[1];
    assert_eq!(resumed.range, Some(format!("bytes={on_disk}-")));
}

#[tokio::test]
async fn history_and_ids_survive_a_restart() {
    let s = setup().await;
    s.server.configure(|b| b.chunk_delay = None);
    let manager = s.launch().await;
    let first = s.add(&manager).await;
    let done = wait_until(&manager, first, JobState::Completed).await;
    manager.shutdown().await.unwrap();

    let manager = s.launch().await;
    let jobs = manager.list().await.unwrap();
    assert_eq!(jobs, [done]);

    let url = s.server.url("/files/other.bin");
    let second = manager.add(url, &s.downloads).await.unwrap();
    assert!(second > first);
    wait_until(&manager, second, JobState::Completed).await;
}

#[tokio::test]
async fn a_database_from_a_newer_ottie_is_refused() {
    let s = setup().await;
    let db = rusqlite::Connection::open(&s.db).unwrap();
    db.pragma_update(None, "user_version", 99).unwrap();
    drop(db);

    let result = Store::open(&s.db).await;
    assert!(
        matches!(result, Err(Error::DatabaseTooNew { found: 99, .. })),
        "got {:?}",
        result.err()
    );
}
