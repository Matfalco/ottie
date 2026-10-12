use reqwest::StatusCode;

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
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Http(err.without_url())
    }
}
