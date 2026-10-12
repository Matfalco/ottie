#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::collections::HashSet;
use std::time::Duration;

use ottie_core::{
    Error, HttpBackend, JobEvent, JobId, JobSnapshot, JobState, Manager, NewJob, Problem, Store,
};
use support::{TestServer, test_data};
use tempfile::TempDir;
use tokio::sync::broadcast::Receiver;

const SIZE: usize = 512 * 1024;
const TIMEOUT: Duration = Duration::from_secs(10);

struct Setup {
    server: TestServer,
    dir: TempDir,
    manager: Manager,
    events: Receiver<JobEvent>,
    data: Vec<u8>,
}

/// `chunk_delay_ms` slows the server down so jobs stay running long enough to poke at.
async fn setup(max_concurrent: usize, chunk_delay_ms: u64) -> Setup {
    let data = test_data(SIZE, 1);
    let server = TestServer::start(data.clone()).await;
    server.configure(|b| b.chunk_delay = Some(Duration::from_millis(chunk_delay_ms)));
    let store = Store::open_in_memory().await.unwrap();
    let manager = Manager::start(HttpBackend::new().unwrap(), store)
        .await
        .unwrap();
    manager.set_max_concurrent(max_concurrent).await.unwrap();
    // Subscribe before adding anything so no event is missed.
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
    async fn add(&self, name: &str) -> JobId {
        let url = self.server.url(&format!("/files/{name}"));
        self.manager
            .add(NewJob::new(url).in_dir(self.dir.path()))
            .await
            .unwrap()
    }

    async fn job(&self, id: JobId) -> JobSnapshot {
        let jobs = self.manager.list().await.unwrap();
        jobs.into_iter().find(|j| j.id == id).unwrap()
    }

    /// Waits for a `Changed` event putting `id` in `state`.
    async fn wait_for(&mut self, id: JobId, state: JobState) -> JobSnapshot {
        self.wait_for_all(&[id], state).await.remove(0)
    }

    /// Waits until every job in `ids` has reached `state`, in any order.
    async fn wait_for_all(&mut self, ids: &[JobId], state: JobState) -> Vec<JobSnapshot> {
        let events = &mut self.events;
        let mut reached = Vec::new();
        tokio::time::timeout(TIMEOUT, async {
            while reached.len() < ids.len() {
                if let JobEvent::Changed(job) = events.recv().await.unwrap()
                    && ids.contains(&job.id)
                    && job.state == state
                {
                    reached.push(job);
                }
            }
        })
        .await
        .unwrap_or_else(|_| panic!("not all of {ids:?} became {state:?}"));
        reached
    }

    fn assert_file(&self, job: &JobSnapshot) {
        let path = self.dir.path().join(job.file_name.as_ref().unwrap());
        assert!(std::fs::read(path).unwrap() == self.data, "content differs");
    }
}

#[tokio::test]
async fn job_goes_through_every_state() {
    let mut s = setup(3, 0).await;
    let id = s.add("one.bin").await;

    let mut changes = Vec::new();
    while changes.last().map(|j: &JobSnapshot| j.state) != Some(JobState::Completed) {
        if let JobEvent::Changed(job) = s.events.recv().await.unwrap() {
            changes.push(job);
        }
    }
    let mut states: Vec<JobState> = changes.iter().map(|j| j.state).collect();
    states.dedup();
    use JobState::*;
    assert_eq!(states, [Queued, Connecting, Downloading, Completed]);
    // The file name is announced while still connecting, before any bytes.
    let named = changes.iter().find(|j| j.file_name.is_some()).unwrap();
    assert_eq!(named.state, Connecting);

    let job = s.job(id).await;
    assert_eq!(job.downloaded, SIZE as u64);
    assert_eq!(job.file_name.as_deref(), Some("one.bin"));
    s.assert_file(&job);
}

#[tokio::test]
async fn queue_respects_max_concurrent() {
    let mut s = setup(1, 1).await;
    let ids = [
        s.add("a.bin").await,
        s.add("b.bin").await,
        s.add("c.bin").await,
    ];

    let mut running = HashSet::new();
    let mut completed = Vec::new();
    while completed.len() < ids.len() {
        let JobEvent::Changed(job) = s.events.recv().await.unwrap() else {
            continue;
        };
        match job.state {
            JobState::Connecting => {
                running.insert(job.id);
                assert_eq!(running.len(), 1, "more than one job running");
            }
            JobState::Completed => {
                running.remove(&job.id);
                completed.push(job.id);
            }
            _ => {}
        }
    }
    assert_eq!(completed, ids, "jobs should finish oldest first");
}

#[tokio::test]
async fn raising_the_limit_starts_queued_jobs() {
    let mut s = setup(1, 5).await;
    let ids = [
        s.add("a.bin").await,
        s.add("b.bin").await,
        s.add("c.bin").await,
    ];
    assert_eq!(s.job(ids[2]).await.state, JobState::Queued);

    s.manager.set_max_concurrent(3).await.unwrap();
    for job in s.manager.list().await.unwrap() {
        assert!(job.state.is_running(), "{} is {:?}", job.id, job.state);
    }
    for job in s.wait_for_all(&ids, JobState::Completed).await {
        s.assert_file(&job);
    }
}

#[tokio::test]
async fn pause_and_resume_a_running_job() {
    let mut s = setup(3, 5).await;
    let id = s.add("data.bin").await;
    let running = s.wait_for(id, JobState::Downloading).await;
    assert_eq!(running.file_name.as_deref(), Some("data.bin"));
    assert_eq!(running.total, Some(SIZE as u64));

    // `pause` only returns once the job has stopped and flushed.
    s.manager.pause(id).await.unwrap();
    let paused = s.job(id).await;
    assert_eq!(paused.state, JobState::Paused);
    assert!(paused.downloaded > 0 && paused.downloaded < SIZE as u64);
    let part = s.dir.path().join("data.bin.part.ottie");
    assert_eq!(std::fs::metadata(&part).unwrap().len(), paused.downloaded);

    s.manager.resume(id).await.unwrap();
    let done = s.wait_for(id, JobState::Completed).await;
    s.assert_file(&done);
    assert!(!part.exists());
    let resumed_from = s.server.requests()[1].range.clone();
    assert_eq!(resumed_from, Some(format!("bytes={}-", paused.downloaded)));
}

#[tokio::test]
async fn pausing_a_queued_job_keeps_it_out_of_the_queue() {
    let mut s = setup(1, 1).await;
    let first = s.add("a.bin").await;
    let second = s.add("b.bin").await;

    s.manager.pause(second).await.unwrap();
    s.wait_for(first, JobState::Completed).await;
    assert_eq!(s.job(second).await.state, JobState::Paused);

    s.manager.resume(second).await.unwrap();
    s.wait_for(second, JobState::Completed).await;
}

#[tokio::test]
async fn cancel_deletes_the_part_file() {
    let mut s = setup(3, 5).await;
    let id = s.add("data.bin").await;
    s.wait_for(id, JobState::Downloading).await;

    s.manager.cancel(id).await.unwrap();
    assert_eq!(s.job(id).await.state, JobState::Cancelled);
    assert_eq!(std::fs::read_dir(s.dir.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn http_errors_mark_the_job_failed() {
    let mut s = setup(3, 0).await;
    let url = s.server.url("/no-such-route");
    let id = s
        .manager
        .add(NewJob::new(url).in_dir(s.dir.path()))
        .await
        .unwrap();

    let failed = s.wait_for(id, JobState::Failed).await;
    assert_eq!(failed.problem, Some(Problem::HttpStatus { status: 404 }));
}

#[tokio::test]
async fn invalid_requests_are_refused() {
    let mut s = setup(3, 0).await;
    let id = s.add("data.bin").await;
    s.wait_for(id, JobState::Completed).await;

    let refused = s.manager.resume(id).await;
    assert!(matches!(
        refused,
        Err(Error::InvalidTransition {
            from: JobState::Completed,
            to: JobState::Queued
        })
    ));
    assert!(matches!(
        s.manager.cancel(id).await,
        Err(Error::InvalidTransition { .. })
    ));

    let unknown: JobId = "2000.01.01-00.00.00-1".parse().unwrap();
    assert!(matches!(
        s.manager.pause(unknown).await,
        Err(Error::JobNotFound(_))
    ));

    let ftp: reqwest::Url = "ftp://example.com/file".parse().unwrap();
    assert!(matches!(
        s.manager.add(NewJob::new(ftp).in_dir(s.dir.path())).await,
        Err(Error::UnsupportedScheme(_))
    ));
}

#[tokio::test]
async fn ids_are_unique_and_in_order() {
    let s = setup(1, 50).await;
    let mut ids = Vec::new();
    for i in 0..20 {
        ids.push(s.add(&format!("{i}.bin")).await);
    }
    let mut sorted = ids.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted, ids);
    let listed: Vec<JobId> = s
        .manager
        .list()
        .await
        .unwrap()
        .iter()
        .map(|j| j.id)
        .collect();
    assert_eq!(listed, ids);
}
