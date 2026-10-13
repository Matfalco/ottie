//! What goes over the wire. These are the API's own types, converted from
//! the engine's, so the engine can change without breaking clients. `ts-rs`
//! writes a TypeScript version of each into `shared/api-types/` when the
//! tests run (`cargo test -p ottie-api`); clients use only those.
//!
//! Byte counts and times are `u64` in Rust but plain `number`s in
//! TypeScript (JSON has no bigger integers); they stay far below 2^53.

use std::path::Path;
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
    /// Why it failed, or the last failure while `retrying`, in English.
    pub error: Option<String>,
    /// The same, as a code to translate (see `Problem`).
    pub problem: Option<Problem>,
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

/// A category: where its downloads go and which files it takes. As listed
/// by `GET /categories` and sent in `categoryChanged` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Category {
    /// General is 1.
    pub id: u32,
    pub name: String,
    /// Where its downloads are saved; null: each download must name a folder.
    pub folder: Option<String>,
    /// Lowercase, without the dot. Each belongs to one category at most.
    pub extensions: Vec<String>,
    /// Bytes per second for all its downloads together; 0 = none.
    #[ts(type = "number")]
    pub speed_limit: u64,
    /// How many of its downloads may run at once; 0 = no cap of its own.
    pub max_concurrent: u32,
    /// Null for the user's own categories.
    pub built_in: Option<BuiltIn>,
}

/// The categories Ottie itself needs; they can't be removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum BuiltIn {
    /// Downloads that match no extension; its folder is the default one.
    General,
    /// Downloads saved in a folder of no category. It never has a folder or
    /// extensions (they are ignored when it's changed).
    Other,
}

/// Body of `POST /categories` and `PUT /categories/{id}`: every setting (a
/// PUT replaces them all).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CategoryBody {
    pub name: String,
    /// An absolute folder, or null.
    pub folder: Option<String>,
    /// With or without the dot, any case: `.MKV` and `mkv` are the same.
    pub extensions: Vec<String>,
    #[ts(type = "number")]
    pub speed_limit: u64,
    pub max_concurrent: u32,
}

/// Answer to `POST /categories`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AddedCategory {
    pub id: u32,
}

/// Body of `POST /categories/match`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MatchLink {
    pub url: String,
    /// The folder it would be saved in, if not its category's: then the
    /// folder decides the category.
    #[serde(default)]
    #[ts(optional)]
    pub dest_dir: Option<String>,
}

/// Answer to `POST /categories/match`: the category a download of that link
/// would get. The server's real file name may still move it, unless the user
/// picks the category or a folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Matched {
    pub category: u32,
    /// The extension the link suggests, if any.
    pub extension: Option<String>,
    /// The folder decided: it's this category's folder, or (Other) the
    /// folder of none.
    pub by_folder: bool,
}

/// Body of `POST /jobs/{id}/category`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MoveJob {
    pub category: u32,
}

/// What applies to the whole daemon, as `GET /settings` answers it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Settings {
    /// The first-start setup (starter categories) was done or skipped;
    /// until then, front ends offer it.
    pub setup_done: bool,
}

/// Body of `PATCH /settings`: only the fields given change.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SettingsChange {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub setup_done: Option<bool>,
}

/// Why a job failed or is retrying. Clients translate it by `code` and
/// fall back to the job's English `error` for codes they don't know.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "code",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum Problem {
    /// Connecting or reading failed; `detail` is English.
    Network {
        detail: String,
    },
    /// Too many redirects, or one to something other than http(s).
    Redirect {
        detail: String,
    },
    /// The server answered with a status we can't use (404, 503…).
    HttpStatus {
        status: u16,
    },
    /// No data for this many seconds.
    Stalled {
        #[ts(type = "number")]
        seconds: u64,
    },
    /// The connection closed before the end of the file.
    Incomplete {
        #[ts(type = "number")]
        received: u64,
        #[ts(type = "number")]
        expected: u64,
    },
    /// The file kept changing on the server.
    RemoteChanging,
    /// Writing the file failed; `detail` is English.
    Disk {
        detail: String,
    },
    Other {
        detail: String,
    },
}

/// Body of every error answer: English text, plus a code (and details) for
/// clients to translate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ApiError {
    /// English: for logs, and for codes a client doesn't know.
    pub error: String,
    #[serde(flatten)]
    pub reason: Reason,
}

/// Why a request was refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "code",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum Reason {
    /// The token is missing or wrong (401).
    Unauthorized,
    NoSuchEndpoint,
    /// The body isn't the JSON expected; `error` says what's wrong.
    InvalidBody,
    InvalidUrl,
    /// `destDir` must be an absolute path.
    RelativeFolder,
    UnsupportedScheme {
        scheme: String,
    },
    NonUtf8Path,
    /// No folder given, and the category has none.
    NoFolder {
        category: String,
    },
    InvalidJobId {
        id: String,
    },
    JobNotFound {
        id: String,
    },
    /// The job's state doesn't allow it (resuming a completed job…).
    InvalidTransition {
        from: JobState,
        to: JobState,
    },
    CategoryNotFound {
        id: u32,
    },
    InvalidCategoryName,
    InvalidExtension {
        extension: String,
    },
    CategoryNameTaken {
        name: String,
    },
    ExtensionTaken {
        extension: String,
        category: String,
    },
    /// General and Other can't be removed.
    CannotRemoveBuiltIn {
        category: String,
    },
    /// The daemon is shutting down.
    Stopping,
    /// A bug or an unexpected failure; `error` has the details.
    Internal,
}

/// A message on the `GET /events` WebSocket. Connect first, then fetch
/// `GET /jobs`, then apply events in order (insert-or-replace by `id`, or
/// drop on `removed`).
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
    /// A job was removed (`DELETE /jobs/{id}`): drop it.
    Removed { id: String },
    /// A category was added or changed: insert-or-replace by `id`.
    CategoryChanged { category: Category },
    /// A category was removed; its jobs were moved to General first (and a
    /// `changed` event was sent for each).
    CategoryRemoved { id: u32 },
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
            error: job.problem.as_ref().map(ToString::to_string),
            problem: job.problem.map(Problem::from),
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

impl From<ottie_core::Settings> for Settings {
    fn from(settings: ottie_core::Settings) -> Self {
        Settings {
            setup_done: settings.setup_done,
        }
    }
}

impl From<ottie_core::Category> for Category {
    fn from(category: ottie_core::Category) -> Self {
        let settings = category.settings;
        Category {
            id: category.id.0,
            name: settings.name,
            folder: settings.folder.as_deref().map(path_text),
            extensions: settings.extensions,
            speed_limit: settings.speed_limit,
            max_concurrent: u32::try_from(settings.max_concurrent).unwrap_or(u32::MAX),
            built_in: category.built_in.map(|built_in| match built_in {
                ottie_core::BuiltIn::General => BuiltIn::General,
                ottie_core::BuiltIn::Other => BuiltIn::Other,
            }),
        }
    }
}

/// Category folders are checked to be UTF-8 when set, so nothing is lost.
fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// Spelled out, so engine changes can't silently change the API.
impl From<ottie_core::Problem> for Problem {
    fn from(problem: ottie_core::Problem) -> Self {
        use ottie_core::Problem as P;
        match problem {
            P::Network { detail } => Problem::Network { detail },
            P::Redirect { detail } => Problem::Redirect { detail },
            P::HttpStatus { status } => Problem::HttpStatus { status },
            P::Stalled { seconds } => Problem::Stalled { seconds },
            P::Incomplete { received, expected } => Problem::Incomplete { received, expected },
            P::RemoteChanging => Problem::RemoteChanging,
            P::Disk { detail } => Problem::Disk { detail },
            P::Other { detail } => Problem::Other { detail },
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
