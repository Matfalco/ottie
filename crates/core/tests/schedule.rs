#![allow(clippy::unwrap_used, clippy::expect_used)]

//! A download's own start and stop time (docs/decisions/0040).

mod support;

use std::time::{Duration, Instant, SystemTime};

use ottie_core::{
    Error, HttpBackend, JobChange, JobEvent, JobId, JobSnapshot, JobState, Manager, NewJob,
    PART_SUFFIX, Reorder, Store,
};
use support::{TestServer, test_data};
use tempfile::TempDir;
use tokio::sync::broadcast::Receiver;

const SIZE: usize = 512 * 1024;
const TIMEOUT: Duration = Duration::from_secs(10);
const HOUR: Duration = Duration::from_secs(3600);

struct Setup {
    server: TestServer,
    dir: TempDir,
    manager: Manager,
    events: Receiver<JobEvent>,
    data: Vec<u8>,
}

/// `chunk_delay_ms` slows the server down so jobs stay running long enough to poke at.
async fn setup(max_concurrent: usize, chunk_delay_ms: u64) -> Setup {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open_in_memory().await.unwrap();
    setup_with(store, dir, max_concurrent, chunk_delay_ms).await
}

async fn setup_with(store: Store, dir: TempDir, max_concurrent: usize, delay_ms: u64) -> Setup {
    let data = test_data(SIZE, 1);
    let server = TestServer::start(data.clone()).await;
    server.configure(|b| b.chunk_delay = Some(Duration::from_millis(delay_ms)));
    let manager = Manager::start(HttpBackend::new().unwrap(), store)
        .await
        .unwrap();
    manager.set_max_concurrent(max_concurrent).await.unwrap();
    // Subscribe before adding anything so no event is missed.
    let events = manager.subscribe();
    Setup {
        server,
        dir,
        manager,
        events,
        data,
    }
}

fn from_now(after: Duration) -> SystemTime {
    SystemTime::now() + after
}

impl Setup {
    fn new_job(&self, name: &str) -> NewJob {
        NewJob::new(self.server.url(&format!("/files/{name}"))).in_dir(self.dir.path())
    }

    async fn add(&self, job: NewJob) -> JobId {
        self.manager.add(job).await.unwrap()
    }

    async fn job(&self, id: JobId) -> JobSnapshot {
        self.manager.job(id).await.unwrap()
    }

    async fn change(&self, id: JobId, change: JobChange) -> Result<(), Error> {
        self.manager.change_job(id, change).await
    }

    async fn start_at(&self, id: JobId, at: Option<SystemTime>) -> Result<(), Error> {
        let change = JobChange {
            start_at: Some(at),
            ..JobChange::default()
        };
        self.change(id, change).await
    }

    /// Waits for a `Changed` event putting `id` in `state`.
    async fn wait_for(&mut self, id: JobId, state: JobState) -> JobSnapshot {
        let events = &mut self.events;
        tokio::time::timeout(TIMEOUT, async {
            loop {
                if let JobEvent::Changed(job) = events.recv().await.unwrap()
                    && job.id == id
                    && job.state == state
                {
                    return job;
                }
            }
        })
        .await
        .unwrap_or_else(|_| panic!("{id} never became {state:?}"))
    }

    fn assert_file(&self, job: &JobSnapshot) {
        let path = self.dir.path().join(job.file_name.as_ref().unwrap());
        assert!(std::fs::read(path).unwrap() == self.data, "content differs");
    }

    fn part_len(&self, job: &JobSnapshot) -> u64 {
        let name = format!("{}{PART_SUFFIX}", job.file_name.as_ref().unwrap());
        std::fs::metadata(self.dir.path().join(name)).unwrap().len()
    }
}

#[tokio::test]
async fn a_new_job_waits_for_its_start_time() {
    let mut s = setup(3, 0).await;
    let start = from_now(Duration::from_millis(800));
    let added = Instant::now();
    let id = s.add(s.new_job("a.bin").starting_at(start)).await;

    let job = s.job(id).await;
    assert_eq!(
        (job.state, job.start_at),
        (JobState::Scheduled, Some(start))
    );
    // It joins the queue when the time comes, and the time goes.
    let queued = s.wait_for(id, JobState::Queued).await;
    assert!(
        added.elapsed() >= Duration::from_millis(700),
        "started early"
    );
    assert_eq!(queued.start_at, None);
    let done = s.wait_for(id, JobState::Completed).await;
    s.assert_file(&done);
}

#[tokio::test]
async fn a_start_time_already_past_queues_at_once() {
    let mut s = setup(3, 0).await;
    let past = SystemTime::now() - Duration::from_secs(60);
    let id = s.add(s.new_job("a.bin").starting_at(past)).await;
    let job = s.job(id).await;
    assert_ne!(job.state, JobState::Scheduled);
    assert_eq!(job.start_at, None);
    s.wait_for(id, JobState::Completed).await;
}

#[tokio::test]
async fn resuming_a_scheduled_job_starts_it_now() {
    let mut s = setup(3, 0).await;
    let id = s.add(s.new_job("a.bin").starting_at(from_now(HOUR))).await;
    s.manager.resume(id).await.unwrap();
    let queued = s.job(id).await;
    assert_ne!(queued.state, JobState::Scheduled);
    assert_eq!(queued.start_at, None);
    s.wait_for(id, JobState::Completed).await;
}

#[tokio::test]
async fn a_scheduled_job_can_be_paused_cancelled_and_reordered() {
    let mut s = setup(1, 0).await;
    let first = s.add(s.new_job("a.bin").starting_at(from_now(HOUR))).await;
    let second = s.add(s.new_job("b.bin").starting_at(from_now(HOUR))).await;
    let place = s.job(first).await.place;
    // It keeps a place in the queue, which it takes when its time comes.
    s.manager.reorder(vec![second], Reorder::Top).await.unwrap();
    assert_eq!(s.job(second).await.place, place);

    s.manager.pause(first).await.unwrap();
    let paused = s.job(first).await;
    assert_eq!((paused.state, paused.start_at), (JobState::Paused, None));
    s.manager.cancel(second).await.unwrap();
    s.wait_for(second, JobState::Cancelled).await;
}

#[tokio::test]
async fn a_start_time_makes_a_running_job_wait() {
    let mut s = setup(3, 30).await;
    let id = s.add(s.new_job("data.bin")).await;
    s.wait_for(id, JobState::Downloading).await;

    let later = from_now(HOUR);
    s.start_at(id, Some(later)).await.unwrap();
    let held = s.wait_for(id, JobState::Scheduled).await;
    assert_eq!(held.start_at, Some(later));
    assert!(held.downloaded > 0 && held.downloaded < SIZE as u64);
    assert_eq!(s.part_len(&held), held.downloaded);

    // Taking the time away queues it again; it carries on from there.
    s.start_at(id, None).await.unwrap();
    assert_eq!(s.job(id).await.start_at, None);
    let done = s.wait_for(id, JobState::Completed).await;
    s.assert_file(&done);
    let resumed_from = s.server.requests()[1].range.clone();
    assert_eq!(resumed_from, Some(format!("bytes={}-", held.downloaded)));
}

#[tokio::test]
async fn a_start_time_reschedules_paused_and_queued_jobs() {
    let mut s = setup(1, 20).await;
    let running = s.add(s.new_job("a.bin")).await;
    let queued = s.add(s.new_job("b.bin")).await;
    let paused = s.add(s.new_job("c.bin")).await;
    s.manager.pause(paused).await.unwrap();
    s.wait_for(running, JobState::Downloading).await;

    for id in [queued, paused] {
        s.start_at(id, Some(from_now(HOUR))).await.unwrap();
        assert_eq!(s.job(id).await.state, JobState::Scheduled);
    }
    // Neither takes the slot when it frees up.
    s.wait_for(running, JobState::Completed).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    for id in [queued, paused] {
        assert_eq!(s.job(id).await.state, JobState::Scheduled);
    }
    // A new time replaces the old one.
    let soon = from_now(Duration::from_millis(300));
    s.start_at(queued, Some(soon)).await.unwrap();
    assert_eq!(s.job(queued).await.start_at, Some(soon));
    s.wait_for(queued, JobState::Completed).await;
}

#[tokio::test]
async fn a_stop_time_pauses_a_running_job() {
    // About 1.3 s for the whole file.
    let mut s = setup(3, 40).await;
    let stop = from_now(Duration::from_millis(500));
    let id = s.add(s.new_job("data.bin").stopping_at(stop)).await;
    assert_eq!(s.job(id).await.stop_at, Some(stop));

    let paused = s.wait_for(id, JobState::Paused).await;
    assert_eq!(paused.stop_at, None);
    assert!(
        paused.downloaded < SIZE as u64,
        "finished before its stop time"
    );
    assert_eq!(s.part_len(&paused), paused.downloaded);
}

#[tokio::test]
async fn a_stop_time_pauses_a_job_that_never_started() {
    let mut s = setup(1, 20).await;
    let running = s.add(s.new_job("a.bin")).await;
    let waiting = s
        .add(
            s.new_job("b.bin")
                .stopping_at(from_now(Duration::from_millis(300))),
        )
        .await;
    let paused = s.wait_for(waiting, JobState::Paused).await;
    assert_eq!((paused.downloaded, paused.stop_at), (0, None));
    s.wait_for(running, JobState::Completed).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(s.job(waiting).await.state, JobState::Paused);
}

#[tokio::test]
async fn a_scheduled_job_pauses_if_its_whole_window_has_passed() {
    // Stopped by its own stop time before it could start: a shutdown, say,
    // over the whole window. Both times have passed when it's checked.
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("ottie.db");
    let s = setup_with(Store::open(&db).await.unwrap(), dir, 3, 0).await;
    let window = s
        .new_job("a.bin")
        .starting_at(from_now(Duration::from_millis(200)))
        .stopping_at(from_now(Duration::from_millis(400)));
    let id = s.add(window).await;
    s.manager.shutdown().await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    let Setup { dir, .. } = s;
    let again = setup_with(Store::open(&db).await.unwrap(), dir, 3, 0).await;
    let job = again.job(id).await;
    assert_eq!(job.state, JobState::Paused);
    assert_eq!((job.start_at, job.stop_at), (None, None));
}

#[tokio::test]
async fn a_scheduled_job_still_waits_after_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("ottie.db");
    let s = setup_with(Store::open(&db).await.unwrap(), dir, 3, 0).await;
    let later = from_now(HOUR);
    let id = s.add(s.new_job("a.bin").starting_at(later)).await;
    s.manager.shutdown().await.unwrap();

    let Setup { dir, .. } = s;
    let again = setup_with(Store::open(&db).await.unwrap(), dir, 3, 0).await;
    let job = again.job(id).await;
    // Saved to the millisecond.
    let millis = |at: SystemTime| {
        at.duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    };
    assert_eq!(job.state, JobState::Scheduled);
    assert_eq!(job.start_at.map(millis), Some(millis(later)));
}

#[tokio::test]
async fn the_stop_time_must_come_after_the_start_time() {
    let s = setup(3, 0).await;
    let start = from_now(HOUR);
    let refused = s
        .manager
        .add(s.new_job("a.bin").starting_at(start).stopping_at(start))
        .await;
    assert!(
        matches!(refused, Err(Error::StopBeforeStart)),
        "{refused:?}"
    );

    let id = s.add(s.new_job("a.bin").starting_at(start)).await;
    let early_stop = JobChange {
        stop_at: Some(Some(start - Duration::from_secs(1))),
        ..JobChange::default()
    };
    let refused = s.change(id, early_stop).await;
    assert!(
        matches!(refused, Err(Error::StopBeforeStart)),
        "{refused:?}"
    );
    // Both at once are checked together.
    let both = JobChange {
        start_at: Some(Some(start + HOUR)),
        stop_at: Some(Some(start + 2 * HOUR)),
        ..JobChange::default()
    };
    s.change(id, both).await.unwrap();
    let job = s.job(id).await;
    assert_eq!(
        (job.start_at, job.stop_at),
        (Some(start + HOUR), Some(start + 2 * HOUR))
    );
}

#[tokio::test]
async fn finished_jobs_take_no_times() {
    let mut s = setup(3, 0).await;
    let id = s.add(s.new_job("a.bin")).await;
    s.wait_for(id, JobState::Completed).await;
    let refused = s.start_at(id, Some(from_now(HOUR))).await;
    assert!(
        matches!(
            refused,
            Err(Error::InvalidTransition {
                from: JobState::Completed,
                to: JobState::Scheduled
            })
        ),
        "{refused:?}"
    );
    // Taking away a time it doesn't have changes nothing.
    s.start_at(id, None).await.unwrap();
}

#[tokio::test]
async fn a_change_of_times_is_announced_once() {
    let mut s = setup(3, 0).await;
    let id = s.add(s.new_job("a.bin").starting_at(from_now(HOUR))).await;
    // Drain the `Changed` of the add.
    s.wait_for(id, JobState::Scheduled).await;

    let change = JobChange {
        start_at: Some(Some(from_now(2 * HOUR))),
        stop_at: Some(Some(from_now(3 * HOUR))),
        speed_limit: Some(1000),
        ..JobChange::default()
    };
    s.change(id, change).await.unwrap();
    s.change(id, change).await.unwrap(); // The same again: nothing changes.
    s.manager.pause(id).await.unwrap(); // A marker after them.
    let mut changes = Vec::new();
    while let Ok(event) = s.events.try_recv() {
        if let JobEvent::Changed(job) = event {
            changes.push(job.state);
        }
    }
    assert_eq!(changes, [JobState::Scheduled, JobState::Paused]);
}
