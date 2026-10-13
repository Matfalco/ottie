//! `/jobs`: list, add, pause, resume, cancel, remove, change a job's own
//! settings, change category.

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use ottie_core::{CategoryId, JobId, NewJob};

use crate::AppState;
use crate::categories::{absolute_folder, parse_url};
use crate::error::Failure;
use crate::types::{AddJob, Added, ApiError, Job, JobChange, MoveJob};

/// Every job, oldest first.
#[utoipa::path(
    get,
    path = "/jobs",
    operation_id = "listJobs",
    tag = "jobs",
    responses((status = OK, description = "Every job, oldest first", body = [Job]))
)]
pub(crate) async fn list(State(state): State<AppState>) -> Result<Json<Vec<Job>>, Failure> {
    let jobs = state.manager.list().await?;
    Ok(Json(jobs.into_iter().map(Job::from).collect()))
}

/// Queues a download. It starts as soon as a slot is free.
#[utoipa::path(
    post,
    path = "/jobs",
    operation_id = "addJob",
    tag = "jobs",
    request_body = AddJob,
    responses(
        (status = CREATED, description = "Queued", body = Added),
        (status = BAD_REQUEST, description = "`invalidUrl`, `unsupportedScheme`, `relativeFolder`, `nonUtf8Path` or `noFolder`", body = ApiError),
        (status = NOT_FOUND, description = "`categoryNotFound`", body = ApiError),
        (status = UNPROCESSABLE_ENTITY, description = "`invalidBody`", body = ApiError),
    )
)]
pub(crate) async fn add(
    State(state): State<AppState>,
    body: Result<Json<AddJob>, JsonRejection>,
) -> Result<(StatusCode, Json<Added>), Failure> {
    let Json(body) = body?;
    let mut job = NewJob::new(parse_url(&body.url)?);
    if let Some(dir) = body.dest_dir {
        job = job.in_dir(absolute_folder(dir)?);
    }
    if let Some(category) = body.category {
        job = job.in_category(CategoryId(category));
    }
    let id = state.manager.add(job).await?;
    Ok((StatusCode::CREATED, Json(Added { id: id.to_string() })))
}

/// Pauses a download. Answers once it has really stopped and its file is flushed.
#[utoipa::path(
    post,
    path = "/jobs/{id}/pause",
    operation_id = "pauseJob",
    tag = "jobs",
    params(("id" = String, Path, description = "Job ID, like `2026.10.13-14.28.59-1`")),
    responses(
        (status = NO_CONTENT, description = "Paused"),
        (status = BAD_REQUEST, description = "`invalidJobId`", body = ApiError),
        (status = NOT_FOUND, description = "`jobNotFound`", body = ApiError),
        (status = CONFLICT, description = "`invalidTransition` (a finished download…) or `jobMoving`", body = ApiError),
    )
)]
pub(crate) async fn pause(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, Failure> {
    state.manager.pause(id.parse::<JobId>()?).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Puts a paused or failed download back in the queue.
#[utoipa::path(
    post,
    path = "/jobs/{id}/resume",
    operation_id = "resumeJob",
    tag = "jobs",
    params(("id" = String, Path, description = "Job ID, like `2026.10.13-14.28.59-1`")),
    responses(
        (status = NO_CONTENT, description = "Queued again"),
        (status = BAD_REQUEST, description = "`invalidJobId`", body = ApiError),
        (status = NOT_FOUND, description = "`jobNotFound`", body = ApiError),
        (status = CONFLICT, description = "`invalidTransition` or `jobMoving`", body = ApiError),
    )
)]
pub(crate) async fn resume(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, Failure> {
    state.manager.resume(id.parse::<JobId>()?).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Stops a download for good and deletes its partial file. It stays on the
/// list as `cancelled`.
#[utoipa::path(
    post,
    path = "/jobs/{id}/cancel",
    operation_id = "cancelJob",
    tag = "jobs",
    params(("id" = String, Path, description = "Job ID, like `2026.10.13-14.28.59-1`")),
    responses(
        (status = NO_CONTENT, description = "Cancelled"),
        (status = BAD_REQUEST, description = "`invalidJobId`", body = ApiError),
        (status = NOT_FOUND, description = "`jobNotFound`", body = ApiError),
        (status = CONFLICT, description = "`invalidTransition` or `jobMoving`", body = ApiError),
    )
)]
pub(crate) async fn cancel(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, Failure> {
    state.manager.cancel(id.parse::<JobId>()?).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Takes a download off the list, in any state. Stops it first if it's
/// running (answering once it has) and deletes its partial file; a completed
/// file stays on disk.
#[utoipa::path(
    delete,
    path = "/jobs/{id}",
    operation_id = "removeJob",
    tag = "jobs",
    params(("id" = String, Path, description = "Job ID, like `2026.10.13-14.28.59-1`")),
    responses(
        (status = NO_CONTENT, description = "Removed"),
        (status = BAD_REQUEST, description = "`invalidJobId`", body = ApiError),
        (status = NOT_FOUND, description = "`jobNotFound`", body = ApiError),
        (status = CONFLICT, description = "`jobMoving`", body = ApiError),
    )
)]
pub(crate) async fn remove(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, Failure> {
    state.manager.remove(id.parse::<JobId>()?).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Changes a download's own speed limit and priority (the fields given), in
/// any state; they take effect at once, even while it runs. Every client
/// hears of it through one `changed` event, none if nothing changed.
#[utoipa::path(
    patch,
    path = "/jobs/{id}",
    operation_id = "changeJob",
    tag = "jobs",
    params(("id" = String, Path, description = "Job ID, like `2026.10.13-14.28.59-1`")),
    request_body = JobChange,
    responses(
        (status = NO_CONTENT, description = "Changed"),
        (status = BAD_REQUEST, description = "`invalidJobId`", body = ApiError),
        (status = NOT_FOUND, description = "`jobNotFound`", body = ApiError),
        (status = UNPROCESSABLE_ENTITY, description = "`invalidBody`", body = ApiError),
    )
)]
pub(crate) async fn change(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Result<Json<JobChange>, JsonRejection>,
) -> Result<StatusCode, Failure> {
    let Json(change) = body?;
    state
        .manager
        .change_job(id.parse::<JobId>()?, change.into())
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Moves a download to another category, as the user's choice (never
/// re-matched), and its file to that category's folder. A running download
/// stops, moves and carries on. Answers once the file has moved, which
/// between drives takes as long as a copy. A category without a folder
/// leaves the file where it is.
#[utoipa::path(
    post,
    path = "/jobs/{id}/category",
    operation_id = "moveJob",
    tag = "jobs",
    params(("id" = String, Path, description = "Job ID, like `2026.10.13-14.28.59-1`")),
    request_body = MoveJob,
    responses(
        (status = NO_CONTENT, description = "Moved"),
        (status = BAD_REQUEST, description = "`invalidJobId`", body = ApiError),
        (status = NOT_FOUND, description = "`jobNotFound` or `categoryNotFound`", body = ApiError),
        (status = CONFLICT, description = "`jobMoving`, or `cannotMoveWhileRunning` (it can't resume)", body = ApiError),
        (status = UNPROCESSABLE_ENTITY, description = "`invalidBody`", body = ApiError),
        (status = INTERNAL_SERVER_ERROR, description = "`moveFailed`: the file couldn't move (disk full…)", body = ApiError),
    )
)]
pub(crate) async fn set_category(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Result<Json<MoveJob>, JsonRejection>,
) -> Result<StatusCode, Failure> {
    let Json(body) = body?;
    state
        .manager
        .set_category(id.parse::<JobId>()?, CategoryId(body.category))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
