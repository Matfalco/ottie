//! Turning engine errors into HTTP answers.

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ottie_core::Error;

use crate::types::ApiError;

/// What a handler returns when it can't do what was asked: a status, and a
/// message sent as `{"error": "…"}`.
#[derive(Debug)]
pub(crate) struct Failure {
    status: StatusCode,
    message: String,
}

impl Failure {
    pub(crate) fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Failure {
            status,
            message: message.into(),
        }
    }

    pub(crate) fn bad_request(message: impl Into<String>) -> Self {
        Failure {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    pub(crate) fn unauthorized() -> Self {
        Failure {
            status: StatusCode::UNAUTHORIZED,
            message: "missing or wrong API token".to_owned(),
        }
    }
}

impl From<Error> for Failure {
    fn from(err: Error) -> Self {
        use Error::*;
        let status = match &err {
            JobNotFound(_) | CategoryNotFound(_) => StatusCode::NOT_FOUND,
            InvalidTransition { .. }
            | CategoryNameTaken(_)
            | ExtensionTaken { .. }
            | CannotRemoveGeneral => StatusCode::CONFLICT,
            UnsupportedScheme(_) | InvalidJobId(_) | NonUtf8Path(_) | NoFolder(_)
            | InvalidCategoryName | InvalidExtension(_) => StatusCode::BAD_REQUEST,
            ManagerStopped => StatusCode::SERVICE_UNAVAILABLE,
            _ => {
                // Engine messages never contain URLs or credentials.
                tracing::error!(error = %err, "request failed");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };
        Failure {
            status,
            message: err.to_string(),
        }
    }
}

/// A body that isn't the JSON we expect.
impl From<JsonRejection> for Failure {
    fn from(rejection: JsonRejection) -> Self {
        Failure {
            status: rejection.status(),
            message: rejection.body_text(),
        }
    }
}

impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        let body = ApiError {
            error: self.message,
        };
        (self.status, Json(body)).into_response()
    }
}
