//! Single-connection HTTP(S) transfer with pause/resume.

use std::io;
use std::path::Path;

use reqwest::header::{
    ACCEPT_ENCODING, ACCEPT_RANGES, CONTENT_RANGE, ETAG, HeaderMap, IF_RANGE, LAST_MODIFIED, RANGE,
};
use reqwest::{Client, Response, StatusCode, Url, redirect};
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio_util::sync::CancellationToken;

use crate::download::{Download, Event, Outcome, RestartReason, Validator, check_scheme};
use crate::error::Error;
use crate::filename;

const MAX_REDIRECTS: usize = 10;

/// HTTP client with our redirect rules: at most 10 hops, http/https only.
pub fn build_client() -> Result<Client, Error> {
    let policy = redirect::Policy::custom(|attempt| {
        if attempt.previous().len() >= MAX_REDIRECTS {
            attempt.error("too many redirects")
        } else if check_scheme(attempt.url()).is_err() {
            let scheme = attempt.url().scheme().to_owned();
            attempt.error(format!("redirect to unsupported scheme `{scheme}`"))
        } else {
            attempt.follow()
        }
    });
    Ok(Client::builder()
        .redirect(policy)
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION")
        ))
        .build()?)
}

/// Downloads into `<name>.part.ottie` until the file is complete or `cancel` fires,
/// then renames it. Call again with the same `Download` to resume.
///
/// `dl` is kept up to date as bytes arrive, so after an error or a pause it
/// describes what's on disk.
pub async fn run(
    client: &Client,
    dl: &mut Download,
    cancel: &CancellationToken,
    mut on_event: impl FnMut(Event),
) -> Result<Outcome, Error> {
    let opened = open(client, dl, &mut on_event).await?;
    if let Some((response, file)) = opened.body {
        let mut writer = BufWriter::new(file);
        let streamed = stream_body(response, &mut writer, dl, cancel, &mut on_event).await;
        // Flush even after an error, so the .part.ottie length matches `dl.downloaded`.
        let flushed = flush(&mut writer).await;
        drop(writer); // Windows can't rename a file that's still open.
        if streamed? == Stop::Cancelled {
            flushed?;
            return Ok(Outcome::Paused);
        }
        flushed?;
    }
    complete(dl, &opened.name).await
}

struct Opened {
    name: String,
    /// `None` when the file was already fully downloaded.
    body: Option<(Response, File)>,
}

/// Sends the request and prepares the `.part.ottie` file: a fresh start, a resume,
/// or a restart from zero when resuming isn't safe.
async fn open(
    client: &Client,
    dl: &mut Download,
    on_event: &mut impl FnMut(Event),
) -> Result<Opened, Error> {
    let (Some(name), Some(part)) = (dl.file_name.clone(), dl.part_path()) else {
        let response = expect_full(get(client, &dl.url, None).await?)?;
        let suggested = filename::from_url(response.url());
        let (name, file) = filename::reserve(&dl.dest_dir, &suggested).await?;
        tracing::debug!(url = %redact(&dl.url), %name, "starting download");
        dl.file_name = Some(name.clone());
        learn(dl, response.headers(), response.content_length());
        return Ok(Opened {
            name,
            body: Some((response, file)),
        });
    };

    let on_disk = match fs::metadata(&part).await {
        Ok(meta) => Some(meta.len()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    let reason = match (on_disk, dl.validator.clone()) {
        (None, _) => RestartReason::PartFileMissing,
        (Some(offset), Some(validator)) if dl.resumable => {
            match try_resume(client, dl, &part, offset, &validator).await? {
                Resume::Continue(body) => return Ok(Opened { name, body }),
                Resume::Restart(response, reason) => {
                    // The server already sent the whole file: use it.
                    restarting(dl, reason, on_event);
                    learn(dl, response.headers(), response.content_length());
                    let file = File::create(&part).await?;
                    return Ok(Opened {
                        name,
                        body: Some((response, file)),
                    });
                }
                Resume::Refused(reason) => reason,
            }
        }
        (Some(_), _) => RestartReason::NotResumable,
    };

    restarting(dl, reason, on_event);
    let response = expect_full(get(client, &dl.url, None).await?)?;
    learn(dl, response.headers(), response.content_length());
    let file = File::create(&part).await?;
    Ok(Opened {
        name,
        body: Some((response, file)),
    })
}

enum Resume {
    /// Continue appending to the `.part.ottie` file (or nothing left to fetch).
    Continue(Option<(Response, File)>),
    /// The server answered with the full file instead of the rest.
    Restart(Response, RestartReason),
    /// Resuming isn't possible; a new full request is needed.
    Refused(RestartReason),
}

async fn try_resume(
    client: &Client,
    dl: &mut Download,
    part: &Path,
    offset: u64,
    validator: &Validator,
) -> Result<Resume, Error> {
    let response = get(client, &dl.url, Some((offset, validator))).await?;
    match response.status() {
        StatusCode::PARTIAL_CONTENT => {
            if !content_range_matches(response.headers(), offset, dl.total) {
                return Ok(Resume::Refused(RestartReason::RemoteChanged));
            }
            let file = OpenOptions::new().append(true).open(part).await?;
            dl.downloaded = offset;
            Ok(Resume::Continue(Some((response, file))))
        }
        // A 200 means either the If-Range validator didn't match (the file
        // changed), or the server doesn't do ranges. The new validator tells which.
        StatusCode::OK => {
            let reason = if validator_from(response.headers()).as_ref() == Some(validator) {
                RestartReason::RangesIgnored
            } else {
                RestartReason::RemoteChanged
            };
            Ok(Resume::Restart(response, reason))
        }
        StatusCode::RANGE_NOT_SATISFIABLE if dl.total == Some(offset) => {
            dl.downloaded = offset;
            Ok(Resume::Continue(None))
        }
        StatusCode::RANGE_NOT_SATISFIABLE => Ok(Resume::Refused(RestartReason::RemoteChanged)),
        status => Err(Error::Status(status)),
    }
}

fn restarting(dl: &mut Download, reason: RestartReason, on_event: &mut impl FnMut(Event)) {
    tracing::debug!(url = %redact(&dl.url), ?reason, "restarting download from zero");
    dl.downloaded = 0;
    on_event(Event::Restarted(reason));
}

/// Records what a full (200) response tells us about the remote file.
fn learn(dl: &mut Download, headers: &HeaderMap, content_length: Option<u64>) {
    let refuses_ranges = headers
        .get(ACCEPT_RANGES)
        .is_some_and(|v| v.as_bytes().eq_ignore_ascii_case(b"none"));
    dl.total = content_length;
    dl.validator = validator_from(headers);
    dl.resumable = dl.validator.is_some() && !refuses_ranges;
    dl.downloaded = 0;
}

#[derive(Debug, PartialEq, Eq)]
enum Stop {
    Finished,
    Cancelled,
}

async fn stream_body(
    mut response: Response,
    writer: &mut BufWriter<File>,
    dl: &mut Download,
    cancel: &CancellationToken,
    on_event: &mut impl FnMut(Event),
) -> Result<Stop, Error> {
    loop {
        // Wait for whichever comes first: the next chunk or a pause request.
        // `biased` checks the pause first, so a fast server can't delay it.
        let chunk = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Ok(Stop::Cancelled),
            chunk = response.chunk() => chunk?,
        };
        let Some(bytes) = chunk else {
            return Ok(Stop::Finished);
        };
        writer.write_all(&bytes).await?;
        dl.downloaded += bytes.len() as u64;
        on_event(Event::Progress {
            downloaded: dl.downloaded,
            total: dl.total,
        });
    }
}

async fn flush(writer: &mut BufWriter<File>) -> io::Result<()> {
    writer.flush().await?;
    writer.get_ref().sync_data().await
}

async fn complete(dl: &mut Download, name: &str) -> Result<Outcome, Error> {
    match dl.total {
        Some(expected) if expected != dl.downloaded => {
            return Err(Error::Incomplete {
                received: dl.downloaded,
                expected,
            });
        }
        _ => dl.total = Some(dl.downloaded),
    }
    let part = dl.dest_dir.join(filename::part_name(name));
    let final_name = filename::finalize(&dl.dest_dir, name, &part).await?;
    let path = dl.dest_dir.join(&final_name);
    dl.file_name = Some(final_name);
    Ok(Outcome::Completed { path })
}

async fn get(
    client: &Client,
    url: &Url,
    range: Option<(u64, &Validator)>,
) -> Result<Response, Error> {
    check_scheme(url)?;
    // Ask for the raw bytes: no compression, so ranges map to file offsets.
    let mut request = client.get(url.clone()).header(ACCEPT_ENCODING, "identity");
    if let Some((offset, validator)) = range {
        request = request
            .header(RANGE, format!("bytes={offset}-"))
            .header(IF_RANGE, validator.header_value());
    }
    Ok(request.send().await?)
}

fn expect_full(response: Response) -> Result<Response, Error> {
    match response.status() {
        StatusCode::OK => Ok(response),
        status => Err(Error::Status(status)),
    }
}

/// Weak ETags (`W/"…"`) can't be used with `If-Range`, so fall back to `Last-Modified`.
fn validator_from(headers: &HeaderMap) -> Option<Validator> {
    let text = |name| headers.get(name).and_then(|v| v.to_str().ok());
    match text(ETAG) {
        Some(etag) if !etag.starts_with("W/") => Some(Validator::ETag(etag.to_owned())),
        _ => text(LAST_MODIFIED).map(|lm| Validator::LastModified(lm.to_owned())),
    }
}

/// A 206 must start exactly where we asked, run to the end, and (if we know
/// the size) describe a file of the same size.
fn content_range_matches(headers: &HeaderMap, offset: u64, total: Option<u64>) -> bool {
    let Some((start, end, size)) = headers
        .get(CONTENT_RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_content_range)
    else {
        return false;
    };
    let size_ok = match (total, size) {
        (Some(total), Some(size)) => total == size && end + 1 == size,
        _ => true,
    };
    start == offset && size_ok
}

/// Parses `bytes <start>-<end>/<size or *>`.
fn parse_content_range(value: &str) -> Option<(u64, u64, Option<u64>)> {
    let (range, size) = value.strip_prefix("bytes ")?.split_once('/')?;
    let (start, end) = range.split_once('-')?;
    let size = match size {
        "*" => None,
        n => Some(n.parse().ok()?),
    };
    Some((start.parse().ok()?, end.parse().ok()?, size))
}

/// URL without credentials, query or fragment, safe to log.
fn redact(url: &Url) -> String {
    let mut url = url.clone();
    url.set_query(None);
    url.set_fragment(None);
    // These only fail for URLs that can't have credentials anyway.
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_content_range() {
        assert_eq!(
            parse_content_range("bytes 10-99/100"),
            Some((10, 99, Some(100)))
        );
        assert_eq!(parse_content_range("bytes 10-99/*"), Some((10, 99, None)));
        assert_eq!(parse_content_range("bytes */100"), None);
        assert_eq!(parse_content_range("items 1-2/3"), None);
    }

    #[test]
    fn redacts_secrets() {
        let url = Url::parse("https://user:pw@example.com/f.zip?token=abc#x").unwrap();
        assert_eq!(redact(&url), "https://example.com/f.zip");
    }
}
