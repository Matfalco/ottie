//! Download engine. No HTTP server, no UI.

mod backend;
mod category;
mod download;
mod error;
mod filename;
mod http;
mod id;
mod job;
mod manager;
mod segmented;
mod store;
mod throttle;

pub use backend::{Backend, HttpBackend};
pub use category::{Category, CategoryId, CategorySettings};
pub use download::{Download, Event, Outcome, Placement, RestartReason, Segment, Validator};
pub use error::Error;
pub use filename::PART_SUFFIX;
pub use http::{HttpConfig, build_client, run};
pub use id::JobId;
pub use job::{JobEvent, JobSnapshot, JobState, NewJob, Priority};
pub use manager::Manager;
pub use store::{RetryPolicy, Settings, Store};
pub use throttle::Throttle;
