//! What goes over the wire. These are the API's own types, converted from
//! the engine's, so the engine can change without breaking clients. `ts-rs`
//! writes a TypeScript version of each into `shared/api-types/` when the
//! tests run (`cargo test -p ottie-api`); clients use only those.
//!
//! Byte counts and times are `u64` in Rust but plain `number`s in
//! TypeScript (JSON has no bigger integers); they stay far below 2^53.

use std::time::UNIX_EPOCH;

use ottie_core::JobSnapshot;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One download, as listed by `GET /jobs` and sent in `changed` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Job {
    /// Like `2026.10.12-14.03.07-1`; sorting by it sorts by creation.
    pub id: String,
    pub url: String,
    pub dest_dir: String,
    /// Known once the server has answered.
    pub file_name: Option<String>,
    pub state: JobState,
    #[ts(type = "number")]
    pub downloaded: u64,
    /// Size in bytes, if the server said.
    #[ts(type = "number | null")]
    pub total: Option<u64>,
    /// Whether pausing keeps what's downloaded.
    pub resumable: bool,
    /// Why it failed, or the last error while `retrying`.
    pub error: Option<String>,
    pub priority: Priority,
    /// Bytes per second; 0 = no limit of its own.
    #[ts(type = "number")]
    pub speed_limit: u64,
    pub category: u32,
    /// Failed attempts since bytes last arrived.
    pub retries: u32,
    /// While `retrying`: when, in milliseconds since 1970 (UTC).
    #[ts(type = "number | null")]
    pub retry_at: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum JobState {
    Queued,
    Connecting,
    Downloading,
    Paused,
    Completed,
    Failed,
    Cancelled,
    Retrying,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Priority {
    Low,
    Normal,
    High,
    Urgent,
}

/// Body of `POST /jobs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AddJob {
    pub url: String,
    /// An absolute folder. Default: the category's folder.
    #[serde(default)]
    #[ts(optional)]
    pub dest_dir: Option<String>,
    /// Default: matched by the file's extension.
    #[serde(default)]
    #[ts(optional)]
    pub category: Option<u32>,
}

/// Answer to `POST /jobs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Added {
    pub id: String,
}

/// Body of every error answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ApiError {
    pub error: String,
}

/// A message on the `GET /events` WebSocket. Connect first, then fetch
/// `GET /jobs`, then apply events in order (insert-or-replace by `id`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum Event {
    /// A job was added or changed state, name or settings.
    Changed { job: Job },
    /// Bytes arrived; sent a few times per second per running job.
    Progress {
        id: String,
        #[ts(type = "number")]
        downloaded: u64,
        #[ts(type = "number | null")]
        total: Option<u64>,
    },
    /// Events were missed (the client was too slow): fetch `GET /jobs` again.
    Resync,
}

impl From<JobSnapshot> for Job {
    fn from(job: JobSnapshot) -> Self {
        Job {
            id: job.id.to_string(),
            url: job.url.into(),
            dest_dir: job.dest_dir.to_string_lossy().into_owned(),
            file_name: job.file_name,
            state: job.state.into(),
            downloaded: job.downloaded,
            total: job.total,
            resumable: job.resumable,
            error: job.error,
            priority: job.priority.into(),
            speed_limit: job.speed_limit,
            category: job.category.0,
            retries: job.retries,
            retry_at: job
                .retry_at
                .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
                .map(|since| u64::try_from(since.as_millis()).unwrap_or(u64::MAX)),
        }
    }
}

/// Spelled out, so renaming an engine state can't silently change the API.
impl From<ottie_core::JobState> for JobState {
    fn from(state: ottie_core::JobState) -> Self {
        use ottie_core::JobState as S;
        match state {
            S::Queued => JobState::Queued,
            S::Connecting => JobState::Connecting,
            S::Downloading => JobState::Downloading,
            S::Paused => JobState::Paused,
            S::Completed => JobState::Completed,
            S::Failed => JobState::Failed,
            S::Cancelled => JobState::Cancelled,
            S::Retrying => JobState::Retrying,
        }
    }
}

impl From<ottie_core::Priority> for Priority {
    fn from(priority: ottie_core::Priority) -> Self {
        use ottie_core::Priority as P;
        match priority {
            P::Low => Priority::Low,
            P::Normal => Priority::Normal,
            P::High => Priority::High,
            P::Urgent => Priority::Urgent,
        }
    }
}
