//! Multi-connection transfers. A coordinator (the `run` future) owns the list
//! of ranges; each connection is its own task writing at its offset. When a
//! connection finishes, it picks up an unfinished range or takes over the
//! second half of the range with the most left. See docs/decisions/0009.

use std::io::{self, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering::SeqCst};
use std::time::{Duration, Instant};

use reqwest::{Client, Response, StatusCode, Url};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncSeekExt, AsyncWriteExt, BufWriter};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::download::{Download, Event, RestartReason, Segment, Validator};
use crate::error::Error;
use crate::http::{HttpConfig, content_range_matches, flush, get, response_reason};

const PROGRESS_INTERVAL: Duration = Duration::from_millis(200);
/// How often each connection flushes to disk, and so how often the saved
/// progress (`Event::Checkpoint`) can move forward.
const FLUSH_INTERVAL: Duration = Duration::from_secs(1);

pub(crate) enum Segmented {
    Finished,
    Cancelled,
    /// The file changed on the server (or stopped honouring ranges).
    Restart(RestartReason),
}

/// Splits a new download into equal ranges, one per connection, none smaller
/// than `min_split`.
pub(crate) fn plan(total: u64, config: &HttpConfig) -> Vec<Segment> {
    let by_size = total / config.min_split.max(1);
    let count = u64::try_from(config.connections)
        .unwrap_or(u64::MAX)
        .min(by_size)
        .max(1);
    let size = total / count;
    (0..count)
        .map(|i| Segment {
            start: i * size,
            end: if i + 1 == count {
                total
            } else {
                (i + 1) * size
            },
            done: 0,
        })
        .collect()
}

/// Shared between the coordinator and one connection. Atomics let both sides
/// read and update these numbers at any time without locks or messages.
struct Shared {
    /// Exclusive end. Only the coordinator changes it, by lowering it on a split.
    end: AtomicU64,
    /// Next offset the connection will write.
    pos: AtomicU64,
    /// Everything before this offset is flushed to disk.
    durable: AtomicU64,
}

struct Part {
    start: u64,
    shared: Arc<Shared>,
    busy: bool,
}

impl Part {
    fn new(segment: Segment) -> Part {
        let reached = segment.start + segment.done;
        Part {
            start: segment.start,
            shared: Arc::new(Shared {
                end: AtomicU64::new(segment.end),
                pos: AtomicU64::new(reached),
                durable: AtomicU64::new(reached),
            }),
            busy: false,
        }
    }

    fn end(&self) -> u64 {
        self.shared.end.load(SeqCst)
    }

    /// Written so far. Can briefly pass `end` after a split, so it's capped.
    fn pos(&self) -> u64 {
        self.shared.pos.load(SeqCst).min(self.end())
    }

    fn left(&self) -> u64 {
        self.end() - self.pos()
    }

    fn segment(&self) -> Segment {
        let end = self.end();
        let durable = self.shared.durable.load(SeqCst).min(end);
        Segment {
            start: self.start,
            end,
            done: durable - self.start,
        }
    }
}

/// What every connection needs, shared read-only.
struct Context {
    client: Client,
    url: Url,
    validator: Validator,
    path: PathBuf,
    total: u64,
}

enum Ended {
    /// Reached the end of its range.
    Done,
    Cancelled,
    Changed(RestartReason),
}

/// Downloads every unfinished range of `dl.segments` into the full-size file
/// at `path`. `first` is an already-open response starting at byte 0 (fresh
/// downloads reuse the request that discovered the file).
pub(crate) async fn run(
    client: &Client,
    config: &HttpConfig,
    dl: &mut Download,
    path: &Path,
    first: Option<Response>,
    cancel: &CancellationToken,
    on_event: &mut impl FnMut(Event),
) -> Result<Segmented, Error> {
    let (Some(total), Some(validator)) = (dl.total, dl.validator.clone()) else {
        return Ok(Segmented::Restart(RestartReason::NotResumable));
    };
    let context = Arc::new(Context {
        client: client.clone(),
        url: dl.url.clone(),
        validator,
        path: path.to_owned(),
        total,
    });
    let mut parts: Vec<Part> = dl.segments.iter().copied().map(Part::new).collect();
    // Stops every connection, either because the user paused or because
    // one connection hit a problem that ends the whole run.
    let stop = cancel.child_token();
    let mut tasks = JoinSet::new();
    let mut first = first;
    let mut failure: Option<Error> = None;
    let mut restart: Option<RestartReason> = None;
    fill(&mut parts, &mut tasks, &context, &stop, &mut first, config);

    let mut ticker = tokio::time::interval(PROGRESS_INTERVAL);
    let mut last_progress = written(&parts);
    let mut last_checkpoint = (Instant::now(), segments(&parts));
    while !tasks.is_empty() {
        tokio::select! {
            Some(joined) = tasks.join_next() => {
                let Ok((index, result)) = joined else {
                    // A connection task panicked. Stop the others; resuming is safe.
                    failure.get_or_insert(Error::Io(io::Error::other("connection task panicked")));
                    stop.cancel();
                    continue;
                };
                parts[index].busy = false;
                match result {
                    Ok(Ended::Done) if !stop.is_cancelled() => {
                        fill(&mut parts, &mut tasks, &context, &stop, &mut first, config);
                    }
                    Ok(Ended::Done | Ended::Cancelled) => {}
                    Ok(Ended::Changed(reason)) => {
                        restart.get_or_insert(reason);
                        stop.cancel();
                    }
                    Err(err) => {
                        failure.get_or_insert(err);
                        stop.cancel();
                    }
                }
            }
            _ = ticker.tick() => {
                let now = written(&parts);
                if now != last_progress {
                    last_progress = now;
                    on_event(Event::Progress { downloaded: now, total: Some(total) });
                }
                if last_checkpoint.0.elapsed() >= FLUSH_INTERVAL {
                    let current = segments(&parts);
                    if current != last_checkpoint.1 {
                        on_event(Event::Checkpoint { segments: current.clone() });
                    }
                    last_checkpoint = (Instant::now(), current);
                }
            }
        }
    }

    // Every connection has stopped and flushed: record exactly what's on disk.
    dl.segments = segments(&parts);
    dl.downloaded = dl.segments.iter().map(|s| s.done).sum();
    on_event(Event::Checkpoint {
        segments: dl.segments.clone(),
    });

    if dl.segments.iter().all(Segment::is_complete) {
        Ok(Segmented::Finished)
    } else if cancel.is_cancelled() {
        Ok(Segmented::Cancelled)
    } else if let Some(reason) = restart {
        Ok(Segmented::Restart(reason))
    } else if let Some(err) = failure {
        Err(err)
    } else {
        Err(Error::Incomplete {
            received: dl.downloaded,
            expected: total,
        })
    }
}

type Task = (usize, Result<Ended, Error>);

/// Starts connections until `config.connections` are busy: first on ranges
/// nobody is working on, then by splitting the busy range with the most left.
fn fill(
    parts: &mut Vec<Part>,
    tasks: &mut JoinSet<Task>,
    context: &Arc<Context>,
    stop: &CancellationToken,
    first: &mut Option<Response>,
    config: &HttpConfig,
) {
    while parts.iter().filter(|p| p.busy).count() < config.connections.max(1) {
        let idle = parts.iter().position(|p| !p.busy && p.left() > 0);
        let Some(index) = idle.or_else(|| split(parts, config.min_split)) else {
            return;
        };
        let part = &mut parts[index];
        part.busy = true;
        // The opening response starts at byte 0, so only the range there can use it.
        let response = if part.pos() == 0 { first.take() } else { None };
        let context = Arc::clone(context);
        let shared = Arc::clone(&part.shared);
        let stop = stop.clone();
        tasks.spawn(async move {
            let result = connection(&context, &shared, response, &stop).await;
            (index, result)
        });
    }
}

/// Gives the second half of the busy range with the most left to a new range.
/// The busy connection notices its lower `end` before its next write.
fn split(parts: &mut Vec<Part>, min_split: u64) -> Option<usize> {
    let (index, left) = parts
        .iter()
        .enumerate()
        .filter(|(_, p)| p.busy)
        .map(|(i, p)| (i, p.left()))
        .max_by_key(|&(_, left)| left)?;
    if left < 2 * min_split.max(1) {
        return None;
    }
    let old_end = parts[index].end();
    let middle = old_end - left / 2;
    parts[index].shared.end.store(middle, SeqCst);
    parts.push(Part::new(Segment {
        start: middle,
        end: old_end,
        done: 0,
    }));
    Some(parts.len() - 1)
}

/// One connection: request its range (unless given an open response), then
/// write until it reaches its `end`, which may move down while it runs.
/// If a split lands just behind it, it may write a few bytes past the new
/// `end`. That's harmless: `If-Range` guarantees they're the same bytes the
/// other connection writes there.
async fn connection(
    context: &Context,
    shared: &Shared,
    response: Option<Response>,
    stop: &CancellationToken,
) -> Result<Ended, Error> {
    let mut pos = shared.pos.load(SeqCst);
    let mut response = match response {
        Some(response) => response,
        None => match request(context, pos, shared.end.load(SeqCst), stop).await? {
            Ok(response) => response,
            Err(ended) => return Ok(ended),
        },
    };

    let mut file = OpenOptions::new().write(true).open(&context.path).await?;
    file.seek(SeekFrom::Start(pos)).await?;
    let mut writer = BufWriter::new(file);
    let mut last_flush = Instant::now();

    let result = async {
        loop {
            if pos >= shared.end.load(SeqCst) {
                return Ok(Ended::Done);
            }
            let chunk = tokio::select! {
                biased;
                _ = stop.cancelled() => return Ok(Ended::Cancelled),
                chunk = response.chunk() => chunk?,
            };
            let end = shared.end.load(SeqCst);
            let Some(bytes) = chunk else {
                // The server closed the range before its end.
                return Err(Error::Incomplete {
                    received: pos,
                    expected: end,
                });
            };
            let wanted = usize::try_from(end.saturating_sub(pos)).unwrap_or(usize::MAX);
            let take = wanted.min(bytes.len());
            writer.write_all(&bytes[..take]).await?;
            pos += take as u64;
            shared.pos.store(pos, SeqCst);
            if last_flush.elapsed() >= FLUSH_INTERVAL {
                flush(&mut writer).await?;
                shared.durable.store(pos, SeqCst);
                last_flush = Instant::now();
            }
        }
    }
    .await;

    // Whatever happened, flush so `durable` covers everything written.
    match flush(&mut writer).await {
        Ok(()) => shared.durable.store(pos, SeqCst),
        Err(err) if result.is_ok() => return Err(err.into()),
        Err(_) => {} // Report the first problem, not this one.
    }
    result
}

/// Requests `pos..end` with `If-Range`. `Ok(Err(_))` means the connection
/// ended without data: paused, or the file changed on the server.
async fn request(
    context: &Context,
    pos: u64,
    end: u64,
    stop: &CancellationToken,
) -> Result<Result<Response, Ended>, Error> {
    let last = end - 1;
    let sent = get(
        &context.client,
        &context.url,
        Some((pos, Some(last))),
        Some(&context.validator),
    );
    let response = tokio::select! {
        biased;
        _ = stop.cancelled() => return Ok(Err(Ended::Cancelled)),
        response = sent => response?,
    };
    Ok(match response.status() {
        StatusCode::PARTIAL_CONTENT
            if content_range_matches(response.headers(), pos, Some(last), Some(context.total)) =>
        {
            Ok(response)
        }
        StatusCode::PARTIAL_CONTENT | StatusCode::RANGE_NOT_SATISFIABLE => {
            Err(Ended::Changed(RestartReason::RemoteChanged))
        }
        StatusCode::OK => Err(Ended::Changed(response_reason(
            &response,
            &context.validator,
        ))),
        status => return Err(Error::Status(status)),
    })
}

/// Bytes written so far, flushed or not (for progress display).
fn written(parts: &[Part]) -> u64 {
    parts.iter().map(|p| p.pos() - p.start).sum()
}

/// The ranges as flushed to disk, sorted, ready to save.
fn segments(parts: &[Part]) -> Vec<Segment> {
    let mut segments: Vec<Segment> = parts.iter().map(Part::segment).collect();
    segments.sort_by_key(|s| s.start);
    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(connections: usize, min_split: u64) -> HttpConfig {
        HttpConfig {
            connections,
            min_split,
        }
    }

    #[test]
    fn plan_covers_the_whole_file_without_gaps() {
        let segments = plan(1003, &config(4, 100));
        assert_eq!(segments.len(), 4);
        assert_eq!(segments[0].start, 0);
        assert_eq!(segments.last().unwrap().end, 1003);
        for pair in segments.windows(2) {
            assert_eq!(pair[0].end, pair[1].start);
        }
    }

    #[test]
    fn plan_respects_the_minimum_size() {
        assert_eq!(plan(250, &config(8, 100)).len(), 2);
        assert_eq!(plan(50, &config(8, 100)).len(), 1);
    }

    #[test]
    fn split_gives_away_the_second_half_of_the_biggest_busy_range() {
        let mut parts: Vec<Part> = plan(1000, &config(2, 100))
            .into_iter()
            .map(Part::new)
            .collect();
        parts[0].busy = true;
        parts[1].busy = true;
        parts[0].shared.pos.store(400, SeqCst); // 100 left
        parts[1].shared.pos.store(600, SeqCst); // 400 left

        let new = split(&mut parts, 100).unwrap();
        assert_eq!(parts[1].end(), 800);
        assert_eq!((parts[new].start, parts[new].end()), (800, 1000));
        // 200 left at most now; splitting it would make pieces under 100.
        parts[new].busy = true;
        parts[1].shared.pos.store(750, SeqCst);
        parts[new].shared.pos.store(900, SeqCst);
        assert!(split(&mut parts, 100).is_none());
    }

    #[test]
    fn segment_never_counts_bytes_past_a_lowered_end() {
        let part = Part::new(Segment {
            start: 0,
            end: 100,
            done: 0,
        });
        part.shared.pos.store(90, SeqCst);
        part.shared.durable.store(90, SeqCst);
        part.shared.end.store(80, SeqCst);
        assert_eq!(part.segment().done, 80);
        assert_eq!(part.left(), 0);
    }
}
