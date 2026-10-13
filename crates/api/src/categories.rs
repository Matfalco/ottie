//! `/categories`: list, add, change, remove, and which one a link matches.

use std::path::PathBuf;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use ottie_core::{CategoryId, CategorySettings, Url};

use crate::AppState;
use crate::error::Failure;
use crate::types::{AddedCategory, ApiError, Category, CategoryBody, MatchLink, Matched, Reason};

/// Every category by ID; General (1) first.
#[utoipa::path(
    get,
    path = "/categories",
    operation_id = "listCategories",
    tag = "categories",
    responses((status = OK, description = "Every category", body = [Category]))
)]
pub(crate) async fn list(State(state): State<AppState>) -> Result<Json<Vec<Category>>, Failure> {
    let categories = state.manager.categories().await?;
    Ok(Json(categories.into_iter().map(Category::from).collect()))
}

/// Adds a category.
#[utoipa::path(
    post,
    path = "/categories",
    operation_id = "addCategory",
    tag = "categories",
    request_body = CategoryBody,
    responses(
        (status = CREATED, description = "Added", body = AddedCategory),
        (status = BAD_REQUEST, description = "`invalidCategoryName`, `invalidExtension`, `relativeFolder` or `nonUtf8Path`", body = ApiError),
        (status = CONFLICT, description = "`categoryNameTaken` or `extensionTaken`", body = ApiError),
        (status = UNPROCESSABLE_ENTITY, description = "`invalidBody`", body = ApiError),
    )
)]
pub(crate) async fn add(
    State(state): State<AppState>,
    body: Result<Json<CategoryBody>, JsonRejection>,
) -> Result<(StatusCode, Json<AddedCategory>), Failure> {
    let Json(body) = body?;
    let id = state.manager.add_category(settings(body)?).await?;
    Ok((StatusCode::CREATED, Json(AddedCategory { id: id.0 })))
}

/// Replaces every setting of a category. A new folder applies to downloads
/// added from now on.
#[utoipa::path(
    put,
    path = "/categories/{id}",
    operation_id = "updateCategory",
    tag = "categories",
    params(("id" = u32, Path, description = "Category ID (General is 1)")),
    request_body = CategoryBody,
    responses(
        (status = NO_CONTENT, description = "Replaced"),
        (status = BAD_REQUEST, description = "`invalidCategoryName`, `invalidExtension`, `relativeFolder` or `nonUtf8Path`", body = ApiError),
        (status = NOT_FOUND, description = "`categoryNotFound`", body = ApiError),
        (status = CONFLICT, description = "`categoryNameTaken` or `extensionTaken`", body = ApiError),
        (status = UNPROCESSABLE_ENTITY, description = "`invalidBody`", body = ApiError),
    )
)]
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

/// Removes a category. Its downloads move to General first (a `changed`
/// event for each); their files stay where they are.
#[utoipa::path(
    delete,
    path = "/categories/{id}",
    operation_id = "removeCategory",
    tag = "categories",
    params(("id" = u32, Path, description = "Category ID (General is 1)")),
    responses(
        (status = NO_CONTENT, description = "Removed"),
        (status = NOT_FOUND, description = "`categoryNotFound`", body = ApiError),
        (status = CONFLICT, description = "`cannotRemoveBuiltIn` (General and Other)", body = ApiError),
    )
)]
pub(crate) async fn remove(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> Result<StatusCode, Failure> {
    state.manager.remove_category(CategoryId(id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// The category a link would get if added without one (in that folder, if
/// `destDir` is given). A POST, so links, which may carry tokens, never sit
/// in an address or a log.
#[utoipa::path(
    post,
    path = "/categories/match",
    operation_id = "matchCategory",
    tag = "categories",
    request_body = MatchLink,
    responses(
        (status = OK, description = "The category it would get", body = Matched),
        (status = BAD_REQUEST, description = "`invalidUrl` or `relativeFolder`", body = ApiError),
        (status = UNPROCESSABLE_ENTITY, description = "`invalidBody`", body = ApiError),
    )
)]
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
