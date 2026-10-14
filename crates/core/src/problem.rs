//! Why a job failed or is retrying, in a form clients can translate: a kind
//! plus its details. `Display` gives the English wording. See
//! docs/decisions/0016.

use std::fmt;

use reqwest::StatusCode;

use crate::error::{Error, with_causes};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// Connecting or reading failed (DNS, refused, reset, TLS…). `detail` is
    /// the network library's English text.
    Network { detail: String },
    /// Too many redirects, or a redirect to something other than http(s).
    Redirect { detail: String },
    /// The server answered with a status we can't use (404, 503…).
    HttpStatus { status: u16 },
    /// No data for this many seconds while waiting for it.
    Stalled { seconds: u64 },
    /// The connection closed before the end of the file.
    Incomplete { received: u64, expected: u64 },
    /// The file kept changing on the server while downloading.
    RemoteChanging,
    /// Writing the file failed (disk full, no permission…).
    Disk { detail: String },
    /// Anything else; `detail` is the English message.
    Other { detail: String },
}

impl From<&Error> for Problem {
    fn from(err: &Error) -> Self {
        match err {
            Error::Http(e) if e.is_redirect() => Problem::Redirect {
                detail: with_causes(e),
            },
            Error::Http(e) => Problem::Network {
                detail: with_causes(e),
            },
            Error::Status { status, .. } => Problem::HttpStatus {
                status: status.as_u16(),
            },
            Error::Stalled(wait) => Problem::Stalled {
                // Rounded up: a 300 ms timeout is "1 s", not "0 s".
                seconds: wait.as_secs() + u64::from(wait.subsec_nanos() > 0),
            },
            Error::Incomplete { received, expected } => Problem::Incomplete {
                received: *received,
                expected: *expected,
            },
            Error::RemoteUnstable => Problem::RemoteChanging,
            Error::Io(e) => Problem::Disk {
                detail: e.to_string(),
            },
            other => Problem::Other {
                detail: other.to_string(),
            },
        }
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Problem::Network { detail } => write!(f, "HTTP request failed: {detail}"),
            Problem::Redirect { detail } => write!(f, "bad redirect: {detail}"),
            Problem::HttpStatus { status } => match StatusCode::from_u16(*status) {
                Ok(status) => write!(f, "server answered {status}"),
                Err(_) => write!(f, "server answered {status}"),
            },
            Problem::Stalled { seconds } => write!(f, "no data from the server for {seconds} s"),
            Problem::Incomplete { received, expected } => {
                write!(f, "download stopped after {received} of {expected} bytes")
            }
            Problem::RemoteChanging => {
                write!(f, "the file kept changing on the server while downloading")
            }
            Problem::Disk { detail } => write!(f, "file error: {detail}"),
            Problem::Other { detail } => f.write_str(detail),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn errors_become_problems_with_the_same_wording() {
        let status = Error::Status {
            status: StatusCode::NOT_FOUND,
            retry_after: None,
        };
        let problem = Problem::from(&status);
        assert_eq!(problem, Problem::HttpStatus { status: 404 });
        assert_eq!(problem.to_string(), status.to_string());

        let stalled = Error::Stalled(Duration::from_secs(60));
        assert_eq!(Problem::from(&stalled), Problem::Stalled { seconds: 60 });
        assert_eq!(Problem::from(&stalled).to_string(), stalled.to_string());

        let disk = Error::Io(std::io::Error::other("disk full"));
        assert_eq!(Problem::from(&disk).to_string(), disk.to_string());
        let other = Error::ManagerStopped;
        assert_eq!(Problem::from(&other).to_string(), other.to_string());
    }
}
