use std::path::PathBuf;

use reqwest::StatusCode;

use crate::category::CategoryId;
use crate::id::JobId;
use crate::job::JobState;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unsupported URL scheme `{0}` (only http and https are supported)")]
    UnsupportedScheme(String),

    /// Network or protocol failure. The URL is stripped so query strings never
    /// end up in logs or error messages.
    #[error("HTTP request failed: {0}")]
    Http(reqwest::Error),

    #[error("server answered {0}")]
    Status(StatusCode),

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

    #[error("the General category can't be removed")]
    CannotRemoveGeneral,

    #[error("no folder given, and category `{0}` has none")]
    NoFolder(String),
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Http(err.without_url())
    }
}
