//! What goes over the wire. These are the API's own types, converted from
//! the engine's, so the engine can change without breaking clients. `ts-rs`
//! writes a TypeScript version of each into `shared/api-types/` when the
//! tests run (`cargo test -p ottie-api`); clients use only those.
//!
//! Byte counts and times are `u64` in Rust but plain `number`s in
//! TypeScript (JSON has no bigger integers); they stay far below 2^53.
//!
//! `utoipa` describes them in the OpenAPI spec (`docs/api/openapi.json`).
//! It ignores serde's `rename_all_fields`, so fields inside the tagged
//! enums (`Problem`, `Reason`, `Event`) stay one word each; a test checks.

use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ottie_core::JobSnapshot;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::error::Failure;

/// One download, as listed by `GET /jobs` and sent in `changed` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
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
    /// Its place in the queue: among the waiting downloads of its priority,
    /// lower places start first. Only the order means something.
    #[ts(type = "number")]
    pub place: u64,
    /// Bytes per second; 0 = no limit of its own.
    #[ts(type = "number")]
    pub speed_limit: u64,
    /// Streaming mode: over several connections, the file comes in order,
    /// its last 1 MiB first, so it can play while downloading.
    pub streaming: bool,
    pub category: u32,
    /// Failed attempts since bytes last arrived.
    pub retries: u32,
    /// While `retrying`: when, in milliseconds since 1970 (UTC).
    #[ts(type = "number | null")]
    pub retry_at: Option<u64>,
    /// Downloads over several connections: each connection's range, as last
    /// saved (`progress` events bring them live). Empty for one connection.
    pub ranges: Vec<ByteRange>,
    /// Its file is moving to another category's folder. Until it's done, it
    /// can't be paused, resumed, cancelled, removed or moved again.
    pub moving: bool,
    /// Downloading, but the speed limits give it no bandwidth for now: other
    /// downloads take it all (urgent ones first).
    pub waiting_for_bandwidth: bool,
    /// While `scheduled`: when it joins the queue, in milliseconds since
    /// 1970 (UTC).
    #[ts(type = "number | null")]
    pub start_at: Option<u64>,
    /// When it pauses if it hasn't finished by then, the same way.
    #[ts(type = "number | null")]
    pub stop_at: Option<u64>,
}

/// A part of the file that one connection fetches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ByteRange {
    #[ts(type = "number")]
    pub start: u64,
    /// Exclusive.
    #[ts(type = "number")]
    pub end: u64,
    /// Bytes from `start` that have arrived.
    #[ts(type = "number")]
    pub done: u64,
}

impl From<ottie_core::Segment> for ByteRange {
    fn from(segment: ottie_core::Segment) -> Self {
        ByteRange {
            start: segment.start,
            end: segment.end,
            done: segment.done,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
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
    /// Waiting for its start time (`startAt`), then it joins the queue.
    Scheduled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Priority {
    Low,
    Normal,
    High,
    Urgent,
}

/// Body of `POST /jobs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
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
    /// Streaming mode. Default: on for videos and music if the settings'
    /// `streamMedia` is, by the link's extension, then by the file's real
    /// name once the server answers.
    #[serde(default)]
    #[ts(optional)]
    pub streaming: Option<bool>,
    /// Wait until then (`scheduled`), in milliseconds since 1970 (UTC).
    /// Default, or a time already past: queued at once.
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub start_at: Option<u64>,
    /// Pause then if it hasn't finished, the same way. After `startAt`.
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub stop_at: Option<u64>,
}

/// Answer to `POST /jobs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Added {
    pub id: String,
}

/// Body of `POST /jobs/batch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AddJobs {
    /// Added in this order, each as `POST /jobs` would.
    pub jobs: Vec<AddJob>,
}

/// Answer to `POST /jobs/batch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AddedJobs {
    /// One per job sent, in the same order: its new ID, or null if it was
    /// refused.
    pub ids: Vec<Option<String>>,
    /// Why each refused one was, by its place in `jobs`.
    pub refused: Vec<RefusedJob>,
}

/// A job of `POST /jobs/batch` that wasn't added, and why: the error
/// `POST /jobs` would have answered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RefusedJob {
    /// Its place in `jobs`, from 0.
    pub index: u32,
    #[serde(flatten)]
    pub error: ApiError,
}

/// Body of `POST /jobs/preview`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PreviewLinks {
    /// Any text: one link per line, an email, a web page. A sequence in a
    /// link (`[001-120]`, `[a-z]`, `{small,large}`) gives each link it holds.
    pub text: String,
    /// The folder they would be saved in, if not their categories': then
    /// the folder decides their category.
    #[serde(default)]
    #[ts(optional)]
    pub dest_dir: Option<String>,
}

/// Answer to `POST /jobs/preview`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Preview {
    /// In the order found, each once.
    pub links: Vec<FoundLink>,
}

/// Something in the text that looks like a link (`scheme://…`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum FoundLink {
    /// A link that can be added.
    Link {
        /// As it would be added (normalized: `HTTPS://Example.com` is
        /// `https://example.com/`).
        url: String,
        /// The file name the link suggests; the server may give another.
        name: String,
        /// The category it would get without one chosen.
        matched: Matched,
        /// A download of this same link is already on the list.
        known: bool,
    },
    /// One that can't: `refused` says why (`invalidUrl` or
    /// `unsupportedScheme`).
    Invalid { text: String, refused: ApiError },
}

/// Answer to `POST /jobs/{id}/stream`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StreamLink {
    /// A path on the daemon (`/api/v1/stream/<ticket>/<name>`) that serves
    /// the file to anyone who has it, without the token, with `Range`
    /// support. Valid for 12 hours after its last use.
    pub url: String,
}

/// A category: where its downloads go and which files it takes. As listed
/// by `GET /categories` and sent in `categoryChanged` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AddedCategory {
    pub id: u32,
}

/// Body of `POST /categories/match`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
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

/// A saved view: a name, which downloads it shows and how they're laid out.
/// Listed by `GET /views` and sent in `viewChanged` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct View {
    pub id: u32,
    pub name: String,
    pub filters: ViewFilters,
    pub arrangement: Arrangement,
}

/// Which downloads a view shows: those that pass every filter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ViewFilters {
    /// Null: any state. An empty list shows nothing. `connecting` counts as
    /// `downloading` (the daemon saves it so).
    pub states: Option<Vec<JobState>>,
    /// Category IDs; null: any category.
    pub categories: Option<Vec<u32>>,
    /// Part of the file name, in any case; empty: any name.
    pub name_contains: String,
    pub added: AddedWithin,
}

/// How recently a download was added; "today" starts at local midnight
/// where the list is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum AddedWithin {
    Any,
    Today,
    /// The past 7 days.
    Week,
}

/// How a list of downloads is laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Arrangement {
    pub sort: SortKey,
    pub direction: SortDirection,
    /// Grouped by status, each group sorted.
    pub grouped: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SortKey {
    /// By ID, which is by creation time.
    Added,
    Name,
    Size,
    Progress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[ts(export)]
pub enum SortDirection {
    #[serde(rename = "asc")]
    Ascending,
    #[serde(rename = "desc")]
    Descending,
}

/// Body of `POST /views` and `PUT /views/{id}`: everything about the view
/// (a PUT replaces it all).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ViewBody {
    pub name: String,
    pub filters: ViewFilters,
    pub arrangement: Arrangement,
}

/// Answer to `POST /views`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AddedView {
    pub id: u32,
}

/// Body of `PATCH /jobs/{id}`: the job's own settings to change; a field
/// left out stays as it is.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct JobChange {
    /// Bytes per second; 0 = no limit of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub speed_limit: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub priority: Option<Priority>,
    /// Streaming mode. Once set, the file type no longer decides it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub streaming: Option<bool>,
    /// Wait until then, in milliseconds since 1970 (UTC): a queued, paused,
    /// failed or retrying download becomes `scheduled`; a running one stops
    /// first. `null`: no start time (a scheduled download joins the queue
    /// now, as it does for a time already past).
    #[serde(default, with = "present", skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number | null")]
    #[schema(value_type = Option<u64>)]
    pub start_at: Option<Option<u64>>,
    /// Pause then, wherever it is; `null`: no stop time. After the start time.
    #[serde(default, with = "present", skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number | null")]
    #[schema(value_type = Option<u64>)]
    pub stop_at: Option<Option<u64>>,
}

/// A field that's there, even as `null` (`Some(None)`), as opposed to one
/// left out (`None`, through `#[serde(default)]`).
mod present {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub(super) fn deserialize<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        Option::<T>::deserialize(deserializer).map(Some)
    }

    /// `Some(None)` as `null`; `None` is skipped (`skip_serializing_if`).
    pub(super) fn serialize<S, T>(
        value: &Option<Option<T>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        value.serialize(serializer)
    }
}

impl From<JobChange> for ottie_core::JobChange {
    fn from(change: JobChange) -> Self {
        ottie_core::JobChange {
            speed_limit: change.speed_limit,
            priority: change.priority.map(Into::into),
            streaming: change.streaming,
            start_at: change.start_at.map(|at| at.map(from_millis)),
            stop_at: change.stop_at.map(|at| at.map(from_millis)),
        }
    }
}

/// A time on the wire: milliseconds since 1970 (UTC).
pub(crate) fn millis(at: SystemTime) -> u64 {
    at.duration_since(UNIX_EPOCH).map_or(0, |since| {
        u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
    })
}

pub(crate) fn from_millis(millis: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(millis)
}

/// Body of `POST /jobs/reorder`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ReorderJobs {
    /// The downloads to move. Those that aren't queued, scheduled or paused stay as they are.
    pub ids: Vec<String>,
    pub to: Reorder,
}

/// Where to move downloads in the queue, among the waiting downloads of
/// their priority: `"up"`, `"down"`, `"top"`, `"bottom"`, or next to a
/// download, `{ "before": id }` or `{ "after": id }` (where it was dropped).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Reorder {
    /// One place earlier.
    Up,
    /// One place later.
    Down,
    /// Before all the others.
    Top,
    /// After all the others.
    Bottom,
    /// Just before this download. Only those of its priority move.
    Before(String),
    /// Just after this download.
    After(String),
}

impl TryFrom<Reorder> for ottie_core::Reorder {
    type Error = ottie_core::Error;

    fn try_from(to: Reorder) -> Result<Self, Self::Error> {
        Ok(match to {
            Reorder::Up => ottie_core::Reorder::Up,
            Reorder::Down => ottie_core::Reorder::Down,
            Reorder::Top => ottie_core::Reorder::Top,
            Reorder::Bottom => ottie_core::Reorder::Bottom,
            Reorder::Before(id) => ottie_core::Reorder::Before(id.parse()?),
            Reorder::After(id) => ottie_core::Reorder::After(id.parse()?),
        })
    }
}

/// Body of `POST /jobs/{id}/category`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MoveJob {
    pub category: u32,
}

/// What applies to the whole daemon, as `GET /settings` answers it and
/// `settingsChanged` events send it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Settings {
    /// The first-start setup (starter categories) was done or skipped;
    /// until then, front ends offer it.
    pub setup_done: bool,
    /// How many downloads run at once; the others wait. At least 1.
    pub max_concurrent: u32,
    /// Bytes per second for all downloads together; 0 = no limit.
    #[ts(type = "number")]
    pub global_limit: u64,
    /// A download that keeps failing with errors that may go away (a dropped
    /// connection, a busy server) fails after this many retries in a row.
    /// 0 = never retry.
    pub max_retries: u32,
    /// Retries a download makes before letting a waiting one go first.
    pub retries_before_yield: u32,
    /// Wait before the first retry, in milliseconds; each next one waits
    /// twice as long, up to a minute.
    #[ts(type = "number")]
    pub first_retry_wait_ms: u64,
    /// The most links added at once (`POST /jobs/batch`) or looked at
    /// (`POST /jobs/preview`).
    pub max_batch: u32,
    /// New video and music downloads start in streaming mode, unless the
    /// one adding them says otherwise.
    pub stream_media: bool,
    /// The look shared by the devices that choose to follow it; null until
    /// one shares its own.
    pub appearance: Option<Appearance>,
    /// The language those devices show, as a tag like `en` or `pt-BR`; null
    /// until one shares its own. A device without it keeps its own.
    pub language: Option<String>,
}

/// Body of `PATCH /settings`: only the fields given change, all at once.
/// A value out of its range (see `crates/api/src/settings.rs`) refuses the
/// whole change with `outOfRange`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SettingsChange {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub setup_done: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub max_concurrent: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub global_limit: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub max_retries: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub retries_before_yield: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub first_retry_wait_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub max_batch: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub stream_media: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub appearance: Option<Appearance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub language: Option<String>,
}

/// How the front ends look: light, dark or as the system says, and which
/// pair of themes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Appearance {
    pub mode: AppearanceMode,
    pub palette: Palette,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum AppearanceMode {
    System,
    Light,
    Dark,
}

/// A light theme and a dark one that go together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Palette {
    /// Seafoam and Lagoon.
    Sea,
    /// Pearl and Slate.
    Neutral,
}

/// Why a job failed or is retrying. Clients translate it by `code` and
/// fall back to the job's English `error` for codes they don't know.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ApiError {
    /// English: for logs, and for codes a client doesn't know.
    pub error: String,
    #[serde(flatten)]
    pub reason: Reason,
}

/// Why a request was refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
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
    /// A download's stop time must come after its start time.
    StopBeforeStart,
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
    /// A setting must be from `min` to `max`; `setting` is its field name
    /// (`maxConcurrent`…).
    OutOfRange {
        setting: String,
        #[ts(type = "number")]
        min: u64,
        #[ts(type = "number")]
        max: u64,
    },
    /// Its file is moving; try again once it's done.
    JobMoving {
        id: String,
    },
    /// The download can't resume, so moving it now would start it over.
    CannotMoveWhileRunning,
    /// The file couldn't move (disk full, no permission…); `detail` says why.
    MoveFailed {
        detail: String,
    },
    ViewNotFound {
        id: u32,
    },
    InvalidViewName,
    ViewNameTaken {
        name: String,
    },
    /// Not a language tag like `en` or `pt-BR`.
    InvalidLanguage {
        language: String,
    },
    /// A play link that doesn't exist or has expired.
    NoSuchLink,
    /// Played past what arrived, but the download failed or was cancelled.
    DownloadStopped {
        id: String,
    },
    /// The download started over while being played (its file changed).
    FileChanged {
        id: String,
    },
    /// More links at once than the setting `maxBatch` allows.
    TooManyLinks {
        #[ts(type = "number")]
        count: u64,
        #[ts(type = "number")]
        max: u64,
    },
    /// The daemon is shutting down.
    Stopping,
    /// A bug or an unexpected failure; `error` has the details.
    Internal,
}

/// A message on the `GET /events` WebSocket. Connect first, then fetch
/// `GET /jobs`, then apply events in order (insert-or-replace by `id`, or
/// drop on `removed`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
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
        /// Several connections: each one's range, with what has arrived so
        /// far. Empty for one connection.
        ranges: Vec<ByteRange>,
    },
    /// A job was removed (`DELETE /jobs/{id}`): drop it.
    Removed { id: String },
    /// A category was added or changed: insert-or-replace by `id`.
    CategoryChanged { category: Category },
    /// A category was removed; its jobs were moved to General first (and a
    /// `changed` event was sent for each).
    CategoryRemoved { id: u32 },
    /// The settings changed (from any client): all of them, as now.
    SettingsChanged { settings: Settings },
    /// A saved view was added or changed: insert-or-replace by `id`.
    ViewChanged { view: View },
    /// A saved view was removed.
    ViewRemoved { id: u32 },
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
            place: job.place,
            speed_limit: job.speed_limit,
            streaming: job.streaming,
            category: job.category.0,
            retries: job.retries,
            retry_at: job.retry_at.map(millis),
            ranges: job.segments.into_iter().map(ByteRange::from).collect(),
            moving: job.moving,
            waiting_for_bandwidth: job.waiting_for_bandwidth,
            start_at: job.start_at.map(millis),
            stop_at: job.stop_at.map(millis),
        }
    }
}

impl From<ottie_core::Matched> for Matched {
    fn from(matched: ottie_core::Matched) -> Self {
        Matched {
            category: matched.category.0,
            extension: matched.extension,
            by_folder: matched.by_folder,
        }
    }
}

impl From<ottie_core::LinkPreview> for FoundLink {
    fn from(preview: ottie_core::LinkPreview) -> Self {
        match preview {
            ottie_core::LinkPreview::Link {
                url,
                name,
                matched,
                known,
            } => FoundLink::Link {
                url: url.to_string(),
                name,
                matched: matched.into(),
                known,
            },
            ottie_core::LinkPreview::Invalid { text, error } => FoundLink::Invalid {
                text,
                refused: Failure::from(error).into(),
            },
        }
    }
}

impl From<ottie_core::Settings> for Settings {
    fn from(settings: ottie_core::Settings) -> Self {
        let retry = settings.retry;
        Settings {
            setup_done: settings.setup_done,
            max_concurrent: u32::try_from(settings.max_concurrent).unwrap_or(u32::MAX),
            global_limit: settings.global_limit,
            max_retries: retry.max_retries,
            retries_before_yield: retry.retries_before_yield,
            first_retry_wait_ms: u64::try_from(retry.first_wait.as_millis()).unwrap_or(u64::MAX),
            max_batch: u32::try_from(settings.max_batch).unwrap_or(u32::MAX),
            stream_media: settings.stream_media,
            appearance: settings.appearance.map(Appearance::from),
            language: settings.language,
        }
    }
}

impl From<SettingsChange> for ottie_core::SettingsChange {
    fn from(change: SettingsChange) -> Self {
        ottie_core::SettingsChange {
            global_limit: change.global_limit,
            max_concurrent: change
                .max_concurrent
                .map(|max| usize::try_from(max).unwrap_or(usize::MAX)),
            max_retries: change.max_retries,
            retries_before_yield: change.retries_before_yield,
            first_retry_wait: change.first_retry_wait_ms.map(Duration::from_millis),
            max_batch: change
                .max_batch
                .map(|max| usize::try_from(max).unwrap_or(usize::MAX)),
            stream_media: change.stream_media,
            setup_done: change.setup_done,
            appearance: change.appearance.map(ottie_core::Appearance::from),
            language: change.language,
        }
    }
}

impl From<ottie_core::Appearance> for Appearance {
    fn from(appearance: ottie_core::Appearance) -> Self {
        use ottie_core::{AppearanceMode as M, Palette as P};
        Appearance {
            mode: match appearance.mode {
                M::System => AppearanceMode::System,
                M::Light => AppearanceMode::Light,
                M::Dark => AppearanceMode::Dark,
            },
            palette: match appearance.palette {
                P::Sea => Palette::Sea,
                P::Neutral => Palette::Neutral,
            },
        }
    }
}

impl From<Appearance> for ottie_core::Appearance {
    fn from(appearance: Appearance) -> Self {
        use ottie_core::{AppearanceMode as M, Palette as P};
        ottie_core::Appearance {
            mode: match appearance.mode {
                AppearanceMode::System => M::System,
                AppearanceMode::Light => M::Light,
                AppearanceMode::Dark => M::Dark,
            },
            palette: match appearance.palette {
                Palette::Sea => P::Sea,
                Palette::Neutral => P::Neutral,
            },
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

impl From<ottie_core::View> for View {
    fn from(view: ottie_core::View) -> Self {
        use ottie_core::{AddedWithin as A, SortDirection as D, SortKey as K};
        let settings = view.settings;
        View {
            id: view.id.0,
            name: settings.name,
            filters: ViewFilters {
                states: settings
                    .states
                    .map(|states| states.into_iter().map(JobState::from).collect()),
                categories: settings
                    .categories
                    .map(|ids| ids.into_iter().map(|id| id.0).collect()),
                name_contains: settings.name_contains,
                added: match settings.added {
                    A::Any => AddedWithin::Any,
                    A::Today => AddedWithin::Today,
                    A::Week => AddedWithin::Week,
                },
            },
            arrangement: Arrangement {
                sort: match settings.sort {
                    K::Added => SortKey::Added,
                    K::Name => SortKey::Name,
                    K::Size => SortKey::Size,
                    K::Progress => SortKey::Progress,
                },
                direction: match settings.direction {
                    D::Ascending => SortDirection::Ascending,
                    D::Descending => SortDirection::Descending,
                },
                grouped: settings.grouped,
            },
        }
    }
}

impl From<ViewBody> for ottie_core::ViewSettings {
    fn from(body: ViewBody) -> Self {
        use ottie_core::{AddedWithin as A, CategoryId, SortDirection as D, SortKey as K};
        let ViewBody {
            name,
            filters,
            arrangement,
        } = body;
        ottie_core::ViewSettings {
            name,
            states: filters
                .states
                .map(|states| states.into_iter().map(Into::into).collect()),
            categories: filters
                .categories
                .map(|ids| ids.into_iter().map(CategoryId).collect()),
            name_contains: filters.name_contains,
            added: match filters.added {
                AddedWithin::Any => A::Any,
                AddedWithin::Today => A::Today,
                AddedWithin::Week => A::Week,
            },
            sort: match arrangement.sort {
                SortKey::Added => K::Added,
                SortKey::Name => K::Name,
                SortKey::Size => K::Size,
                SortKey::Progress => K::Progress,
            },
            direction: match arrangement.direction {
                SortDirection::Ascending => D::Ascending,
                SortDirection::Descending => D::Descending,
            },
            grouped: arrangement.grouped,
        }
    }
}

impl From<JobState> for ottie_core::JobState {
    fn from(state: JobState) -> Self {
        use ottie_core::JobState as S;
        match state {
            JobState::Queued => S::Queued,
            JobState::Connecting => S::Connecting,
            JobState::Downloading => S::Downloading,
            JobState::Paused => S::Paused,
            JobState::Completed => S::Completed,
            JobState::Failed => S::Failed,
            JobState::Cancelled => S::Cancelled,
            JobState::Retrying => S::Retrying,
            JobState::Scheduled => S::Scheduled,
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
            S::Scheduled => JobState::Scheduled,
        }
    }
}

impl From<Priority> for ottie_core::Priority {
    fn from(priority: Priority) -> Self {
        use ottie_core::Priority as P;
        match priority {
            Priority::Low => P::Low,
            Priority::Normal => P::Normal,
            Priority::High => P::High,
            Priority::Urgent => P::Urgent,
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

#[cfg(test)]
mod tests {
    use super::{ByteRange, Event, JobChange};

    #[test]
    fn a_time_left_out_is_not_a_time_taken_away() {
        let change = |json| serde_json::from_value::<JobChange>(json).unwrap();
        let left_out = change(serde_json::json!({ "priority": "high" }));
        assert_eq!((left_out.start_at, left_out.stop_at), (None, None));
        let taken_away = change(serde_json::json!({ "startAt": null, "stopAt": 5 }));
        assert_eq!(
            (taken_away.start_at, taken_away.stop_at),
            (Some(None), Some(Some(5)))
        );
        // And back: only what was given.
        let json = serde_json::to_value(&taken_away).unwrap();
        assert_eq!(json, serde_json::json!({ "startAt": null, "stopAt": 5 }));
    }

    #[test]
    fn progress_carries_each_connections_range() {
        let range = ByteRange::from(ottie_core::Segment {
            start: 100,
            end: 300,
            done: 50,
        });
        let event = Event::Progress {
            id: "2026.10.13-09.00.00-1".into(),
            downloaded: 50,
            total: Some(300),
            ranges: vec![range],
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "progress",
                "id": "2026.10.13-09.00.00-1",
                "downloaded": 50,
                "total": 300,
                "ranges": [{ "start": 100, "end": 300, "done": 50 }],
            })
        );
    }
}
