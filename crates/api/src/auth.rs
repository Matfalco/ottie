//! The API token: every request must carry it, this computer's or a
//! device's (access.rs).

use std::fmt;
use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::header::{AUTHORIZATION, SEC_WEBSOCKET_PROTOCOL};
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::AppState;
use crate::error::Failure;
use crate::types::Reason;

/// The WebSocket protocol the events stream speaks. Clients offer it along
/// with their token; the server answers with this one.
pub const EVENTS_PROTOCOL: &str = "ottie.v1";

/// Browsers can't set headers on a WebSocket, so the token travels as an
/// offered protocol: `ottie.token.<token>`. (Not in the URL, where it could
/// end up in logs.)
pub const TOKEN_PROTOCOL_PREFIX: &str = "ottie.token.";

/// The secret every request must carry. `Debug` doesn't show it.
#[derive(Clone)]
pub struct Token(Arc<str>);

impl Token {
    pub fn new(secret: impl Into<String>) -> Self {
        Token(secret.into().into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Compares in constant time, so response timing doesn't reveal how
    /// much of a guess was right.
    pub(crate) fn matches(&self, candidate: &str) -> bool {
        let (a, b) = (self.0.as_bytes(), candidate.as_bytes());
        a.len() == b.len() && a.iter().zip(b).fold(0, |diff, (x, y)| diff | (x ^ y)) == 0
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Token(…)")
    }
}

/// Lets the request through only if it carries a token, and only if that
/// token may do it (a read-only device only reads). Handlers learn who it is
/// from the `Caller` it leaves in the request.
pub(crate) async fn require_token(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let headers = request.headers();
    let bearer = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    let offered = headers
        .get_all(SEC_WEBSOCKET_PROTOCOL)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .filter_map(|protocol| protocol.trim().strip_prefix(TOKEN_PROTOCOL_PREFIX));
    let candidates: Vec<&str> = bearer.into_iter().chain(offered).collect();
    let Some(caller) = state.access.identify(&candidates) else {
        return Failure::unauthorized().into_response();
    };
    if caller.read_only() && !only_reads(request.method(), request.uri().path()) {
        return Failure::new(
            StatusCode::FORBIDDEN,
            Reason::ReadOnly,
            "this device's token is read-only",
        )
        .into_response();
    }
    request.extensions_mut().insert(caller);
    next.run(request).await
}

/// What a read-only token may do: every `GET`, and the `POST`s that change
/// nothing (the links in a text, a link's category, a play link).
fn only_reads(method: &Method, path: &str) -> bool {
    if matches!(*method, Method::GET | Method::HEAD) {
        return true;
    }
    *method == Method::POST
        && (path == "/jobs/preview"
            || path == "/categories/match"
            || path
                .strip_prefix("/jobs/")
                .and_then(|rest| rest.strip_suffix("/stream"))
                .is_some_and(|id| !id.contains('/')))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_exact_token_matches() {
        let token = Token::new("s3cret");
        assert!(token.matches("s3cret"));
        assert!(!token.matches("s3cre"));
        assert!(!token.matches("s3cret!"));
        assert!(!token.matches(""));
        assert_eq!(format!("{token:?}"), "Token(…)");
    }

    #[test]
    fn read_only_tokens_only_read() {
        assert!(only_reads(&Method::GET, "/jobs"));
        assert!(only_reads(&Method::POST, "/jobs/preview"));
        assert!(only_reads(
            &Method::POST,
            "/jobs/2026.10.16-10.00.00-1/stream"
        ));
        assert!(!only_reads(&Method::POST, "/jobs"));
        assert!(!only_reads(&Method::POST, "/jobs/batch"));
        assert!(!only_reads(&Method::POST, "/jobs/1/pause"));
        assert!(!only_reads(&Method::POST, "/jobs/a/b/stream"));
        assert!(!only_reads(&Method::PATCH, "/settings"));
        assert!(!only_reads(&Method::DELETE, "/jobs/1"));
    }
}
