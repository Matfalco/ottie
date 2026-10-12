use std::path::PathBuf;

use reqwest::Url;

use crate::error::Error;
use crate::filename;

/// Everything needed to start or resume one download. This is the state that
/// will be persisted, so a download can resume after a restart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Download {
    /// The URL the user gave. Resuming requests it again (rather than the URL
    /// it redirected to) because redirect targets are often short-lived signed links.
    pub url: Url,
    pub dest_dir: PathBuf,
    /// Final file name, without `.part.ottie`. Chosen when the first response arrives.
    pub file_name: Option<String>,
    /// Size in bytes, if the server told us.
    pub total: Option<u64>,
    /// Bytes written to the `.part.ottie` file.
    pub downloaded: u64,
    /// Identifies the exact version of the remote file we've been downloading.
    pub validator: Option<Validator>,
    /// Whether a resume can continue where it stopped. If not, it restarts from zero.
    pub resumable: bool,
    /// Byte ranges fetched over separate connections, sorted by `start`.
    /// Empty for a single-connection download, whose `.part.ottie` length is
    /// then the truth for how much we have. When not empty, the `.part.ottie`
    /// file has its full size from the start and these ranges are the truth.
    pub segments: Vec<Segment>,
}

/// One range of a multi-connection download.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    pub start: u64,
    /// Exclusive. Can move down while downloading, when an idle connection
    /// takes over the second half of this range.
    pub end: u64,
    /// Bytes from `start` that are flushed to disk.
    pub done: u64,
}

impl Segment {
    pub fn is_complete(&self) -> bool {
        self.start + self.done >= self.end
    }
}

impl Download {
    pub fn new(url: Url, dest_dir: impl Into<PathBuf>) -> Result<Self, Error> {
        check_scheme(&url)?;
        Ok(Self {
            url,
            dest_dir: dest_dir.into(),
            file_name: None,
            total: None,
            downloaded: 0,
            validator: None,
            resumable: false,
            segments: Vec::new(),
        })
    }

    pub fn part_path(&self) -> Option<PathBuf> {
        self.file_name
            .as_deref()
            .map(|name| self.dest_dir.join(filename::part_name(name)))
    }
}

/// Header value proving the remote file hasn't changed: a strong `ETag`, or
/// failing that `Last-Modified`. Sent back in `If-Range` when resuming.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Validator {
    ETag(String),
    LastModified(String),
}

impl Validator {
    pub fn header_value(&self) -> &str {
        match self {
            Validator::ETag(v) | Validator::LastModified(v) => v,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The server answered and the `.part.ottie` file is ready. Sent once per
    /// run, before any `Progress`, so the caller can save everything needed to
    /// resume even if the process dies before the first byte arrives.
    Started {
        file_name: String,
        total: Option<u64>,
        validator: Option<Validator>,
        resumable: bool,
        /// Where this run continues from (0 unless resuming).
        downloaded: u64,
        /// The ranges this run will fetch (empty for a single connection).
        segments: Vec<Segment>,
    },
    Progress {
        downloaded: u64,
        total: Option<u64>,
    },
    /// Multi-connection only: the ranges as now flushed to disk. Safe to save;
    /// a resume from these never trusts bytes that aren't really there.
    Checkpoint {
        segments: Vec<Segment>,
    },
    /// The bytes already on disk were thrown away and the download started over.
    Restarted(RestartReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartReason {
    /// The server gave no way to check the file is unchanged, or refuses ranges.
    NotResumable,
    /// The remote file is a different version from the one we started.
    RemoteChanged,
    /// The server ignored our `Range` header and sent the whole file.
    RangesIgnored,
    /// The `.part.ottie` file was deleted.
    PartFileMissing,
    /// The `.part.ottie` file no longer has the size it was created with.
    PartFileDamaged,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Completed {
        path: PathBuf,
    },
    /// Stopped by the cancellation token. Running again resumes.
    Paused,
}

pub(crate) fn check_scheme(url: &Url) -> Result<(), Error> {
    match url.scheme() {
        "http" | "https" => Ok(()),
        other => Err(Error::UnsupportedScheme(other.to_owned())),
    }
}
