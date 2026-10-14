#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::collections::HashSet;
use std::time::Duration;

use ottie_core::{
    Error, HttpBackend, JobChange, JobEvent, JobId, JobSnapshot, JobState, Manager, NewJob,
    Priority, Problem, Reorder, SettingsChange, Store,
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

#[tokio::test]
async fn removing_a_running_job_stops_it_and_deletes_the_part_file() {
    let mut s = setup(3, 5).await;
    let id = s.add("data.bin").await;
    s.wait_for(id, JobState::Downloading).await;

    // Like pause, it answers once the job has stopped.
    s.manager.remove(id).await.unwrap();
    assert!(s.manager.list().await.unwrap().is_empty());
    assert_eq!(std::fs::read_dir(s.dir.path()).unwrap().count(), 0);
    tokio::time::timeout(TIMEOUT, async {
        loop {
            if let JobEvent::Removed(removed) = s.events.recv().await.unwrap() {
                assert_eq!(removed, id);
                return;
            }
        }
    })
    .await
    .expect("no Removed event");
}

#[tokio::test]
async fn removing_a_completed_job_keeps_the_file() {
    let mut s = setup(3, 0).await;
    let id = s.add("data.bin").await;
    let done = s.wait_for(id, JobState::Completed).await;

    s.manager.remove(id).await.unwrap();
    assert!(s.manager.list().await.unwrap().is_empty());
    s.assert_file(&done);
}

#[tokio::test]
async fn jobs_can_be_removed_in_any_state() {
    let mut s = setup(1, 5).await;
    let running = s.add("a.bin").await;
    let queued = s.add("b.bin").await;
    let paused = s.add("c.bin").await;
    let cancelled = s.add("d.bin").await;
    s.manager.pause(paused).await.unwrap();
    s.manager.cancel(cancelled).await.unwrap();
    s.wait_for(running, JobState::Downloading).await;

    for id in [queued, paused, cancelled] {
        s.manager.remove(id).await.unwrap();
    }
    let left: Vec<JobId> = s
        .manager
        .list()
        .await
        .unwrap()
        .iter()
        .map(|j| j.id)
        .collect();
    assert_eq!(left, [running]);
    assert!(matches!(
        s.manager.remove(queued).await,
        Err(Error::JobNotFound(_))
    ));
}

#[tokio::test]
async fn removing_a_running_job_frees_its_slot() {
    let mut s = setup(1, 5).await;
    let first = s.add("a.bin").await;
    let second = s.add("b.bin").await;
    s.wait_for(first, JobState::Downloading).await;

    s.manager.remove(first).await.unwrap();
    let done = s.wait_for(second, JobState::Completed).await;
    s.assert_file(&done);
}

#[tokio::test]
async fn changed_settings_are_announced_once() {
    let mut s = setup(1, 0).await;
    let change = SettingsChange {
        max_concurrent: Some(4),
        global_limit: Some(1_000_000),
        ..SettingsChange::default()
    };
    let settings = s.manager.change_settings(change.clone()).await.unwrap();
    assert_eq!(
        (settings.max_concurrent, settings.global_limit),
        (4, 1_000_000)
    );
    match s.events.try_recv() {
        Ok(JobEvent::SettingsChanged(announced)) => assert_eq!(announced, settings),
        other => panic!("expected the new settings, got {other:?}"),
    }

    // Nothing new: nothing to announce.
    s.manager.change_settings(change).await.unwrap();
    assert!(s.events.try_recv().is_err());
}

#[tokio::test]
async fn a_job_change_is_announced_once() {
    let mut s = setup(1, 50).await;
    let id = s.add("a.bin").await;
    s.manager.pause(id).await.unwrap();
    while s.events.try_recv().is_ok() {} // What came before.

    let change = JobChange {
        speed_limit: Some(64 * 1024),
        priority: Some(Priority::Urgent),
        streaming: Some(true),
    };
    s.manager.change_job(id, change).await.unwrap();
    match s.events.try_recv() {
        Ok(JobEvent::Changed(job)) => {
            assert_eq!(
                (job.speed_limit, job.priority, job.streaming),
                (64 * 1024, Priority::Urgent, true)
            );
        }
        other => panic!("expected the changed job, got {other:?}"),
    }
    assert!(s.events.try_recv().is_err(), "one event for every field");

    // Nothing new: nothing to announce.
    s.manager.change_job(id, change).await.unwrap();
    s.manager
        .change_job(id, JobChange::default())
        .await
        .unwrap();
    assert!(s.events.try_recv().is_err());

    s.manager.remove(id).await.unwrap();
    assert!(matches!(
        s.manager.change_job(id, change).await,
        Err(Error::JobNotFound(_))
    ));
}

#[tokio::test]
async fn reordering_changes_which_job_starts_next() {
    let mut s = setup(1, 5).await;
    let a = s.add("a.bin").await; // starts at once
    let b = s.add("b.bin").await;
    let c = s.add("c.bin").await;
    let d = s.add("d.bin").await;
    s.manager.reorder(vec![d], Reorder::Top).await.unwrap();
    s.manager.reorder(vec![b], Reorder::Down).await.unwrap();

    let mut finished = Vec::new();
    tokio::time::timeout(TIMEOUT, async {
        while finished.len() < 4 {
            if let JobEvent::Changed(job) = s.events.recv().await.unwrap()
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

#[tokio::test]
async fn reordering_moves_waiting_jobs_within_their_priority() {
    let mut s = setup(1, 50).await;
    let running = s.add("a.bin").await;
    let high = s.add("b.bin").await;
    let queued = s.add("c.bin").await;
    let paused = s.add("d.bin").await;
    s.manager.set_priority(high, Priority::High).await.unwrap();
    s.manager.pause(paused).await.unwrap();
    while s.events.try_recv().is_ok() {} // What came before.
    let place = |jobs: &[JobSnapshot], id| jobs.iter().find(|j| j.id == id).unwrap().place;
    let before = s.manager.list().await.unwrap();

    // First of the Normal ones (the High one is in a lane of its own), and
    // a running job has no place to move to.
    s.manager
        .reorder(vec![queued, running], Reorder::Up)
        .await
        .unwrap();
    assert!(s.events.try_recv().is_err(), "nothing moved");

    // A paused job moves too; the two swap places, and both are announced.
    s.manager.reorder(vec![paused], Reorder::Up).await.unwrap();
    let mut moved = Vec::new();
    while let Ok(JobEvent::Changed(job)) = s.events.try_recv() {
        moved.push((job.id, job.place));
    }
    let expected = [
        (queued, place(&before, paused)),
        (paused, place(&before, queued)),
    ];
    assert_eq!(moved, expected);

    // Dropped after the queued one: back where it was.
    s.manager
        .reorder(vec![paused], Reorder::After(queued))
        .await
        .unwrap();
    let places: Vec<(JobId, u64)> = s
        .manager
        .list()
        .await
        .unwrap()
        .iter()
        .filter(|j| [queued, paused].contains(&j.id))
        .map(|j| (j.id, j.place))
        .collect();
    assert_eq!(
        places,
        [
            (queued, place(&before, queued)),
            (paused, place(&before, paused))
        ]
    );
    // Next to a job of another priority, or one that isn't waiting: nothing moves.
    while s.events.try_recv().is_ok() {}
    for to in [Reorder::Before(high), Reorder::After(running)] {
        s.manager.reorder(vec![paused], to).await.unwrap();
    }
    assert!(s.events.try_recv().is_err(), "nothing moved");

    // An unknown job: nothing moves.
    s.manager.remove(high).await.unwrap();
    while s.events.try_recv().is_ok() {}
    for to in [Reorder::Top, Reorder::Before(high)] {
        let ids = if to == Reorder::Top {
            vec![queued, high]
        } else {
            vec![queued]
        };
        assert!(matches!(
            s.manager.reorder(ids, to).await,
            Err(Error::JobNotFound(id)) if id == high
        ));
    }
    assert!(s.events.try_recv().is_err());
}

#[tokio::test]
async fn videos_and_music_start_in_streaming_mode() {
    let s = setup(0, 0).await;
    let video = s.add("film.mp4").await;
    let music = s.add("song.FLAC").await;
    let archive = s.add("files.zip").await;
    assert!(s.job(video).await.streaming);
    assert!(s.job(music).await.streaming);
    assert!(!s.job(archive).await.streaming);

    // Asked for: whatever the file type.
    let url = s.server.url("/files/files.zip");
    let asked = NewJob::new(url).in_dir(s.dir.path()).streaming(true);
    let asked = s.manager.add(asked).await.unwrap();
    assert!(s.job(asked).await.streaming);

    // Turned off in the settings: from now on, only when asked for.
    let off = SettingsChange {
        stream_media: Some(false),
        ..SettingsChange::default()
    };
    s.manager.change_settings(off).await.unwrap();
    let later = s.add("other.mkv").await;
    assert!(!s.job(later).await.streaming);
}

#[tokio::test]
async fn the_real_name_decides_streaming_mode_unless_chosen() {
    let mut s = setup(3, 0).await;
    let header = "attachment; filename=\"film.mkv\"";
    s.server
        .configure(|b| b.content_disposition = Some(header.as_bytes().to_vec()));
    let guessed = s.add("get.php").await;
    let url = s.server.url("/files/get.php");
    let chosen = NewJob::new(url).in_dir(s.dir.path()).streaming(false);
    let chosen = s.manager.add(chosen).await.unwrap();
    assert!(!s.job(guessed).await.streaming, "from .php");

    s.wait_for_all(&[guessed, chosen], JobState::Completed)
        .await;
    assert!(s.job(guessed).await.streaming, "from .mkv");
    assert!(!s.job(chosen).await.streaming, "the choice stays");
}
