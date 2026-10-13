//! Where players are reading a download's file (docs/decisions/0029). Each
//! [`Reader`](crate::Reader) keeps its next offset here, and a multi-connection
//! download sends its connections there (see `segmented.rs`). In return, the
//! download tells readers which ranges are already in the file.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use crate::download::Segment;
use crate::id::JobId;

/// The offsets players are reading in one job's file. Cheap to clone; a
/// backend gets one for each run, like its [`Throttle`](crate::Throttle).
#[derive(Clone, Default)]
pub struct Playback {
    /// `None` = nobody can read this download (outside the manager).
    heads: Option<Arc<Heads>>,
}

#[derive(Default)]
struct Heads(Mutex<Players>);

#[derive(Default)]
struct Players {
    next_key: u64,
    /// By key, so in the order the readers opened.
    by_key: BTreeMap<u64, u64>,
    /// While a multi-connection download runs and someone reads: its ranges
    /// as written to the file, which readers can see before they're synced
    /// to disk and saved.
    written: Option<Vec<Segment>>,
}

impl Heads {
    fn lock(&self) -> MutexGuard<'_, Players> {
        // Nothing can panic while it's held, so a poisoned lock is still consistent.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Playback {
    /// For downloads outside the manager: nobody reads them.
    pub fn none() -> Self {
        Self::default()
    }

    /// Where each player reads next, the most recently opened first (a
    /// player that seeks opens a new request).
    pub(crate) fn offsets(&self) -> Vec<u64> {
        let Some(heads) = &self.heads else {
            return Vec::new();
        };
        heads.lock().by_key.values().rev().copied().collect()
    }

    /// Tells readers which ranges are in the file now; `None` once the run
    /// ends (the job's saved ranges tell then).
    pub(crate) fn show_written(&self, segments: Option<Vec<Segment>>) {
        if let Some(heads) = &self.heads {
            heads.lock().written = segments;
        }
    }
}

/// One player's place in a job's file. Dropping it forgets the player.
pub(crate) struct Head {
    heads: Arc<Heads>,
    key: u64,
}

impl Head {
    pub(crate) fn set(&self, offset: u64) {
        self.heads.lock().by_key.insert(self.key, offset);
    }

    /// The ranges a running download has written to the file, if it said.
    pub(crate) fn written(&self) -> Option<Vec<Segment>> {
        self.heads.lock().written.clone()
    }
}

impl Drop for Head {
    fn drop(&mut self) {
        self.heads.lock().by_key.remove(&self.key);
    }
}

/// Every job's [`Playback`], shared by the manager's handles (readers join
/// through them) and its actor (which hands them to running jobs). A job's
/// entry lives as long as a reader or a run holds it.
#[derive(Clone, Default)]
pub(crate) struct Playbacks(Arc<Mutex<HashMap<JobId, Weak<Heads>>>>);

impl Playbacks {
    pub(crate) fn of(&self, id: JobId) -> Playback {
        Playback {
            heads: Some(self.heads(id)),
        }
    }

    /// A new player for the job, with no offset until it reads.
    pub(crate) fn head(&self, id: JobId) -> Head {
        let heads = self.heads(id);
        let key = {
            let mut offsets = heads.lock();
            offsets.next_key += 1;
            offsets.next_key
        };
        Head { heads, key }
    }

    fn heads(&self, id: JobId) -> Arc<Heads> {
        let mut jobs = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        jobs.retain(|_, heads| heads.strong_count() > 0);
        if let Some(heads) = jobs.get(&id).and_then(Weak::upgrade) {
            return heads;
        }
        let heads = Arc::new(Heads::default());
        jobs.insert(id, Arc::downgrade(&heads));
        heads
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(n: u32) -> JobId {
        format!("2026.10.13-08.00.00-{n}").parse().unwrap()
    }

    #[test]
    fn a_running_job_sees_its_readers() {
        let playbacks = Playbacks::default();
        let run = playbacks.of(job(1));
        let first = playbacks.head(job(1));
        let second = playbacks.head(job(1));
        let other = playbacks.head(job(2));
        assert!(run.offsets().is_empty(), "nobody has read yet");

        first.set(100);
        second.set(5000);
        other.set(7);
        assert_eq!(run.offsets(), [5000, 100]);
        first.set(200);
        assert_eq!(run.offsets(), [5000, 200]);
        drop(second);
        assert_eq!(run.offsets(), [200]);
    }

    #[test]
    fn a_reader_that_came_first_is_seen_by_the_next_run() {
        let playbacks = Playbacks::default();
        let head = playbacks.head(job(1));
        head.set(42);
        assert_eq!(playbacks.of(job(1)).offsets(), [42]);
        drop(head);
        assert!(playbacks.of(job(1)).offsets().is_empty());
        assert!(Playback::none().offsets().is_empty());
    }

    #[test]
    fn readers_see_what_the_run_has_written() {
        let playbacks = Playbacks::default();
        let run = playbacks.of(job(1));
        let head = playbacks.head(job(1));
        assert_eq!(head.written(), None);
        let written = vec![Segment {
            start: 0,
            end: 100,
            done: 40,
        }];
        run.show_written(Some(written.clone()));
        assert_eq!(head.written(), Some(written));
        run.show_written(None);
        assert_eq!(head.written(), None, "the saved ranges tell again");
    }

    #[test]
    fn unused_entries_go_away() {
        let playbacks = Playbacks::default();
        drop(playbacks.head(job(1)));
        drop(playbacks.of(job(2)));
        let _kept = playbacks.of(job(3));
        assert_eq!(playbacks.0.lock().unwrap().len(), 1);
    }
}
