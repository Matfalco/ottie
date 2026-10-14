//! What the tray shows (docs/decisions/0034): the daemon's downloads,
//! followed through its events stream and summed up once a second.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use futures_util::StreamExt;
use tauri::{AppHandle, Manager};
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message;

use crate::client::{Daemon, Event, Job};
use crate::prefs;

/// How often the summary is refreshed.
const EVERY: Duration = Duration::from_secs(1);
/// The wait before following the daemon again after losing it.
const RECONNECT: Duration = Duration::from_secs(2);
/// How much a new measurement moves the speed (the rest is history), as on
/// the page (`ui/src/lib/jobs.ts`).
const SMOOTHING: f64 = 0.3;
/// Measurements closer than this are too noisy to use.
const MIN_INTERVAL: Duration = Duration::from_millis(200);
/// No progress for this long: the download's speed is 0.
const STALE_AFTER: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    /// The events stream is open; until then, the rest is empty.
    pub connected: bool,
    /// Connecting, downloading or retrying.
    pub running: usize,
    /// Queued.
    pub waiting: usize,
    pub paused: usize,
    /// Every download's speed added up, in bytes per second.
    pub speed: u64,
    /// The total limit; 0 = none.
    pub global_limit: u64,
}

/// Follows the daemon for as long as the app runs, sending a summary every
/// second; reconnects when the stream ends (the daemon restarted or stopped).
pub async fn follow(app: AppHandle, daemon: Arc<Daemon>, out: watch::Sender<Summary>) {
    let mut model = Model::default();
    loop {
        // Either way it ended, the tray says it's connecting again.
        let _ = session(&app, &daemon, &mut model, &out).await;
        out.send_replace(Summary::default());
        tokio::time::sleep(RECONNECT).await;
    }
}

/// One connection to the events stream, until it ends.
async fn session(
    app: &AppHandle,
    daemon: &Daemon,
    model: &mut Model,
    out: &watch::Sender<Summary>,
) -> Result<()> {
    let mut events = daemon.events().await?;
    load(app, daemon, model).await?;
    let mut tick = tokio::time::interval(EVERY);
    loop {
        tokio::select! {
            message = events.next() => {
                let text = match message {
                    Some(Ok(Message::Text(text))) => text,
                    Some(Ok(Message::Close(_))) | None => return Ok(()),
                    // Pings are answered by the stream itself.
                    Some(Ok(_)) => continue,
                    Some(Err(err)) => return Err(err.into()),
                };
                // Unreadable: not one of the tray's, or from a newer daemon.
                let Ok(event) = serde_json::from_str::<Event>(&text) else {
                    continue;
                };
                if let Event::SettingsChanged { settings } = &event {
                    remember_limit(app, settings.global_limit);
                }
                if model.apply(event, Instant::now()) {
                    load(app, daemon, model).await?;
                }
            }
            _ = tick.tick() => {
                out.send_replace(model.summary(Instant::now()));
            }
        }
    }
}

/// The whole list and the settings, at the start and when the stream says
/// events were lost (`resync`).
async fn load(app: &AppHandle, daemon: &Daemon, model: &mut Model) -> Result<()> {
    let jobs = daemon.jobs().await?;
    let settings = daemon.settings().await?;
    model.reset(jobs, Instant::now());
    model.global_limit = settings.global_limit;
    remember_limit(app, settings.global_limit);
    Ok(())
}

/// The tray's "Limit speed" switches the last limit back on.
fn remember_limit(app: &AppHandle, limit: u64) {
    if limit > 0 {
        // Best effort: without it, the tray offers the default limit.
        let _ = app
            .state::<prefs::Saved>()
            .change(app, |prefs| prefs.last_limit = Some(limit));
    }
}

/// The downloads as the events tell them.
#[derive(Debug, Default)]
struct Model {
    jobs: HashMap<String, Tracked>,
    global_limit: u64,
}

#[derive(Debug)]
struct Tracked {
    state: String,
    rate: Rate,
}

impl Model {
    /// A fresh list; downloads still there keep their measured speed.
    fn reset(&mut self, jobs: Vec<Job>, now: Instant) {
        let mut old = std::mem::take(&mut self.jobs);
        for job in jobs {
            let rate = match old.remove(&job.id) {
                Some(tracked) => tracked.rate.next(job.downloaded, now),
                None => Rate::first(job.downloaded, now),
            };
            self.jobs.insert(
                job.id,
                Tracked {
                    state: job.state,
                    rate,
                },
            );
        }
    }

    /// Applies an event; true if the whole list must be fetched again.
    fn apply(&mut self, event: Event, now: Instant) -> bool {
        match event {
            Event::Changed { job } => {
                let rate = match self.jobs.get(&job.id) {
                    Some(tracked) => tracked.rate.next(job.downloaded, now),
                    None => Rate::first(job.downloaded, now),
                };
                self.jobs.insert(
                    job.id,
                    Tracked {
                        state: job.state,
                        rate,
                    },
                );
            }
            Event::Progress { id, downloaded } => {
                if let Some(tracked) = self.jobs.get_mut(&id) {
                    tracked.rate = tracked.rate.next(downloaded, now);
                }
            }
            Event::Removed { id } => {
                self.jobs.remove(&id);
            }
            Event::SettingsChanged { settings } => self.global_limit = settings.global_limit,
            Event::Resync => return true,
            Event::Other => {}
        }
        false
    }

    fn summary(&self, now: Instant) -> Summary {
        let mut summary = Summary {
            connected: true,
            global_limit: self.global_limit,
            ..Summary::default()
        };
        let mut speed = 0.0;
        for tracked in self.jobs.values() {
            match tracked.state.as_str() {
                "connecting" | "downloading" | "retrying" => {
                    summary.running += 1;
                    speed += tracked.rate.current(now);
                }
                "queued" => summary.waiting += 1,
                "paused" => summary.paused += 1,
                _ => {}
            }
        }
        // Rounded down to whole bytes: only shown, never computed with.
        summary.speed = speed as u64;
        summary
    }
}

/// One download's speed, smoothed as on the page (`nextRate`).
#[derive(Debug, Clone, Copy)]
struct Rate {
    at: Instant,
    downloaded: u64,
    per_second: f64,
}

impl Rate {
    fn first(downloaded: u64, now: Instant) -> Rate {
        Rate {
            at: now,
            downloaded,
            per_second: 0.0,
        }
    }

    fn next(self, downloaded: u64, now: Instant) -> Rate {
        // The download restarted from zero.
        if downloaded < self.downloaded {
            return Rate::first(downloaded, now);
        }
        let elapsed = now.saturating_duration_since(self.at);
        if elapsed < MIN_INTERVAL {
            return self;
        }
        let measured = (downloaded - self.downloaded) as f64 / elapsed.as_secs_f64();
        let per_second = if self.per_second == 0.0 {
            measured
        } else {
            self.per_second + SMOOTHING * (measured - self.per_second)
        };
        Rate {
            at: now,
            downloaded,
            per_second,
        }
    }

    fn current(&self, now: Instant) -> f64 {
        if now.saturating_duration_since(self.at) > STALE_AFTER {
            0.0
        } else {
            self.per_second
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::Settings;

    fn job(id: &str, state: &str, downloaded: u64) -> Job {
        Job {
            id: id.to_owned(),
            state: state.to_owned(),
            downloaded,
            moving: false,
        }
    }

    #[test]
    fn the_summary_counts_downloads_by_state() {
        let now = Instant::now();
        let mut model = Model::default();
        model.reset(
            vec![
                job("a", "downloading", 0),
                job("b", "retrying", 0),
                job("c", "queued", 0),
                job("d", "paused", 0),
                job("e", "completed", 0),
            ],
            now,
        );
        model.apply(
            Event::SettingsChanged {
                settings: Settings { global_limit: 512 },
            },
            now,
        );
        let summary = model.summary(now);
        assert_eq!(
            summary,
            Summary {
                connected: true,
                running: 2,
                waiting: 1,
                paused: 1,
                speed: 0,
                global_limit: 512,
            }
        );
        model.apply(Event::Removed { id: "a".to_owned() }, now);
        model.apply(
            Event::Changed {
                job: job("c", "connecting", 0),
            },
            now,
        );
        let summary = model.summary(now);
        assert_eq!((summary.running, summary.waiting), (2, 0));
    }

    #[test]
    fn the_speed_adds_up_progress_and_goes_stale() {
        let start = Instant::now();
        let mut model = Model::default();
        model.reset(
            vec![job("a", "downloading", 0), job("b", "downloading", 0)],
            start,
        );
        let second = start + Duration::from_secs(1);
        for (id, downloaded) in [("a", 1000), ("b", 3000)] {
            let progress = Event::Progress {
                id: id.to_owned(),
                downloaded,
            };
            model.apply(progress, second);
        }
        assert_eq!(model.summary(second).speed, 4000);
        // Smoothed: a faster second only moves it part of the way.
        let third = second + Duration::from_secs(1);
        let progress = Event::Progress {
            id: "a".to_owned(),
            downloaded: 3000,
        };
        model.apply(progress, third);
        assert_eq!(model.summary(third).speed, 1300 + 3000);
        // "b" has had nothing for 3.5 s, "a" for 2.5 s.
        let later = third + Duration::from_millis(2500);
        assert_eq!(model.summary(later).speed, 1300);
    }

    #[test]
    fn a_restart_or_a_close_measurement_doesnt_count() {
        let start = Instant::now();
        let rate = Rate::first(5000, start);
        let soon = rate.next(9000, start + Duration::from_millis(50));
        assert_eq!(soon.downloaded, 5000);
        let restarted = rate.next(100, start + Duration::from_secs(1));
        assert_eq!(restarted.per_second, 0.0);
        assert_eq!(restarted.downloaded, 100);
    }
}
