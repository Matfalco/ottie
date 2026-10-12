//! Download engine. No HTTP server, no UI.

mod backend;
mod download;
mod error;
mod filename;
mod http;
mod id;
mod job;
mod manager;

pub use backend::{Backend, HttpBackend};
pub use download::{Download, Event, Outcome, RestartReason, Validator};
pub use error::Error;
pub use filename::PART_SUFFIX;
pub use http::{build_client, run};
pub use id::JobId;
pub use job::{JobEvent, JobSnapshot, JobState};
pub use manager::{Manager, ManagerConfig};
