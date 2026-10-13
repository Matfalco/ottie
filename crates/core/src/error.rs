use std::path::PathBuf;
use std::time::Duration;

use reqwest::StatusCode;

use crate::category::CategoryId;
use crate::id::JobId;
use crate::job::JobState;
use crate::view::ViewId;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unsupported URL scheme `{0}` (only http and https are supported)")]
    UnsupportedScheme(String),

    /// Network or protocol failure. The URL is stripped so query strings never
    /// end up in logs or error messages.
    #[error("HTTP request failed: {0}")]
    Http(reqwest::Error),

    /// `retry_after` is the server's `Retry-After`, if it sent one in seconds.
    #[error("server answered {status}")]
    Status {
        status: StatusCode,
        retry_after: Option<Duration>,
    },

    #[error("no data from the server for {} s", .0.as_secs())]
    Stalled(Duration),

    #[error("download stopped after {received} of {expected} bytes")]
    Incomplete { received: u64, expected: u64 },

    #[error("the file kept changing on the server while downloading")]
    RemoteUnstable,

    #[error("file error: {0}")]
    Io(#[from] std::io::Error),

    #[error("no job with ID {0}")]
    JobNotFound(JobId),

    #[error("a {from:?} job can't become {to:?}")]
    InvalidTransition { from: JobState, to: JobState },

    #[error("`{0}` is not a job ID (expected yyyy.mm.dd-hh.mm.ss-N)")]
    InvalidJobId(String),

    #[error("the download manager has stopped")]
    ManagerStopped,

    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error(
        "the database was written by a newer Ottie (schema {found}, this version knows up to {supported})"
    )]
    DatabaseTooNew { found: usize, supported: usize },

    #[error("upgrading the database to schema {0} left references to missing rows")]
    MigrationBroken(usize),

    #[error("the database thread has stopped")]
    StoreStopped,

    #[error("folder path is not valid UTF-8: {}", .0.display())]
    NonUtf8Path(PathBuf),

    #[error("no category with ID {0}")]
    CategoryNotFound(CategoryId),

    #[error("a category name can't be empty")]
    InvalidCategoryName,

    #[error("there is already a category named `{0}`")]
    CategoryNameTaken(String),

    #[error("`{0}` is not a file extension")]
    InvalidExtension(String),

    #[error("files ending in .{extension} already go to category `{category}`")]
    ExtensionTaken { extension: String, category: String },

    #[error("no more category IDs are available")]
    TooManyCategories,

    #[error("the built-in category `{0}` can't be removed")]
    CannotRemoveBuiltIn(String),

    #[error("no folder given, and category `{0}` has none")]
    NoFolder(String),

    #[error("download {0} is being moved")]
    JobMoving(JobId),

    #[error("download {0} can't resume, so it can only move to another folder once it has stopped")]
    CannotMoveWhileRunning(JobId),

    #[error("could not move the file: {0}")]
    MoveFailed(#[source] std::io::Error),

    #[error("no view with ID {0}")]
    ViewNotFound(ViewId),

    #[error("a view name can't be empty")]
    InvalidViewName,

    #[error("there is already a view named `{0}`")]
    ViewNameTaken(String),

    #[error("no more view IDs are available")]
    TooManyViews,
}

impl Error {
    /// Whether trying again later may work: network trouble, a dropped or
    /// stalled connection, or a server that is busy or failing. Not: an
    /// answer like 404, a bad redirect, or a problem with our own disk.
    pub fn is_retryable(&self) -> bool {
        match self {
            Error::Http(err) => !err.is_redirect() && !err.is_builder(),
            Error::Status { status, .. } => {
                status.is_server_error()
                    || matches!(
                        *status,
                        StatusCode::REQUEST_TIMEOUT | StatusCode::TOO_MANY_REQUESTS
                    )
            }
            Error::Incomplete { .. } | Error::Stalled(_) => true,
            _ => false,
        }
    }

    /// How long the server asked us to wait before trying again, if it did.
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Status { retry_after, .. } => *retry_after,
            _ => None,
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Http(err.without_url())
    }
}
