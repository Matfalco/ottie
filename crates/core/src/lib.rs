//! Download engine. No HTTP server, no UI.

mod download;
mod error;
mod filename;
mod http;

pub use download::{Download, Event, Outcome, RestartReason, Validator};
pub use error::Error;
pub use http::{build_client, run};
