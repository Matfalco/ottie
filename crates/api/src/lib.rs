// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Ottie's HTTP API: REST + WebSocket events over the download engine. Every
//! client (web UI, desktop and Android apps, browser extension, CLI) goes
//! through it. See docs/decisions/0014.
//!
//! Everything is under `/api/v1` and needs the token, as
//! `Authorization: Bearer <token>` or, for the events WebSocket (browsers
//! can't set headers there), as the WebSocket protocol `ottie.token.<token>`.

mod auth;
mod error;
mod events;
mod jobs;
mod types;
mod ui;

use axum::Router;
use axum::http::{Method, Uri};
use axum::middleware;
use axum::routing::{delete, get, post};
use ottie_core::Manager;
use tokio_util::sync::CancellationToken;

pub use auth::{EVENTS_PROTOCOL, TOKEN_PROTOCOL_PREFIX, Token};
pub use types::{AddJob, Added, ApiError, Event, Job, JobState, Priority, Problem, Reason};
pub use ui::UiSource;

/// Shared by every request handler.
#[derive(Clone)]
struct AppState {
    manager: Manager,
    token: Token,
    /// Cancelled when the daemon stops, so open WebSockets close.
    shutdown: CancellationToken,
}

/// The API under `/api/v1`, and the web UI everywhere else. Cancel
/// `shutdown` before stopping the server so event streams end.
pub fn router(manager: Manager, token: Token, shutdown: CancellationToken, ui: UiSource) -> Router {
    let state = AppState {
        manager,
        token,
        shutdown,
    };
    let api = Router::new()
        .route("/jobs", get(jobs::list).post(jobs::add))
        .route("/jobs/{id}", delete(jobs::remove))
        .route("/jobs/{id}/pause", post(jobs::pause))
        .route("/jobs/{id}/resume", post(jobs::resume))
        .route("/jobs/{id}/cancel", post(jobs::cancel))
        .route("/events", get(events::subscribe))
        .fallback(not_found)
        // Also covers the fallback, so unknown paths don't reveal what exists.
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_token,
        ))
        .with_state(state);
    Router::new()
        .nest("/api/v1", api)
        .fallback(move |method: Method, uri: Uri| async move { ui::serve(&ui, method, &uri).await })
}

async fn not_found() -> error::Failure {
    error::Failure::no_such_endpoint()
}
