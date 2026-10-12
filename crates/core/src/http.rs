//! HTTP(S) transfers with pause/resume: a single connection, or several in
//! parallel (see `segmented.rs`) when the server supports ranges and the file
//! is big enough.

use std::io;
use std::path::Path;

use reqwest::header::{
    ACCEPT_ENCODING, ACCEPT_RANGES, CONTENT_DISPOSITION, CONTENT_RANGE, ETAG, HeaderMap, IF_RANGE,
    LAST_MODIFIED, RANGE,
};
use reqwest::{Client, Response, StatusCode, Url, redirect};
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio_util::sync::CancellationToken;

use crate::download::{Download, Event, Outcome, RestartReason, Validator, check_scheme};
use crate::error::Error;
use crate::filename;
use crate::segmented::{self, Segmented};
use crate::throttle::Throttle;

const MAX_REDIRECTS: usize = 10;

/// How HTTP downloads are split across connections.
#[derive(Debug, Clone)]
pub struct HttpConfig {
    /// Connections per download. 1 disables splitting.
    pub connections: usize,
    /// Never split a range into pieces smaller than this many bytes.
    pub min_split: u64,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            connections: 4,
            min_split: 1024 * 1024,
        }
    }
}

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
    config: &HttpConfig,
    dl: &mut Download,
    throttle: &Throttle,
    cancel: &CancellationToken,
    mut on_event: impl FnMut(Event),
) -> Result<Outcome, Error> {
    let env = Env {
        client,
        config,
        throttle,
        cancel,
    };
    let mut opened = match (dl.segments.is_empty(), dl.file_name.clone(), dl.part_path()) {
        (false, Some(name), Some(part)) => {
            match resume_segmented(&env, dl, &name, &part, &mut on_event).await? {
                Resumed::Stopped(outcome) => return Ok(outcome),
                Resumed::Restart(reason) => {
                    restarting(dl, reason, &mut on_event);
                    fresh(client, dl, name, &part).await?
                }
            }
        }
        _ => {
            dl.segments.clear();
            open(client, dl, &mut on_event).await?
        }
    };

    // Loops at most twice: once more if the file changes on the server
    // in the middle of a multi-connection download.
    let mut restarted_mid_run = false;
    loop {
        let Opened { name, body } = opened;
        let Some((response, file)) = body else {
            started(dl, &name, &mut on_event);
            return complete(dl, &name).await;
        };
        let Some(total) = splittable(config, dl, &response) else {
            started(dl, &name, &mut on_event);
            return single(&env, response, file, dl, &name, &mut on_event).await;
        };

        // Full size up front, so every connection can write at its offset
        // and a later resume can tell from the length that the file is intact.
        file.set_len(total).await?;
        drop(file);
        dl.segments = segmented::plan(total, config);
        started(dl, &name, &mut on_event);
        let part = dl.dest_dir.join(filename::part_name(&name));
        let first = Some(response);
        match segmented::run(&env, dl, &part, first, &mut on_event).await? {
            Segmented::Finished => return complete(dl, &name).await,
            Segmented::Cancelled => return Ok(Outcome::Paused),
            Segmented::Restart(reason) if !restarted_mid_run => {
                restarted_mid_run = true;
                restarting(dl, reason, &mut on_event);
                opened = fresh(client, dl, name, &part).await?;
            }
            Segmented::Restart(_) => return Err(Error::RemoteUnstable),
        }
    }
}

/// What every step of one run shares. The `'a` says these are borrowed from
/// the caller of [`run`] and only live as long as that call.
pub(crate) struct Env<'a> {
    pub client: &'a Client,
    pub config: &'a HttpConfig,
    pub throttle: &'a Throttle,
    pub cancel: &'a CancellationToken,
}

struct Opened {
    name: String,
    /// `None` when the file was already fully downloaded.
    body: Option<(Response, File)>,
}

/// Sends the request and prepares a single-connection `.part.ottie` file: a
/// fresh start, a resume, or a restart from zero when resuming isn't safe.
async fn open(
    client: &Client,
    dl: &mut Download,
    on_event: &mut impl FnMut(Event),
) -> Result<Opened, Error> {
    let (Some(name), Some(part)) = (dl.file_name.clone(), dl.part_path()) else {
        let response = get_from_start(client, &dl.url).await?;
        let suggested = suggested_name(&response);
        if let Some(placement) = dl.placement.take()
            && let Some(folder) = placement.folder_for(&suggested)
        {
            dl.dest_dir = folder.to_owned();
        }
        let (name, file) = filename::reserve(&dl.dest_dir, &suggested).await?;
        tracing::debug!(url = %redact(&dl.url), %name, "starting download");
        dl.file_name = Some(name.clone());
        learn(dl, &response);
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
                    learn(dl, &response);
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
    fresh(client, dl, name, &part).await
}

/// The name from the server's `Content-Disposition`, else the last part of
/// the URL it answered from (after redirects). Sanitized either way.
fn suggested_name(response: &Response) -> String {
    response
        .headers()
        .get(CONTENT_DISPOSITION)
        .and_then(|value| filename::from_content_disposition(value.as_bytes()))
        .unwrap_or_else(|| filename::from_url(response.url()))
}

/// Requests the file from the start again and empties the `.part.ottie` file.
async fn fresh(
    client: &Client,
    dl: &mut Download,
    name: String,
    part: &Path,
) -> Result<Opened, Error> {
    let response = get_from_start(client, &dl.url).await?;
    learn(dl, &response);
    let file = File::create(part).await?;
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
    let response = get(client, &dl.url, Some((offset, None)), Some(validator)).await?;
    match response.status() {
        StatusCode::PARTIAL_CONTENT => {
            if !content_range_matches(response.headers(), offset, None, dl.total) {
                return Ok(Resume::Refused(RestartReason::RemoteChanged));
            }
            let file = OpenOptions::new().append(true).open(part).await?;
            dl.downloaded = offset;
            Ok(Resume::Continue(Some((response, file))))
        }
        StatusCode::OK => {
            let reason = response_reason(&response, validator);
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

enum Resumed {
    /// Completed, or paused again.
    Stopped(Outcome),
    Restart(RestartReason),
}

async fn resume_segmented(
    env: &Env<'_>,
    dl: &mut Download,
    name: &str,
    part: &Path,
    on_event: &mut impl FnMut(Event),
) -> Result<Resumed, Error> {
    let reason = match fs::metadata(part).await {
        Err(e) if e.kind() == io::ErrorKind::NotFound => RestartReason::PartFileMissing,
        Err(e) => return Err(e.into()),
        Ok(meta) if Some(meta.len()) != dl.total => RestartReason::PartFileDamaged,
        Ok(_) if !dl.resumable || dl.validator.is_none() => RestartReason::NotResumable,
        Ok(_) => {
            dl.downloaded = dl.segments.iter().map(|s| s.done).sum();
            started(dl, name, on_event);
            match segmented::run(env, dl, part, None, on_event).await? {
                Segmented::Finished => return complete(dl, name).await.map(Resumed::Stopped),
                Segmented::Cancelled => return Ok(Resumed::Stopped(Outcome::Paused)),
                Segmented::Restart(reason) => reason,
            }
        }
    };
    Ok(Resumed::Restart(reason))
}

/// A 200 to a range request means either the `If-Range` validator didn't
/// match (the file changed) or the server ignores ranges. The validator tells which.
pub(crate) fn response_reason(response: &Response, validator: &Validator) -> RestartReason {
    if validator_from(response.headers()).as_ref() == Some(validator) {
        RestartReason::RangesIgnored
    } else {
        RestartReason::RemoteChanged
    }
}

fn restarting(dl: &mut Download, reason: RestartReason, on_event: &mut impl FnMut(Event)) {
    tracing::debug!(url = %redact(&dl.url), ?reason, "restarting download from zero");
    dl.downloaded = 0;
    dl.segments.clear();
    on_event(Event::Restarted(reason));
}

fn started(dl: &Download, name: &str, on_event: &mut impl FnMut(Event)) {
    on_event(Event::Started {
        file_name: name.to_owned(),
        dest_dir: dl.dest_dir.clone(),

        total: dl.total,
        validator: dl.validator.clone(),
        resumable: dl.resumable,
        downloaded: dl.downloaded,
        segments: dl.segments.clone(),
    });
}

/// Records what a response from the start of the file tells us about it.
fn learn(dl: &mut Download, response: &Response) {
    let headers = response.headers();
    let refuses_ranges = headers
        .get(ACCEPT_RANGES)
        .is_some_and(|v| v.as_bytes().eq_ignore_ascii_case(b"none"));
    dl.total = match response.status() {
        StatusCode::PARTIAL_CONTENT => content_range(headers).and_then(|(_, _, size)| size),
        _ => response.content_length(),
    };
    dl.validator = validator_from(headers);
    dl.resumable = dl.validator.is_some() && !refuses_ranges;
    dl.downloaded = 0;
}

/// The total size if this fresh response may be split across connections:
/// the server answered our range with a 206 (so ranges really work), we can
/// check the file stays the same, and it's big enough to be worth it.
fn splittable(config: &HttpConfig, dl: &Download, response: &Response) -> Option<u64> {
    let total = dl.total?;
    let worth_it = config.connections > 1 && total >= 2 * config.min_split.max(1);
    let safe = response.status() == StatusCode::PARTIAL_CONTENT && dl.resumable;
    (worth_it && safe && dl.downloaded == 0).then_some(total)
}

async fn single(
    env: &Env<'_>,
    response: Response,
    file: File,
    dl: &mut Download,
    name: &str,
    on_event: &mut impl FnMut(Event),
) -> Result<Outcome, Error> {
    let mut writer = BufWriter::new(file);
    let streamed = stream_body(env, response, &mut writer, dl, on_event).await;
    // Flush even after an error, so the .part.ottie length matches `dl.downloaded`.
    let flushed = flush(&mut writer).await;
    drop(writer); // Windows can't rename a file that's still open.
    if streamed? == Stop::Cancelled {
        flushed?;
        return Ok(Outcome::Paused);
    }
    flushed?;
    complete(dl, name).await
}

#[derive(Debug, PartialEq, Eq)]
enum Stop {
    Finished,
    Cancelled,
}

async fn stream_body(
    env: &Env<'_>,
    mut response: Response,
    writer: &mut BufWriter<File>,
    dl: &mut Download,
    on_event: &mut impl FnMut(Event),
) -> Result<Stop, Error> {
    let Env {
        throttle, cancel, ..
    } = env;
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
        // Over the speed limit: wait before reading more (a pause still wins).
        tokio::select! {
            biased;
            _ = cancel.cancelled() => return Ok(Stop::Cancelled),
            () = throttle.spend(bytes.len()) => {}
        }
    }
}

/// Writes buffered bytes and asks the OS to put them on disk.
pub(crate) async fn flush(writer: &mut BufWriter<File>) -> io::Result<()> {
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
    dl.segments.clear();
    Ok(Outcome::Completed { path })
}

/// Sends a GET. `range` is `(first byte, last byte if any)`; `if_range` makes
/// the server send the whole file instead if it has changed since.
pub(crate) async fn get(
    client: &Client,
    url: &Url,
    range: Option<(u64, Option<u64>)>,
    if_range: Option<&Validator>,
) -> Result<Response, Error> {
    check_scheme(url)?;
    // Ask for the raw bytes: no compression, so ranges map to file offsets.
    let mut request = client.get(url.clone()).header(ACCEPT_ENCODING, "identity");
    if let Some((first, last)) = range {
        let last = last.map(|l| l.to_string()).unwrap_or_default();
        request = request.header(RANGE, format!("bytes={first}-{last}"));
    }
    if let Some(validator) = if_range {
        request = request.header(IF_RANGE, validator.header_value());
    }
    Ok(request.send().await?)
}

/// Asks for `bytes=0-`: a 206 answer proves the server really supports
/// ranges, which the `Accept-Ranges` header alone doesn't.
async fn get_from_start(client: &Client, url: &Url) -> Result<Response, Error> {
    let response = get(client, url, Some((0, None)), None).await?;
    match response.status() {
        StatusCode::OK => Ok(response),
        StatusCode::PARTIAL_CONTENT if content_range_matches(response.headers(), 0, None, None) => {
            Ok(response)
        }
        // Some servers refuse any range on an empty file.
        StatusCode::RANGE_NOT_SATISFIABLE => {
            let response = get(client, url, None, None).await?;
            match response.status() {
                StatusCode::OK => Ok(response),
                status => Err(Error::Status(status)),
            }
        }
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

/// A 206 must start exactly where we asked, end where we asked (or at the
/// end of the file), and describe a file of the size we expect.
pub(crate) fn content_range_matches(
    headers: &HeaderMap,
    first: u64,
    last: Option<u64>,
    total: Option<u64>,
) -> bool {
    let Some((start, end, size)) = content_range(headers) else {
        return false;
    };
    let end_ok = match (last, size) {
        (Some(last), _) => end == last,
        (None, Some(size)) => end + 1 == size,
        (None, None) => true,
    };
    let size_ok = match (total, size) {
        (Some(total), Some(size)) => total == size,
        _ => true,
    };
    start == first && end_ok && size_ok
}

fn content_range(headers: &HeaderMap) -> Option<(u64, u64, Option<u64>)> {
    headers
        .get(CONTENT_RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_content_range)
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
