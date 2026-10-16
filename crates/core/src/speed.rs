//! The total download speed over the last hour, one sample per second, for
//! the front ends' graph (docs/decisions/0044). Each running job's speed is
//! estimated from its progress reports as the pages do (`nextRate` in the
//! UI): bytes arrive in chunks of up to a few hundred kB when a limit holds a
//! job back, so counting the bytes of each whole second would zigzag. Kept
//! by the manager's actor, in memory only: a restart starts it again.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use crate::id::JobId;

/// One hour of one-per-second samples.
pub(crate) const HISTORY_SECS: usize = 3600;
/// How much a new measurement moves a job's speed (the rest is history).
const SMOOTHING: f64 = 0.3;
/// Measurements closer than this are too noisy to use.
const MIN_INTERVAL: Duration = Duration::from_millis(200);
/// No progress for this long: the job counts as not downloading.
const STALE_AFTER: Duration = Duration::from_secs(3);

#[derive(Debug, Default)]
pub(crate) struct SpeedHistory {
    /// The total, in bytes per second, oldest first.
    samples: VecDeque<u64>,
    /// Each job's speed, from its last progress reports.
    rates: HashMap<JobId, Rate>,
}

#[derive(Debug, Clone, Copy)]
struct Rate {
    at: Instant,
    downloaded: u64,
    bytes_per_sec: f64,
}

impl SpeedHistory {
    /// A job reported `downloaded` bytes in all at `now`.
    pub(crate) fn progress(&mut self, id: JobId, downloaded: u64, now: Instant) {
        let fresh = Rate {
            at: now,
            downloaded,
            bytes_per_sec: 0.0,
        };
        let rate = match self.rates.get(&id) {
            // First sight, or it restarted from zero.
            None => fresh,
            Some(previous) if downloaded < previous.downloaded => fresh,
            Some(previous) => {
                let elapsed = now.saturating_duration_since(previous.at);
                if elapsed < MIN_INTERVAL {
                    return;
                }
                let measured = (downloaded - previous.downloaded) as f64 / elapsed.as_secs_f64();
                let bytes_per_sec = if previous.bytes_per_sec == 0.0 {
                    measured
                } else {
                    previous.bytes_per_sec + SMOOTHING * (measured - previous.bytes_per_sec)
                };
                Rate {
                    at: now,
                    downloaded,
                    bytes_per_sec,
                }
            }
        };
        self.rates.insert(id, rate);
    }

    /// Closes a second: the speeds of the jobs still reporting, added up,
    /// become a sample; the oldest goes once there's an hour of them.
    pub(crate) fn tick(&mut self, now: Instant) {
        self.rates
            .retain(|_, rate| now.saturating_duration_since(rate.at) <= STALE_AFTER);
        let total: f64 = self.rates.values().map(|rate| rate.bytes_per_sec).sum();
        // Whole bytes per second; a speed is never negative nor near `u64::MAX`.
        self.samples.push_back(total.round() as u64);
        while self.samples.len() > HISTORY_SECS {
            self.samples.pop_front();
        }
    }

    /// Bytes per second, oldest first; at most an hour.
    pub(crate) fn samples(&self) -> Vec<u64> {
        self.samples.iter().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::IdGenerator;

    fn secs(s: f64) -> Duration {
        Duration::from_secs_f64(s)
    }

    #[test]
    fn adds_up_each_jobs_smoothed_speed() {
        let mut ids = IdGenerator::after(None);
        let (a, b) = (ids.next(1), ids.next(1));
        let start = Instant::now();
        let mut history = SpeedHistory::default();
        history.progress(a, 0, start);
        history.progress(b, 0, start);
        history.progress(a, 1000, start + secs(1.0));
        history.progress(b, 500, start + secs(1.0));
        history.tick(start + secs(1.0));
        assert_eq!(history.samples(), [1500]);
        // A chunk twice as big moves it only part of the way.
        history.progress(a, 3000, start + secs(2.0));
        history.tick(start + secs(2.0));
        assert_eq!(history.samples(), [1500, 1800]);
    }

    #[test]
    fn a_job_that_stops_reporting_stops_counting() {
        let id = IdGenerator::after(None).next(1);
        let start = Instant::now();
        let mut history = SpeedHistory::default();
        history.progress(id, 0, start);
        history.progress(id, 1000, start + secs(1.0));
        history.tick(start + secs(5.0));
        assert_eq!(history.samples(), [0]);
    }

    #[test]
    fn keeps_one_hour() {
        let start = Instant::now();
        let mut history = SpeedHistory::default();
        for second in 0..HISTORY_SECS + 10 {
            history.tick(start + secs(second as f64));
        }
        assert_eq!(history.samples().len(), HISTORY_SECS);
    }
}
