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
mod playback;
mod problem;
mod segmented;
mod settings;
mod store;
mod stream;
mod throttle;
mod view;

pub use backend::{Backend, HttpBackend};
pub use category::{BuiltIn, Category, CategoryId, CategorySettings, Matched};
pub use download::{Download, Event, Outcome, Placement, RestartReason, Segment, Validator};
pub use error::Error;
pub use filename::PART_SUFFIX;
pub use http::{HttpConfig, build_client, run};
pub use id::JobId;
pub use job::{JobChange, JobEvent, JobSnapshot, JobState, NewJob, Priority, Reorder};
pub use manager::Manager;
pub use playback::Playback;
pub use problem::Problem;
pub use settings::{Appearance, AppearanceMode, Palette, RetryPolicy, Settings, SettingsChange};
pub use store::Store;
pub use stream::{Opened, Reader};
pub use throttle::Throttle;
pub use view::{AddedWithin, SortDirection, SortKey, View, ViewId, ViewSettings};

/// The URL type every download uses, so clients don't need `reqwest` themselves.
pub use reqwest::Url;
