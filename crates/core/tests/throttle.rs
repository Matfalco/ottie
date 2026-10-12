#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Speed limits and priorities, measured on a local server. Timings use wide
//! margins: they check shares and lower bounds, not exact speeds.

mod support;

use std::time::{Duration, Instant};

use ottie_core::{HttpBackend, HttpConfig, JobId, JobSnapshot, JobState, Manager, Priority, Store};
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
        self.manager.add(url, self.dir.path()).await.unwrap()
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

    /// Lets two running jobs share the limit for a while, then returns how
    /// many bytes each got during that time.
    async fn race(&self, a: JobId, b: JobId) -> (f64, f64) {
        tokio::time::sleep(Duration::from_millis(300)).await;
        let (a0, b0) = (self.job(a).await.downloaded, self.job(b).await.downloaded);
        tokio::time::sleep(Duration::from_millis(1200)).await;
        let (a1, b1) = (self.job(a).await.downloaded, self.job(b).await.downloaded);
        ((a1 - a0) as f64, (b1 - b0) as f64)
    }
}

async fn server(size: u64) -> TestServer {
    TestServer::start(test_data(size as usize, 3)).await
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
async fn the_global_limit_is_shared_per_download_not_per_connection() {
    let s = setup().await;
    s.manager.set_global_limit(256 * KIB).await.unwrap();
    // Four connections versus one (no ETag means no splitting).
    let split = server(4 * 1024 * KIB).await;
    let single = server(4 * 1024 * KIB).await;
    single.configure(|b| b.etag = None);
    let a = s.add(&split, "a.bin").await;
    let b = s.add(&single, "b.bin").await;

    let (got_a, got_b) = s.race(a, b).await;
    let ratio = got_a / got_b;
    assert!((0.6..1.6).contains(&ratio), "a {got_a}, b {got_b}");
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
    s.manager.set_global_limit(300 * KIB).await.unwrap();
    let server = server(4 * 1024 * KIB).await;
    let high = s.add(&server, "high.bin").await;
    let normal = s.add(&server, "normal.bin").await;
    s.manager.set_priority(high, Priority::High).await.unwrap();

    let (got_high, got_normal) = s.race(high, normal).await;
    let ratio = got_high / got_normal;
    assert!(
        (1.4..2.8).contains(&ratio),
        "high {got_high}, normal {got_normal}"
    );
}

#[tokio::test]
async fn urgent_jobs_are_served_first() {
    let s = setup().await;
    s.manager.set_global_limit(256 * KIB).await.unwrap();
    let server = server(4 * 1024 * KIB).await;
    let urgent = s.add(&server, "urgent.bin").await;
    let normal = s.add(&server, "normal.bin").await;
    s.manager
        .set_priority(urgent, Priority::Urgent)
        .await
        .unwrap();

    let (got_urgent, got_normal) = s.race(urgent, normal).await;
    assert!(
        got_normal < got_urgent / 5.0,
        "urgent {got_urgent}, normal {got_normal}"
    );
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
