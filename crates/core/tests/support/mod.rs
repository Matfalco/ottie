//! In-process HTTP server for engine tests, with fault injection.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use std::io;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::header::{
    ACCEPT_RANGES, CONTENT_LENGTH, CONTENT_RANGE, ETAG, IF_RANGE, LOCATION, RANGE,
};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::get;
use futures_util::Stream;
use reqwest::Url;

const CHUNK: usize = 16 * 1024;

/// What the server does. Change it between requests with [`TestServer::configure`].
pub struct Behavior {
    pub body: Bytes,
    pub etag: Option<String>,
    /// Honour `Range` requests. When false, always answer 200 with the whole file.
    pub ranges: bool,
    /// Pause before each 16 KiB chunk.
    pub chunk_delay: Option<Duration>,
    /// Pause this long instead before chunks at or after this file offset,
    /// so one part of the file is slow.
    pub slow_from: Option<(usize, Duration)>,
    /// Kill the connection after this many body bytes, once (the next request is normal).
    pub drop_after: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct RecordedRequest {
    pub range: Option<String>,
    pub if_range: Option<String>,
}

struct Shared {
    behavior: Behavior,
    requests: Vec<RecordedRequest>,
}

type AppState = Arc<Mutex<Shared>>;

pub struct TestServer {
    addr: SocketAddr,
    state: AppState,
}

impl TestServer {
    /// Serves `body` at `/files/<any name>`, with ETag `"v1"` and range support.
    pub async fn start(body: Vec<u8>) -> Self {
        let state = Arc::new(Mutex::new(Shared {
            behavior: Behavior {
                body: Bytes::from(body),
                etag: Some("\"v1\"".to_owned()),
                ranges: true,
                chunk_delay: None,
                slow_from: None,
                drop_after: None,
            },
            requests: Vec::new(),
        }));
        let app = Router::new()
            .route("/files/{name}", get(serve_file))
            .route("/redirect-to-ftp", get(redirect_to_ftp))
            .with_state(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self { addr, state }
    }

    pub fn url(&self, path: &str) -> Url {
        Url::parse(&format!("http://{}{path}", self.addr)).unwrap()
    }

    pub fn configure(&self, change: impl FnOnce(&mut Behavior)) {
        change(&mut self.state.lock().unwrap().behavior);
    }

    pub fn requests(&self) -> Vec<RecordedRequest> {
        self.state.lock().unwrap().requests.clone()
    }
}

async fn serve_file(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let text = |name| headers.get(name).map(|v| v.to_str().unwrap().to_owned());
    let range = text(RANGE);
    let if_range = text(IF_RANGE);
    let (body, etag, ranges, pacing, drop_after) = {
        let mut shared = state.lock().unwrap();
        shared.requests.push(RecordedRequest {
            range: range.clone(),
            if_range: if_range.clone(),
        });
        let b = &mut shared.behavior;
        let drop_after = b.drop_after.take();
        let pacing = Pacing {
            delay: b.chunk_delay,
            slow_from: b.slow_from,
        };
        (b.body.clone(), b.etag.clone(), b.ranges, pacing, drop_after)
    };

    let total = body.len();
    let validator_ok = if_range.is_none() || if_range == etag;
    let requested = match range.as_deref().and_then(|r| r.strip_prefix("bytes=")) {
        Some(spec) if ranges && validator_ok => {
            // "bytes=N-" or "bytes=N-M" (M inclusive).
            let (start, last) = spec.split_once('-').unwrap();
            let start: usize = start.parse().unwrap();
            let end = match last {
                "" => total,
                last => (last.parse::<usize>().unwrap() + 1).min(total),
            };
            if start >= total {
                return Response::builder()
                    .status(StatusCode::RANGE_NOT_SATISFIABLE)
                    .header(CONTENT_RANGE, format!("bytes */{total}"))
                    .body(Body::empty())
                    .unwrap();
            }
            Some((start, end))
        }
        _ => None,
    };
    let (start, end) = requested.unwrap_or((0, total));

    let mut response = Response::builder()
        .header(ACCEPT_RANGES, if ranges { "bytes" } else { "none" })
        .header(CONTENT_LENGTH, end - start);
    if let Some(etag) = &etag {
        response = response.header(ETAG, etag);
    }
    if requested.is_some() {
        response = response
            .status(StatusCode::PARTIAL_CONTENT)
            .header(CONTENT_RANGE, format!("bytes {start}-{}/{total}", end - 1));
    }
    let data = body.slice(start..end);
    response
        .body(Body::from_stream(chunked(data, start, pacing, drop_after)))
        .unwrap()
}

#[derive(Clone, Copy)]
struct Pacing {
    delay: Option<Duration>,
    slow_from: Option<(usize, Duration)>,
}

async fn redirect_to_ftp() -> Response {
    Response::builder()
        .status(StatusCode::FOUND)
        .header(LOCATION, "ftp://127.0.0.1/file.bin")
        .body(Body::empty())
        .unwrap()
}

/// Sends `data` (which starts at file offset `offset`) in 16 KiB chunks,
/// optionally slowly, optionally failing midway.
fn chunked(
    data: Bytes,
    offset: usize,
    pacing: Pacing,
    drop_after: Option<usize>,
) -> impl Stream<Item = io::Result<Bytes>> {
    futures_util::stream::unfold((data, 0usize), move |(mut data, sent)| async move {
        if data.is_empty() {
            return None;
        }
        let mut len = CHUNK.min(data.len());
        if let Some(limit) = drop_after {
            if sent >= limit {
                let err = io::Error::other("injected connection drop");
                return Some((Err(err), (Bytes::new(), sent)));
            }
            len = len.min(limit - sent);
        }
        let delay = match pacing.slow_from {
            Some((from, slow)) if offset + sent >= from => Some(slow),
            _ => pacing.delay,
        };
        if let Some(delay) = delay {
            tokio::time::sleep(delay).await;
        }
        let chunk = data.split_to(len);
        Some((Ok(chunk), (data, sent + len)))
    })
}

/// Deterministic, non-repeating test content.
pub fn test_data(len: usize, seed: u64) -> Vec<u8> {
    let mut x = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    (0..len)
        .map(|_| {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (x >> 56) as u8
        })
        .collect()
}
