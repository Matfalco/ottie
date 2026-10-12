use std::path::PathBuf;

use reqwest::Url;

use crate::download::{Download, RestartReason};
use crate::id::JobId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    /// Waiting for a free slot (see the manager's max concurrent downloads).
    Queued,
    /// Running, no bytes received yet.
    Connecting,
    Downloading,
    Paused,
    Completed,
    /// Stopped by an error; can be resumed.
    Failed,
    Cancelled,
}

impl JobState {
    /// The state machine. Every change of state is checked against this.
    pub fn can_become(self, next: JobState) -> bool {
        use JobState::*;
        matches!(
            (self, next),
            (Queued, Connecting | Paused | Cancelled)
                | (
                    Connecting,
                    Downloading | Completed | Paused | Failed | Cancelled
                )
                | (Downloading, Completed | Paused | Failed | Cancelled)
                | (Paused | Failed, Queued | Cancelled)
        )
    }

    pub fn is_running(self) -> bool {
        matches!(self, JobState::Connecting | JobState::Downloading)
    }
}

/// A copy of a job's public state, for listing and events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobSnapshot {
    pub id: JobId,
    pub url: Url,
    pub dest_dir: PathBuf,
    pub file_name: Option<String>,
    pub state: JobState,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub resumable: bool,
    /// Why the job failed, when `state` is `Failed`.
    pub error: Option<String>,
}

impl JobSnapshot {
    pub(crate) fn new(id: JobId, state: JobState, dl: &Download, error: Option<String>) -> Self {
        Self {
            id,
            url: dl.url.clone(),
            dest_dir: dl.dest_dir.clone(),
            file_name: dl.file_name.clone(),
            state,
            downloaded: dl.downloaded,
            total: dl.total,
            resumable: dl.resumable,
            error,
        }
    }
}

/// Broadcast to every subscriber of the manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobEvent {
    /// A job was added or changed state. Clients can insert-or-replace by `id`.
    Changed(JobSnapshot),
    /// Bytes arrived. Sent at most a few times per second per job.
    Progress {
        id: JobId,
        downloaded: u64,
        total: Option<u64>,
    },
    /// The job threw away its partial file and started over.
    Restarted { id: JobId, reason: RestartReason },
}

#[cfg(test)]
mod tests {
    use super::JobState::*;
    use super::*;

    const ALL: [JobState; 7] = [
        Queued,
        Connecting,
        Downloading,
        Paused,
        Completed,
        Failed,
        Cancelled,
    ];

    #[test]
    fn finished_states_are_final() {
        for next in ALL {
            assert!(!Completed.can_become(next));
            assert!(!Cancelled.can_become(next));
        }
    }

    #[test]
    fn no_state_becomes_itself() {
        for state in ALL {
            assert!(!state.can_become(state), "{state:?}");
        }
    }

    #[test]
    fn resuming_goes_back_through_the_queue() {
        assert!(Paused.can_become(Queued));
        assert!(Failed.can_become(Queued));
        assert!(!Paused.can_become(Downloading));
        assert!(!Paused.can_become(Connecting));
    }
}
