//! `/jobs`: list, add, pause, resume, cancel, remove.

use std::path::PathBuf;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use ottie_core::{CategoryId, JobId, NewJob, Url};

use crate::AppState;
use crate::error::Failure;
use crate::types::{AddJob, Added, Job, Reason};

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
    let url = Url::parse(&body.url).map_err(|_| {
        Failure::new(
            StatusCode::BAD_REQUEST,
            Reason::InvalidUrl,
            "`url` is not a URL",
        )
    })?;
    let mut job = NewJob::new(url);
    if let Some(dir) = body.dest_dir {
        let dir = PathBuf::from(dir);
        // Relative to what? The daemon's working folder means nothing to a client.
        if !dir.is_absolute() {
            return Err(Failure::new(
                StatusCode::BAD_REQUEST,
                Reason::RelativeFolder,
                "`destDir` must be an absolute path",
            ));
        }
        job = job.in_dir(dir);
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
