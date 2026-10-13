//! `/settings`: what applies to the whole daemon.

use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;

use crate::AppState;
use crate::error::Failure;
use crate::types::{Settings, SettingsChange};

/// `GET /settings`.
pub(crate) async fn get(State(state): State<AppState>) -> Result<Json<Settings>, Failure> {
    Ok(Json(state.manager.settings().await?.into()))
}

/// `PATCH /settings`: changes the fields given, each saved at once.
pub(crate) async fn change(
    State(state): State<AppState>,
    body: Result<Json<SettingsChange>, JsonRejection>,
) -> Result<StatusCode, Failure> {
    let Json(change) = body?;
    if let Some(done) = change.setup_done {
        state.manager.set_setup_done(done).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}
