//! Turning engine errors into HTTP answers: a status, a code clients can
//! translate, and the English text (`{"error": "…", "code": "…", …}`).

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ottie_core::Error;

use crate::types::{ApiError, Reason};

/// What a handler returns when it can't do what was asked.
#[derive(Debug)]
pub(crate) struct Failure {
    status: StatusCode,
    reason: Reason,
    message: String,
}

impl Failure {
    pub(crate) fn new(status: StatusCode, reason: Reason, message: impl Into<String>) -> Self {
        Failure {
            status,
            reason,
            message: message.into(),
        }
    }

    pub(crate) fn unauthorized() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            Reason::Unauthorized,
            "missing or wrong API token",
        )
    }

    pub(crate) fn no_such_endpoint() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            Reason::NoSuchEndpoint,
            "no such API endpoint",
        )
    }
}

impl From<Error> for Failure {
    fn from(err: Error) -> Self {
        use StatusCode as S;
        let (status, reason) = match &err {
            Error::JobNotFound(id) => (S::NOT_FOUND, Reason::JobNotFound { id: id.to_string() }),
            Error::CategoryNotFound(id) => (S::NOT_FOUND, Reason::CategoryNotFound { id: id.0 }),
            Error::InvalidTransition { from, to } => (
                S::CONFLICT,
                Reason::InvalidTransition {
                    from: (*from).into(),
                    to: (*to).into(),
                },
            ),
            Error::CategoryNameTaken(name) => (
                S::CONFLICT,
                Reason::CategoryNameTaken { name: name.clone() },
            ),
            Error::ExtensionTaken {
                extension,
                category,
            } => (
                S::CONFLICT,
                Reason::ExtensionTaken {
                    extension: extension.clone(),
                    category: category.clone(),
                },
            ),
            Error::CannotRemoveBuiltIn(category) => (
                S::CONFLICT,
                Reason::CannotRemoveBuiltIn {
                    category: category.clone(),
                },
            ),
            Error::UnsupportedScheme(scheme) => (
                S::BAD_REQUEST,
                Reason::UnsupportedScheme {
                    scheme: scheme.clone(),
                },
            ),
            Error::InvalidJobId(id) => (S::BAD_REQUEST, Reason::InvalidJobId { id: id.clone() }),
            Error::NonUtf8Path(_) => (S::BAD_REQUEST, Reason::NonUtf8Path),
            Error::NoFolder(category) => (
                S::BAD_REQUEST,
                Reason::NoFolder {
                    category: category.clone(),
                },
            ),
            Error::InvalidCategoryName => (S::BAD_REQUEST, Reason::InvalidCategoryName),
            Error::InvalidExtension(extension) => (
                S::BAD_REQUEST,
                Reason::InvalidExtension {
                    extension: extension.clone(),
                },
            ),
            Error::ManagerStopped => (S::SERVICE_UNAVAILABLE, Reason::Stopping),
            _ => {
                // Engine messages never contain URLs or credentials.
                tracing::error!(error = %err, "request failed");
                (S::INTERNAL_SERVER_ERROR, Reason::Internal)
            }
        };
        Failure::new(status, reason, err.to_string())
    }
}

/// A body that isn't the JSON we expect.
impl From<JsonRejection> for Failure {
    fn from(rejection: JsonRejection) -> Self {
        Failure::new(
            rejection.status(),
            Reason::InvalidBody,
            rejection.body_text(),
        )
    }
}

impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        let body = ApiError {
            error: self.message,
            reason: self.reason,
        };
        (self.status, Json(body)).into_response()
    }
}
