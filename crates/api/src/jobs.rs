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
use crate::types::{AddJob, Added, Job, JobChange, MoveJob};

/// `GET /jobs`: every job, oldest first.
pub(crate) async fn list(State(state): State<AppState>) -> Result<Json<Vec<Job>>, Failure> {
    let jobs = state.manager.list().await?;
    Ok(Json(jobs.into_iter().map(Job::from).collect()))
}

/// `POST /jobs`: queues a download. Answers 201 with its ID.
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

/// `POST /jobs/{id}/pause`: answers once the job has really stopped.
pub(crate) async fn pause(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, Failure> {
    state.manager.pause(id.parse::<JobId>()?).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /jobs/{id}/resume`: puts a paused or failed job back in the queue.
pub(crate) async fn resume(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, Failure> {
    state.manager.resume(id.parse::<JobId>()?).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /jobs/{id}/cancel`: stops the job for good and deletes its partial file.
pub(crate) async fn cancel(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, Failure> {
    state.manager.cancel(id.parse::<JobId>()?).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /jobs/{id}`: takes the job off the list, in any state. Stops it
/// first if it's running (answering once it has) and deletes its partial
/// file; a completed file stays on disk.
pub(crate) async fn remove(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, Failure> {
    state.manager.remove(id.parse::<JobId>()?).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `PATCH /jobs/{id}`: changes the job's own speed limit and priority (the
/// fields given), in any state; they take effect immediately, even while it
/// runs. Every client hears of it through one `changed` event, none if
/// nothing changed.
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

/// `POST /jobs/{id}/category`: moves the job to another category, as the
/// user's choice (never re-matched), and its file to that category's folder
/// (docs/decisions/0028). A running job stops, moves and carries on. Answers
/// once the file has moved, which between drives takes as long as a copy.
/// A category without a folder leaves the file where it is.
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
