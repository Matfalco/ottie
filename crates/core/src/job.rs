use std::path::PathBuf;
use std::time::SystemTime;

use reqwest::Url;

use crate::category::{Category, CategoryId};
use crate::download::RestartReason;
use crate::id::JobId;
use crate::problem::Problem;
use crate::settings::Settings;

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
    /// Stopped by an error that may go away; waiting to try again. Holds its
    /// download slot until it gives way to a waiting job (docs/decisions/0013).
    Retrying,
}

impl JobState {
    /// The state machine. Every change of state is checked against this.
    pub fn can_become(self, next: JobState) -> bool {
        use JobState::*;
        matches!(
            (self, next),
            (Queued, Connecting | Paused | Cancelled)
                | (Connecting, Downloading)
                | (
                    Connecting | Downloading,
                    Completed | Paused | Failed | Cancelled | Retrying
                )
                | (Retrying, Connecting | Queued | Paused | Cancelled)
                | (Paused | Failed, Queued | Cancelled)
        )
    }

    pub fn is_running(self) -> bool {
        matches!(self, JobState::Connecting | JobState::Downloading)
    }
}

/// Decides which queued job starts next and how the global speed limit is
/// shared. See docs/decisions/0010.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Priority {
    /// Half a Normal download's share of the global limit.
    Low,
    #[default]
    Normal,
    /// Twice a Normal download's share.
    High,
    /// Served before everything else: others only get what Urgent jobs can't use.
    Urgent,
}

/// What [`crate::Manager::add`] needs to know about a new download.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewJob {
    pub url: Url,
    /// Where to save it. `None`: the category's folder.
    pub dest_dir: Option<PathBuf>,
    /// `None`: the category whose extensions include the file extension in
    /// the URL, or General if none does.
    pub category: Option<CategoryId>,
}

impl NewJob {
    pub fn new(url: Url) -> Self {
        Self {
            url,
            dest_dir: None,
            category: None,
        }
    }

    /// Without [`NewJob::in_category`], the folder also decides the category:
    /// the one whose folder it is, else the built-in Other.
    pub fn in_dir(mut self, dest_dir: impl Into<PathBuf>) -> Self {
        self.dest_dir = Some(dest_dir.into());
        self
    }

    pub fn in_category(mut self, category: CategoryId) -> Self {
        self.category = Some(category);
        self
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
    /// Why the job failed (`Failed`), or the last failure (`Retrying`, or
    /// `Queued` after giving its slot away). `Display` gives English text.
    pub problem: Option<Problem>,
    pub priority: Priority,
    /// This job's own limit in bytes per second; 0 = none.
    pub speed_limit: u64,
    pub category: CategoryId,
    /// Failed attempts since bytes last arrived. With `Retrying`, the number
    /// of the retry it's waiting for.
    pub retries: u32,
    /// When it will try again, while `Retrying`.
    pub retry_at: Option<SystemTime>,
}

/// Broadcast to every subscriber of the manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobEvent {
    /// A job was added, changed state, or learned its file name and size.
    /// Clients can insert-or-replace by `id`.
    Changed(JobSnapshot),
    /// Bytes arrived. Sent at most a few times per second per job.
    Progress {
        id: JobId,
        downloaded: u64,
        total: Option<u64>,
    },
    /// The job was removed. Clients drop it.
    Removed(JobId),
    /// The job threw away its partial file and started over.
    Restarted { id: JobId, reason: RestartReason },
    /// A category was added or changed. Clients can insert-or-replace by `id`.
    CategoryChanged(Category),
    /// A category was removed; its jobs were moved to General first.
    CategoryRemoved(CategoryId),
    /// The settings changed; here they are, all of them.
    SettingsChanged(Settings),
}

#[cfg(test)]
mod tests {
    use super::JobState::*;
    use super::*;

    const ALL: [JobState; 8] = [
        Queued,
        Connecting,
        Downloading,
        Paused,
        Completed,
        Failed,
        Cancelled,
        Retrying,
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
    fn retrying_starts_again_or_gives_way() {
        assert!(Downloading.can_become(Retrying));
        assert!(Retrying.can_become(Connecting));
        assert!(Retrying.can_become(Queued));
        assert!(Retrying.can_become(Paused));
        assert!(!Retrying.can_become(Downloading));
        assert!(!Queued.can_become(Retrying));
    }

    #[test]
    fn resuming_goes_back_through_the_queue() {
        assert!(Paused.can_become(Queued));
        assert!(Failed.can_become(Queued));
        assert!(!Paused.can_become(Downloading));
        assert!(!Paused.can_become(Connecting));
    }
}
