#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Retrying failed downloads: which errors, how often, how long to wait, and
//! when a retrying job gives its slot to a waiting one.

mod support;

use std::time::{Duration, Instant};

use axum::http::StatusCode;
use ottie_core::{
    HttpBackend, HttpConfig, JobEvent, JobId, JobSnapshot, JobState, Manager, NewJob, Problem,
    RetryPolicy, Store,
};
use support::{TestServer, test_data};
use tempfile::TempDir;
use tokio::sync::broadcast::Receiver;

const SIZE: usize = 256 * 1024;
const TIMEOUT: Duration = Duration::from_secs(10);
const ALWAYS: usize = usize::MAX;

struct Setup {
    manager: Manager,
    dir: TempDir,
    events: Receiver<JobEvent>,
}

/// Retries wait 20 ms (then 40, 80…), so tests stay fast; a server silent
/// for 300 ms counts as stalled.
fn policy(max_retries: u32, retries_before_yield: u32) -> RetryPolicy {
    RetryPolicy {
        max_retries,
        retries_before_yield,
        first_wait: Duration::from_millis(20),
    }
}

async fn setup(policy: RetryPolicy, max_concurrent: usize) -> Setup {
    let config = HttpConfig {
        stall_timeout: Duration::from_millis(300),
        ..HttpConfig::default()
    };
    let store = Store::open_in_memory().await.unwrap();
    let manager = Manager::start(HttpBackend::with_config(config).unwrap(), store)
        .await
        .unwrap();
    manager.set_retry_policy(policy).await.unwrap();
    manager.set_max_concurrent(max_concurrent).await.unwrap();
    // Subscribe before adding anything so no event is missed.
    let events = manager.subscribe();
    Setup {
        manager,
        dir: tempfile::tempdir().unwrap(),
        events,
    }
}

async fn server() -> TestServer {
    TestServer::start(test_data(SIZE, 5)).await
}

impl Setup {
    async fn add(&self, server: &TestServer, name: &str) -> JobId {
        let url = server.url(&format!("/files/{name}"));
        self.manager
            .add(NewJob::new(url).in_dir(self.dir.path()))
            .await
            .unwrap()
    }

    /// Every state change, in order, until each of `ids` has completed or failed.
    async fn run_to_end(&mut self, ids: &[JobId]) -> Vec<JobSnapshot> {
        let mut log: Vec<JobSnapshot> = Vec::new();
        let finished = |log: &[JobSnapshot], id: JobId| {
            log.iter()
                .any(|j| j.id == id && matches!(j.state, JobState::Completed | JobState::Failed))
        };
        tokio::time::timeout(TIMEOUT, async {
            while !ids.iter().all(|&id| finished(&log, id)) {
                if let JobEvent::Changed(job) = self.events.recv().await.unwrap() {
                    // Only state changes (not new file names) go in the log.
                    let last = log.iter().rev().find(|j| j.id == job.id);
                    if last.is_none_or(|last| last.state != job.state) {
                        log.push(job);
                    }
                }
            }
        })
        .await
        .expect("jobs didn't finish");
        log
    }

    fn assert_file(&self, name: &str) {
        let data = std::fs::read(self.dir.path().join(name)).unwrap();
        assert!(data == test_data(SIZE, 5), "content differs");
    }
}

fn states(log: &[JobSnapshot], id: JobId) -> Vec<JobState> {
    log.iter().filter(|j| j.id == id).map(|j| j.state).collect()
}

fn count(log: &[JobSnapshot], id: JobId, state: JobState) -> usize {
    states(log, id).iter().filter(|&&s| s == state).count()
}

#[tokio::test]
async fn server_errors_are_retried_until_they_clear() {
    let mut s = setup(policy(10, 3), 3).await;
    let server = server().await;
    server.configure(|b| b.fail_next = Some((StatusCode::SERVICE_UNAVAILABLE, 3)));
    let id = s.add(&server, "a.bin").await;
    let log = s.run_to_end(&[id]).await;

    use JobState::*;
    let expected = [
        Queued,
        Connecting,
        Retrying,
        Connecting,
        Retrying,
        Connecting,
        Retrying,
        Connecting,
        Downloading,
        Completed,
    ];
    assert_eq!(states(&log, id), expected);
    // The snapshot says which retry it's waiting for, when, and why.
    let third = log.iter().filter(|j| j.state == Retrying).nth(2).unwrap();
    assert_eq!(third.retries, 3);
    assert!(third.retry_at.is_some());
    assert_eq!(third.problem, Some(Problem::HttpStatus { status: 503 }));
    s.assert_file("a.bin");
}

#[tokio::test]
async fn retries_run_out_and_the_job_fails() {
    let mut s = setup(policy(2, 3), 3).await;
    let server = server().await;
    server.configure(|b| b.fail_next = Some((StatusCode::INTERNAL_SERVER_ERROR, ALWAYS)));
    let id = s.add(&server, "a.bin").await;
    let log = s.run_to_end(&[id]).await;

    assert_eq!(count(&log, id, JobState::Retrying), 2);
    let failed = log.last().unwrap();
    assert_eq!(failed.state, JobState::Failed);
    assert_eq!(failed.problem, Some(Problem::HttpStatus { status: 500 }));
}

#[tokio::test]
async fn a_404_is_not_retried() {
    let mut s = setup(policy(10, 3), 3).await;
    let server = server().await;
    server.configure(|b| b.fail_next = Some((StatusCode::NOT_FOUND, ALWAYS)));
    let id = s.add(&server, "a.bin").await;
    let log = s.run_to_end(&[id]).await;

    assert_eq!(count(&log, id, JobState::Retrying), 0);
    assert_eq!(log.last().unwrap().state, JobState::Failed);
}

#[tokio::test]
async fn a_dropped_connection_resumes_on_retry() {
    let mut s = setup(policy(10, 3), 3).await;
    let server = server().await;
    server.configure(|b| {
        // Slow enough that the headers and some data arrive before the drop.
        b.chunk_delay = Some(Duration::from_millis(1));
        b.drop_after = Some(100_000);
    });
    let id = s.add(&server, "a.bin").await;
    let log = s.run_to_end(&[id]).await;

    assert_eq!(count(&log, id, JobState::Retrying), 1);
    assert_eq!(log.last().unwrap().state, JobState::Completed);
    // The retry continued where the first attempt stopped.
    let retried = &server.requests()[1];
    let range = retried.range.as_deref().unwrap();
    assert!(
        range != "bytes=0-" && range.starts_with("bytes="),
        "{range}"
    );
    s.assert_file("a.bin");
}

#[tokio::test]
async fn a_stalled_connection_is_retried() {
    let mut s = setup(policy(10, 3), 3).await;
    let server = server().await;
    server.configure(|b| b.stall_after = Some(100_000));
    let id = s.add(&server, "a.bin").await;
    let log = s.run_to_end(&[id]).await;

    let retrying = log.iter().find(|j| j.state == JobState::Retrying).unwrap();
    assert_eq!(
        retrying.problem,
        Some(Problem::Stalled { seconds: 1 }),
        "the 300 ms stall timeout, rounded up"
    );
    assert_eq!(log.last().unwrap().state, JobState::Completed);
    s.assert_file("a.bin");
}

#[tokio::test]
async fn retry_after_sets_the_wait() {
    let mut s = setup(policy(10, 3), 3).await;
    let server = server().await;
    server.configure(|b| {
        b.fail_next = Some((StatusCode::TOO_MANY_REQUESTS, 1));
        b.retry_after = Some(1);
    });
    let id = s.add(&server, "a.bin").await;

    let mut retrying_since = None;
    let mut waited = None;
    tokio::time::timeout(TIMEOUT, async {
        while waited.is_none() {
            let JobEvent::Changed(job) = s.events.recv().await.unwrap() else {
                continue;
            };
            match job.state {
                JobState::Retrying => retrying_since = Some(Instant::now()),
                JobState::Connecting => waited = retrying_since.map(|at| at.elapsed()),
                _ => {}
            }
        }
    })
    .await
    .expect("never retried");
    let waited = waited.unwrap();
    // The policy says 20 ms; the server asked for 1 s.
    assert!(
        waited >= Duration::from_millis(900),
        "waited only {waited:?}"
    );
    s.manager.cancel(id).await.unwrap();
}

#[tokio::test]
async fn a_failing_job_gives_way_after_its_in_slot_retries() {
    let mut s = setup(policy(5, 1), 1).await;
    let bad = server().await;
    bad.configure(|b| b.fail_next = Some((StatusCode::BAD_GATEWAY, ALWAYS)));
    let good = server().await;
    let failing = s.add(&bad, "bad.bin").await; // gets the only slot
    let waiting = s.add(&good, "good.bin").await;
    let log = s.run_to_end(&[failing, waiting]).await;

    use JobState::*;
    let at = |id, state| log.iter().position(|j| j.id == id && j.state == state);
    let good_starts = at(waiting, Connecting).unwrap();
    // One retry in its slot, then the second failure gives the slot away.
    let before: Vec<_> = log[..good_starts]
        .iter()
        .filter(|j| j.id == failing)
        .map(|j| j.state)
        .collect();
    assert_eq!(
        before,
        [Queued, Connecting, Retrying, Connecting, Retrying, Queued]
    );
    // The good one finishes; the bad one then uses up its retries and fails.
    assert!(at(waiting, Completed).unwrap() < at(failing, Failed).unwrap());
    assert_eq!(count(&log, failing, Retrying), 5);
    s.assert_file("good.bin");
}

#[tokio::test]
async fn without_waiting_jobs_a_failing_job_keeps_its_slot() {
    let mut s = setup(policy(3, 0), 1).await;
    let server = server().await;
    server.configure(|b| b.fail_next = Some((StatusCode::SERVICE_UNAVAILABLE, ALWAYS)));
    let id = s.add(&server, "a.bin").await;
    let log = s.run_to_end(&[id]).await;

    use JobState::*;
    let expected = [
        Queued, Connecting, Retrying, Connecting, Retrying, Connecting, Retrying, Connecting,
        Failed,
    ];
    assert_eq!(states(&log, id), expected);
}

#[tokio::test]
async fn pausing_a_retrying_job_stops_its_retries() {
    let slow = RetryPolicy {
        first_wait: Duration::from_millis(300),
        ..policy(10, 3)
    };
    let mut s = setup(slow, 3).await;
    let server = server().await;
    server.configure(|b| b.fail_next = Some((StatusCode::SERVICE_UNAVAILABLE, ALWAYS)));
    let id = s.add(&server, "a.bin").await;
    tokio::time::timeout(TIMEOUT, async {
        loop {
            if let JobEvent::Changed(job) = s.events.recv().await.unwrap()
                && job.state == JobState::Retrying
            {
                return;
            }
        }
    })
    .await
    .expect("never retrying");

    s.manager.pause(id).await.unwrap();
    let requests = server.requests().len();
    tokio::time::sleep(Duration::from_millis(600)).await;
    let job = s.manager.list().await.unwrap().remove(0);
    assert_eq!(job.state, JobState::Paused);
    assert_eq!(job.retry_at, None);
    assert_eq!(
        server.requests().len(),
        requests,
        "no retry after the pause"
    );
}
