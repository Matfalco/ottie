//! The API token: every request must carry it.

use std::fmt;
use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::header::{AUTHORIZATION, SEC_WEBSOCKET_PROTOCOL};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::AppState;
use crate::error::Failure;

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
    fn matches(&self, candidate: &str) -> bool {
        let (a, b) = (self.0.as_bytes(), candidate.as_bytes());
        a.len() == b.len() && a.iter().zip(b).fold(0, |diff, (x, y)| diff | (x ^ y)) == 0
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Token(…)")
    }
}

/// Lets the request through only if it carries the token.
pub(crate) async fn require_token(
    State(state): State<AppState>,
    request: Request,
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
    let mut candidates = bearer.into_iter().chain(offered);
    if candidates.any(|candidate| state.token.matches(candidate)) {
        next.run(request).await
    } else {
        Failure::unauthorized().into_response()
    }
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
}
