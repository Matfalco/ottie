//! `/views`: the saved views of the downloads list (docs/decisions/0026).

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use ottie_core::ViewId;

use crate::AppState;
use crate::error::Failure;
use crate::types::{AddedView, View, ViewBody};

/// `GET /views`: every saved view, oldest first.
pub(crate) async fn list(State(state): State<AppState>) -> Result<Json<Vec<View>>, Failure> {
    let views = state.manager.views().await?;
    Ok(Json(views.into_iter().map(View::from).collect()))
}

/// `POST /views`: answers 201 with the new ID.
pub(crate) async fn add(
    State(state): State<AppState>,
    body: Result<Json<ViewBody>, JsonRejection>,
) -> Result<(StatusCode, Json<AddedView>), Failure> {
    let Json(body) = body?;
    let id = state.manager.add_view(body.into()).await?;
    Ok((StatusCode::CREATED, Json(AddedView { id: id.0 })))
}

/// `PUT /views/{id}`: replaces the view, name included.
pub(crate) async fn update(
    State(state): State<AppState>,
    Path(id): Path<u32>,
    body: Result<Json<ViewBody>, JsonRejection>,
) -> Result<StatusCode, Failure> {
    let Json(body) = body?;
    state.manager.update_view(ViewId(id), body.into()).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /views/{id}`.
pub(crate) async fn remove(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> Result<StatusCode, Failure> {
    state.manager.remove_view(ViewId(id)).await?;
    Ok(StatusCode::NO_CONTENT)
}
