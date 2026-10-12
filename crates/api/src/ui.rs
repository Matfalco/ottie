//! The web UI. SvelteKit builds it into `ui-dist/` (`pnpm --filter ui build`),
//! and those files are compiled into the binary, so the daemon serves its UI
//! with nothing else to install. In debug builds they're read from disk
//! instead, so rebuilding the UI doesn't need a Rust rebuild.
//!
//! The UI's files are public; only the API needs the token.

use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};

use axum::body::Body;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderValue, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

use crate::error::Failure;

#[derive(RustEmbed)]
#[folder = "ui-dist/"]
// A Rust build without a UI build still works; it then explains what to do.
#[allow_missing = true]
struct Embedded;

/// Where the UI's files come from.
#[derive(Debug, Clone, Default)]
pub enum UiSource {
    /// Compiled into the binary.
    #[default]
    Embedded,
    /// A folder on disk (`ottie serve --ui-dir`), for a UI built elsewhere.
    Folder(PathBuf),
}

impl UiSource {
    async fn load(&self, path: &str) -> Option<Cow<'static, [u8]>> {
        match self {
            UiSource::Embedded => Embedded::get(path).map(|file| file.data),
            UiSource::Folder(root) => {
                let file = inside(root, path)?;
                tokio::fs::read(file).await.ok().map(Cow::Owned)
            }
        }
    }
}

/// `root` joined with `path`, or `None` if `path` could leave `root`
/// (`..`, an absolute path, a Windows drive).
fn inside(root: &Path, path: &str) -> Option<PathBuf> {
    let mut full = root.to_owned();
    for part in Path::new(path).components() {
        match part {
            Component::Normal(name) => full.push(name),
            _ => return None,
        }
    }
    Some(full)
}

/// Every request outside `/api/v1`.
pub(crate) async fn serve(source: &UiSource, method: Method, uri: &Uri) -> Response {
    let path = uri.path();
    if path == "/api" || path.starts_with("/api/") {
        return Failure::no_such_endpoint().into_response();
    }
    if method != Method::GET && method != Method::HEAD {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }
    let path = path.trim_start_matches('/');
    let wanted = if path.is_empty() { "index.html" } else { path };
    if let Some(data) = source.load(wanted).await {
        return file(wanted, data);
    }
    // A page of the app rather than a file (no extension): the app's
    // index.html shows the right one itself.
    let is_page = !wanted.rsplit('/').next().unwrap_or("").contains('.');
    match source.load("index.html").await {
        Some(index) if is_page => file("index.html", index),
        Some(_) => StatusCode::NOT_FOUND.into_response(),
        None => not_built(),
    }
}

fn file(path: &str, data: Cow<'static, [u8]>) -> Response {
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    // Files under _app/immutable/ have a hash in their name, so they never
    // change; everything else (index.html) must be checked every time.
    let cache = if path.starts_with("_app/immutable/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    let mut response = Response::new(Body::from(data));
    let headers = response.headers_mut();
    if let Ok(mime) = HeaderValue::from_str(mime.as_ref()) {
        headers.insert(CONTENT_TYPE, mime);
    }
    headers.insert(CACHE_CONTROL, HeaderValue::from_static(cache));
    response
}

fn not_built() -> Response {
    let page = "<!doctype html><meta charset=utf-8><title>Ottie</title>\
        <p>This build of Ottie has no web UI. Build it with \
        <code>pnpm --filter ui build</code>, then rebuild Ottie \
        (or start it with <code>--ui-dir</code>). The API works without it.";
    let mut response = (StatusCode::SERVICE_UNAVAILABLE, page).into_response();
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_cannot_leave_the_folder() {
        let root = Path::new("/ui");
        assert_eq!(
            inside(root, "_app/x.js"),
            Some(PathBuf::from("/ui/_app/x.js"))
        );
        for evil in ["../secret", "a/../../b", "/etc/passwd", "./x"] {
            assert_eq!(inside(root, evil), None, "{evil}");
        }
        // A drive letter only means something on Windows.
        #[cfg(windows)]
        assert_eq!(inside(root, "C:/Windows"), None);
    }
}
