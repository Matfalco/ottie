//! `/categories`: list, add, change, remove, and which one a link matches.

use std::path::PathBuf;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use ottie_core::{CategoryId, CategorySettings, Url};

use crate::AppState;
use crate::error::Failure;
use crate::types::{AddedCategory, Category, CategoryBody, MatchLink, Matched, Reason};

/// `GET /categories`: every category by ID; General (1) first.
pub(crate) async fn list(State(state): State<AppState>) -> Result<Json<Vec<Category>>, Failure> {
    let categories = state.manager.categories().await?;
    Ok(Json(categories.into_iter().map(Category::from).collect()))
}

/// `POST /categories`: answers 201 with the new ID.
pub(crate) async fn add(
    State(state): State<AppState>,
    body: Result<Json<CategoryBody>, JsonRejection>,
) -> Result<(StatusCode, Json<AddedCategory>), Failure> {
    let Json(body) = body?;
    let id = state.manager.add_category(settings(body)?).await?;
    Ok((StatusCode::CREATED, Json(AddedCategory { id: id.0 })))
}

/// `PUT /categories/{id}`: replaces every setting. A new folder applies to
/// downloads added from now on.
pub(crate) async fn update(
    State(state): State<AppState>,
    Path(id): Path<u32>,
    body: Result<Json<CategoryBody>, JsonRejection>,
) -> Result<StatusCode, Failure> {
    let Json(body) = body?;
    state
        .manager
        .update_category(CategoryId(id), settings(body)?)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /categories/{id}`: its downloads move to General first (their
/// files stay where they are). General itself can't be removed.
pub(crate) async fn remove(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> Result<StatusCode, Failure> {
    state.manager.remove_category(CategoryId(id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /categories/match`: the category a link would get if added without
/// one (in that folder, if `destDir` is given). A POST, so links (which may carry tokens) never sit in an address.
pub(crate) async fn matching(
    State(state): State<AppState>,
    body: Result<Json<MatchLink>, JsonRejection>,
) -> Result<Json<Matched>, Failure> {
    let Json(body) = body?;
    let url = parse_url(&body.url)?;
    let dest_dir = body.dest_dir.map(absolute_folder).transpose()?;
    let matched = state.manager.match_category(url, dest_dir).await?;
    Ok(Json(Matched {
        category: matched.category.0,
        extension: matched.extension,
        by_folder: matched.by_folder,
    }))
}

pub(crate) fn parse_url(url: &str) -> Result<Url, Failure> {
    Url::parse(url).map_err(|_| {
        Failure::new(
            StatusCode::BAD_REQUEST,
            Reason::InvalidUrl,
            "`url` is not a URL",
        )
    })
}

/// An absolute folder, as the engine wants it.
pub(crate) fn absolute_folder(folder: String) -> Result<PathBuf, Failure> {
    let folder = PathBuf::from(folder);
    // Relative to what? The daemon's working folder means nothing to a client.
    if folder.is_absolute() {
        Ok(folder)
    } else {
        Err(Failure::new(
            StatusCode::BAD_REQUEST,
            Reason::RelativeFolder,
            "the folder must be an absolute path",
        ))
    }
}

fn settings(body: CategoryBody) -> Result<CategorySettings, Failure> {
    Ok(CategorySettings {
        name: body.name,
        folder: body.folder.map(absolute_folder).transpose()?,
        extensions: body.extensions,
        speed_limit: body.speed_limit,
        max_concurrent: usize::try_from(body.max_concurrent).unwrap_or(usize::MAX),
    })
}
