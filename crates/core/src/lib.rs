// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
mod problem;
mod segmented;
mod store;
mod throttle;

pub use backend::{Backend, HttpBackend};
pub use category::{BuiltIn, Category, CategoryId, CategorySettings, Matched};
pub use download::{Download, Event, Outcome, Placement, RestartReason, Segment, Validator};
pub use error::Error;
pub use filename::PART_SUFFIX;
pub use http::{HttpConfig, build_client, run};
pub use id::JobId;
pub use job::{JobEvent, JobSnapshot, JobState, NewJob, Priority};
pub use manager::Manager;
pub use problem::Problem;
pub use store::{RetryPolicy, Settings, Store};
pub use throttle::Throttle;

/// The URL type every download uses, so clients don't need `reqwest` themselves.
pub use reqwest::Url;
