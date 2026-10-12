#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Speed limits and priorities, measured on a local server. Timings use wide
//! margins: they check shares and lower bounds, not exact speeds. Shares are
//! measured by when jobs finish, which is reported at once, rather than by
//! progress over a short window, which lags behind.

mod support;

use std::time::{Duration, Instant};

use ottie_core::{
    CategorySettings, HttpBackend, HttpConfig, JobId, JobSnapshot, JobState, Manager, NewJob,
    Priority, Store,
};
use support::{TestServer, test_data};
use tempfile::TempDir;

const KIB: u64 = 1024;
const TIMEOUT: Duration = Duration::from_secs(15);

struct Setup {
    manager: Manager,
    dir: TempDir,
}

/// Splits files of 256 KiB or more across 4 connections.
async fn setup() -> Setup {
    let config = HttpConfig {
        connections: 4,
        min_split: 128 * KIB,
        ..HttpConfig::default()
    };
    let store = Store::open_in_memory().await.unwrap();
    let manager = Manager::start(HttpBackend::with_config(config).unwrap(), store)
        .await
        .unwrap();
    Setup {
        manager,
        dir: tempfile::tempdir().unwrap(),
    }
}

impl Setup {
    async fn add(&self, server: &TestServer, name: &str) -> JobId {
        let url = server.url(&format!("/files/{name}"));
        self.manager
            .add(NewJob::new(url).in_dir(self.dir.path()))
            .await
            .unwrap()
    }

    async fn job(&self, id: JobId) -> JobSnapshot {
        let jobs = self.manager.list().await.unwrap();
        jobs.into_iter().find(|j| j.id == id).unwrap()
    }

    async fn wait_until(&self, id: JobId, state: JobState) {
        tokio::time::timeout(TIMEOUT, async {
            while self.job(id).await.state != state {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("job {id} never became {state:?}"));
    }

    /// Waits for both jobs to complete and returns how long `b` took
    /// divided by how long `a` took, both counted from `started`.
    async fn finish_ratio(&self, a: JobId, b: JobId, started: Instant) -> f64 {
        let mut done: [Option<f64>; 2] = [None, None];
        tokio::time::timeout(TIMEOUT, async {
            while done.iter().any(Option::is_none) {
                let jobs = self.manager.list().await.unwrap();
                for (slot, id) in done.iter_mut().zip([a, b]) {
                    let finished = jobs
                        .iter()
                        .any(|j| j.id == id && j.state == JobState::Completed);
                    if slot.is_none() && finished {
                        *slot = Some(started.elapsed().as_secs_f64());
                    }
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("jobs didn't finish");
        let [Some(a), Some(b)] = done else {
            unreachable!("the loop only ends when both are set")
        };
        b / a
    }
}

async fn server(size: u64) -> TestServer {
    TestServer::start(test_data(size as usize, 3)).await
}

/// No ETag, so downloads from it use one connection. Priority tests use it:
/// a split download also receives up to a few hundred KiB past its ranges'
/// ends (data buffered while it was throttled), which counts against the
/// limit and would blur the shares on files this small.
async fn single_connection_server(size: u64) -> TestServer {
    let server = server(size).await;
    server.configure(|b| b.etag = None);
    server
}

#[tokio::test]
async fn a_job_limit_slows_that_job() {
    let s = setup().await;
    let server = server(256 * KIB).await;
    // A slightly slow server, so the limit is in place before much arrives.
    server.configure(|b| b.chunk_delay = Some(Duration::from_millis(2)));
    let started = Instant::now();
    let id = s.add(&server, "a.bin").await;
    s.manager.set_speed_limit(id, 128 * KIB).await.unwrap();
    s.wait_until(id, JobState::Completed).await;
    // 256 KiB at 128 KiB/s takes about 2 s.
    let took = started.elapsed();
    assert!(took >= Duration::from_millis(1500), "took only {took:?}");
}

#[tokio::test]
async fn a_category_limit_slows_its_jobs_together() {
    let s = setup().await;
    let videos = CategorySettings {
        extensions: vec!["mkv".into()],
        speed_limit: 128 * KIB,
        ..CategorySettings::named("Videos")
    };
    s.manager.add_category(videos).await.unwrap();
    // Too small to split, so one connection each.
    let server = server(128 * KIB).await;
    let started = Instant::now();
    let a = s.add(&server, "a.mkv").await;
    let b = s.add(&server, "b.mkv").await;
    s.wait_until(a, JobState::Completed).await;
    s.wait_until(b, JobState::Completed).await;
    // 256 KiB at 128 KiB/s together takes about 2 s; with the limit applied
    // to each job alone, 1 s.
    let took = started.elapsed();
    assert!(took >= Duration::from_millis(1500), "took only {took:?}");
}

#[tokio::test]
async fn the_global_limit_is_shared_per_download_not_per_connection() {
    let s = setup().await;
    s.manager.set_global_limit(4 * 1024 * KIB).await.unwrap();
    // Four connections versus one.
    let split = server(4 * 1024 * KIB).await;
    let single = single_connection_server(4 * 1024 * KIB).await;
    let started = Instant::now();
    let a = s.add(&split, "a.bin").await;
    let b = s.add(&single, "b.bin").await;

    // Equal shares: both finish after about 2 s, the split one a little later
    // because of the bytes it discards (about 0.9×). Shared per connection,
    // the split one would get 4/5 and finish at 1.25 s, the other at 2 s (1.6×).
    let ratio = s.finish_ratio(a, b, started).await;
    assert!(
        (0.75..1.3).contains(&ratio),
        "single took {ratio:.2}× as long"
    );
}

#[tokio::test]
async fn downloads_together_stay_under_the_global_limit() {
    let s = setup().await;
    s.manager.set_global_limit(256 * KIB).await.unwrap();
    let split = server(256 * KIB).await;
    let single = server(256 * KIB).await;
    single.configure(|b| b.etag = None);
    let started = Instant::now();
    let a = s.add(&split, "a.bin").await;
    let b = s.add(&single, "b.bin").await;
    s.wait_until(a, JobState::Completed).await;
    s.wait_until(b, JobState::Completed).await;
    // 512 KiB at 256 KiB/s is 2 s; each job may start with 64 KiB of
    // unspent budget, so at least 1.5 s.
    let took = started.elapsed();
    assert!(took >= Duration::from_millis(1400), "took only {took:?}");
}

#[tokio::test]
async fn high_priority_gets_twice_the_share() {
    let s = setup().await;
    s.manager.set_global_limit(384 * KIB).await.unwrap();
    // High gets a file twice as big, so with twice the share both finish
    // together, after about 2 s.
    let big = single_connection_server(512 * KIB).await;
    let small = single_connection_server(256 * KIB).await;
    let started = Instant::now();
    let high = s.add(&big, "high.bin").await;
    s.manager.set_priority(high, Priority::High).await.unwrap();
    let normal = s.add(&small, "normal.bin").await;

    // With equal shares, Normal would finish at 1.33 s and High at 2 s (0.67×).
    let ratio = s.finish_ratio(high, normal, started).await;
    assert!(
        (0.85..1.2).contains(&ratio),
        "normal took {ratio:.2}× as long"
    );
}

#[tokio::test]
async fn urgent_jobs_are_served_first() {
    let s = setup().await;
    s.manager.set_global_limit(256 * KIB).await.unwrap();
    let server = single_connection_server(256 * KIB).await;
    let started = Instant::now();
    let urgent = s.add(&server, "urgent.bin").await;
    s.manager
        .set_priority(urgent, Priority::Urgent)
        .await
        .unwrap();
    let normal = s.add(&server, "normal.bin").await;

    // Urgent takes the whole limit and finishes after about 1 s; Normal only
    // starts then, and finishes at 2 s (2×). Shared equally, both at 2 s (1×).
    let ratio = s.finish_ratio(urgent, normal, started).await;
    assert!(ratio > 1.6, "normal took only {ratio:.2}× as long");
}

#[tokio::test]
async fn limits_change_while_downloading() {
    let s = setup().await;
    s.manager.set_global_limit(32 * KIB).await.unwrap();
    let server = server(2 * 1024 * KIB).await;
    let id = s.add(&server, "a.bin").await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(s.job(id).await.downloaded < 512 * KIB);

    // 2 MiB at 32 KiB/s would take a minute; lifting the limit finishes it.
    s.manager.set_global_limit(0).await.unwrap();
    s.wait_until(id, JobState::Completed).await;
}

#[tokio::test]
async fn the_queue_starts_higher_priorities_first() {
    let s = setup().await;
    s.manager.set_max_concurrent(1).await.unwrap();
    let server = server(64 * KIB).await;
    server.configure(|b| b.chunk_delay = Some(Duration::from_millis(20)));
    let a = s.add(&server, "a.bin").await; // starts at once
    let b = s.add(&server, "b.bin").await;
    let c = s.add(&server, "c.bin").await;
    let d = s.add(&server, "d.bin").await;
    s.manager.set_priority(c, Priority::High).await.unwrap();
    s.manager.set_priority(d, Priority::Urgent).await.unwrap();

    let mut events = s.manager.subscribe();
    let mut finished = Vec::new();
    tokio::time::timeout(TIMEOUT, async {
        while finished.len() < 4 {
            if let ottie_core::JobEvent::Changed(job) = events.recv().await.unwrap()
                && job.state == JobState::Completed
            {
                finished.push(job.id);
            }
        }
    })
    .await
    .expect("not all jobs finished");
    assert_eq!(finished, [a, d, c, b]);
}
