//! Speed limits. Each running job has a token bucket (a byte budget) that a
//! background task refills every tick, from the job's own limit and its fair
//! share of the global limit. Connections spend from their job's bucket and
//! wait when it's empty. See docs/decisions/0010.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::time::{Duration, Instant};

use tokio::time::MissedTickBehavior;

use crate::id::JobId;
use crate::job::Priority;

const TICK: Duration = Duration::from_millis(50);
/// A bucket holds at most this many seconds of its rate, so speed stays
/// smooth rather than bursting after a pause...
const BURST_SECS: f64 = 0.25;
/// ...but always has room for a typical network read.
const MIN_CAPACITY: f64 = 64.0 * 1024.0;
/// Ticks longer than this (the computer slept) don't hand out a huge budget.
const MAX_TICK_SECS: f64 = 0.2;

/// Shared between the refill task and every [`Throttle`]. Only short,
/// non-async code runs while it's locked.
#[derive(Default)]
struct State {
    /// Bytes per second for all downloads together; 0 = unlimited.
    global: u64,
    buckets: HashMap<JobId, Bucket>,
}

struct Bucket {
    /// This job's own limit in bytes per second; 0 = none.
    limit: u64,
    priority: Priority,
    /// Bytes that may be downloaded now. Goes negative when a chunk was
    /// bigger than the budget; the job then waits until it's paid back.
    tokens: f64,
}

/// Owns the speed limits. Cheap to clone. The refill task stops once every
/// clone is dropped.
#[derive(Clone)]
pub(crate) struct Limiter {
    state: Arc<Mutex<State>>,
}

impl Limiter {
    /// Must be called inside a tokio runtime.
    pub(crate) fn new(global: u64) -> Self {
        let state = Arc::new(Mutex::new(State {
            global,
            ..State::default()
        }));
        tokio::spawn(refill_forever(Arc::downgrade(&state)));
        Limiter { state }
    }

    pub(crate) fn set_global(&self, bytes_per_sec: u64) {
        lock(&self.state).global = bytes_per_sec;
    }

    /// Gives a running job its bucket. It's removed when the last clone of
    /// the returned `Throttle` is dropped.
    pub(crate) fn register(&self, id: JobId, limit: u64, priority: Priority) -> Throttle {
        lock(&self.state).buckets.insert(
            id,
            Bucket {
                limit,
                priority,
                tokens: 0.0,
            },
        );
        Throttle {
            registration: Some(Arc::new(Registration {
                state: Arc::clone(&self.state),
                id,
            })),
        }
    }

    /// Only affects running jobs; the manager keeps the value for later runs.
    pub(crate) fn set_limit(&self, id: JobId, bytes_per_sec: u64) {
        if let Some(bucket) = lock(&self.state).buckets.get_mut(&id) {
            bucket.limit = bytes_per_sec;
        }
    }

    pub(crate) fn set_priority(&self, id: JobId, priority: Priority) {
        if let Some(bucket) = lock(&self.state).buckets.get_mut(&id) {
            bucket.priority = priority;
        }
    }
}

/// A job's handle on its speed limit. Clone it for each connection.
#[derive(Clone, Default)]
pub struct Throttle {
    /// `None` = never limited.
    registration: Option<Arc<Registration>>,
}

struct Registration {
    state: Arc<Mutex<State>>,
    id: JobId,
}

impl Drop for Registration {
    fn drop(&mut self) {
        lock(&self.state).buckets.remove(&self.id);
    }
}

impl Throttle {
    /// A throttle that never waits, for downloads outside the manager.
    pub fn unlimited() -> Self {
        Self::default()
    }

    /// Records that `bytes` were downloaded, then waits until the budget
    /// allows more. Returns at once when no limit applies.
    pub(crate) async fn spend(&self, bytes: usize) {
        let Some(registration) = &self.registration else {
            return;
        };
        let mut spent = false;
        loop {
            {
                let mut state = lock(&registration.state);
                let global = state.global;
                let Some(bucket) = state.buckets.get_mut(&registration.id) else {
                    return;
                };
                if global == 0 && bucket.limit == 0 {
                    return;
                }
                if !spent {
                    bucket.tokens -= bytes as f64;
                    spent = true;
                }
                if bucket.tokens >= 0.0 {
                    return;
                }
            }
            // The lock is released while waiting; the refill task runs every tick.
            tokio::time::sleep(TICK).await;
        }
    }
}

/// A panic while the lock is held can't leave these numbers in a harmful
/// state, so a "poisoned" lock is simply used again.
fn lock(state: &Mutex<State>) -> MutexGuard<'_, State> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

async fn refill_forever(state: Weak<Mutex<State>>) {
    let mut ticker = tokio::time::interval(TICK);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut last = Instant::now();
    loop {
        ticker.tick().await;
        let Some(state) = state.upgrade() else {
            return; // Every Limiter is gone.
        };
        let now = Instant::now();
        let secs = (now - last).as_secs_f64().min(MAX_TICK_SECS);
        last = now;
        lock(&state).refill(secs);
    }
}

impl State {
    /// Hands out `secs` worth of budget: Urgent jobs first, then everyone
    /// else by weight, each capped by its own limit and bucket size.
    fn refill(&mut self, secs: f64) {
        let global = self.global as f64;
        let mut urgent = Vec::new();
        let mut others = Vec::new();
        for (id, bucket) in &self.buckets {
            let rate = match (bucket.limit, self.global) {
                (0, 0) => continue, // Unlimited: no budget to track.
                (0, _) => global,
                (limit, _) => limit as f64,
            };
            let capacity = (rate * BURST_SECS).max(MIN_CAPACITY);
            let mut want = capacity - bucket.tokens;
            if bucket.limit > 0 {
                want = want.min(bucket.limit as f64 * secs);
            }
            if want > 0.0 {
                let entry = (*id, want, weight(bucket.priority));
                if bucket.priority == Priority::Urgent {
                    urgent.push(entry);
                } else {
                    others.push(entry);
                }
            }
        }

        let mut gifts: Vec<(JobId, f64)> = Vec::new();
        if self.global == 0 {
            // Only per-job limits: everyone gets what they want.
            gifts.extend(
                urgent
                    .iter()
                    .chain(&others)
                    .map(|&(id, want, _)| (id, want)),
            );
        } else {
            let mut budget = global * secs;
            for group in [&urgent, &others] {
                let wants: Vec<(f64, f64)> = group.iter().map(|&(_, w, wt)| (w, wt)).collect();
                let given = share(budget, &wants);
                budget -= given.iter().sum::<f64>();
                gifts.extend(group.iter().zip(given).map(|(&(id, _, _), g)| (id, g)));
            }
        }
        for (id, gift) in gifts {
            if let Some(bucket) = self.buckets.get_mut(&id) {
                bucket.tokens += gift;
            }
        }
    }
}

/// How big a share of the global limit each priority gets, relative to the
/// others in the same group (Urgent jobs share equally among themselves).
fn weight(priority: Priority) -> f64 {
    match priority {
        Priority::Low => 1.0,
        Priority::Normal => 2.0,
        Priority::High => 4.0,
        Priority::Urgent => 1.0,
    }
}

/// Splits `budget` in proportion to each `(want, weight)`, never giving
/// anyone more than it wants; what one can't use goes to the others
/// ("water filling"). Returns the amount given to each, in order.
fn share(mut budget: f64, wants: &[(f64, f64)]) -> Vec<f64> {
    let mut given = vec![0.0; wants.len()];
    let mut open: Vec<usize> = (0..wants.len()).filter(|&i| wants[i].0 > 0.0).collect();
    while budget > 0.0 && !open.is_empty() {
        let total_weight: f64 = open.iter().map(|&i| wants[i].1).sum();
        let per_weight = budget / total_weight;
        // Those whose remaining want fits in their share are fully served;
        // the leftover is split again among the rest.
        let (served, rest): (Vec<usize>, Vec<usize>) = open
            .iter()
            .partition(|&&i| wants[i].0 - given[i] <= per_weight * wants[i].1);
        if served.is_empty() {
            for &i in &open {
                given[i] += per_weight * wants[i].1;
            }
            break;
        }
        for &i in &served {
            budget -= wants[i].0 - given[i];
            given[i] = wants[i].0;
        }
        open = rest;
    }
    given
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn equal_weights_share_equally() {
        let given = share(300.0, &[(1000.0, 2.0), (1000.0, 2.0), (1000.0, 2.0)]);
        assert!(given.iter().all(|&g| close(g, 100.0)), "{given:?}");
    }

    #[test]
    fn weights_set_the_ratio() {
        // High (4), Normal (2), Low (1).
        let given = share(700.0, &[(1000.0, 4.0), (1000.0, 2.0), (1000.0, 1.0)]);
        assert!(close(given[0], 400.0) && close(given[1], 200.0) && close(given[2], 100.0));
    }

    #[test]
    fn unused_share_goes_to_the_others() {
        // The first only wants 50 of its 150; the other two split the rest.
        let given = share(450.0, &[(50.0, 2.0), (1000.0, 2.0), (1000.0, 2.0)]);
        assert!(close(given[0], 50.0) && close(given[1], 200.0) && close(given[2], 200.0));
    }

    #[test]
    fn nobody_gets_more_than_it_wants() {
        let given = share(1000.0, &[(10.0, 1.0), (20.0, 4.0)]);
        assert!(close(given[0], 10.0) && close(given[1], 20.0));
    }

    fn state(global: u64, jobs: &[(u64, Priority)]) -> (State, Vec<JobId>) {
        let mut state = State {
            global,
            ..State::default()
        };
        let ids: Vec<JobId> = (1..=jobs.len() as u32)
            .map(|n| JobId::from_parts(0, n))
            .collect();
        for (&id, &(limit, priority)) in ids.iter().zip(jobs) {
            state.buckets.insert(
                id,
                Bucket {
                    limit,
                    priority,
                    // Deep in debt, so each wants more than a tick's budget.
                    tokens: -1e9,
                },
            );
        }
        (state, ids)
    }

    fn tokens(state: &State, id: JobId) -> f64 {
        state.buckets[&id].tokens + 1e9
    }

    #[test]
    fn urgent_jobs_are_served_first() {
        let (mut state, ids) = state(1000, &[(0, Priority::Urgent), (0, Priority::High)]);
        state.refill(1.0);
        assert!(close(tokens(&state, ids[0]), 1000.0));
        assert!(close(tokens(&state, ids[1]), 0.0));
    }

    #[test]
    fn urgent_leftovers_go_to_the_others() {
        // The urgent job's own limit is 300, so 700 is left for the rest.
        let (mut state, ids) = state(1000, &[(300, Priority::Urgent), (0, Priority::Normal)]);
        state.refill(1.0);
        assert!(close(tokens(&state, ids[0]), 300.0));
        assert!(close(tokens(&state, ids[1]), 700.0));
    }

    #[test]
    fn own_limits_apply_without_a_global_limit() {
        let (mut state, ids) = state(0, &[(500, Priority::Normal), (0, Priority::Normal)]);
        state.refill(0.1);
        assert!(close(tokens(&state, ids[0]), 50.0));
        assert!(
            close(tokens(&state, ids[1]), 0.0),
            "unlimited jobs aren't tracked"
        );
    }

    #[test]
    fn full_buckets_take_nothing() {
        let (mut state, ids) = state(1000, &[(0, Priority::Normal), (0, Priority::Normal)]);
        state.buckets.get_mut(&ids[0]).unwrap().tokens = MIN_CAPACITY;
        state.refill(0.1);
        assert!(
            close(tokens(&state, ids[1]), 100.0),
            "gets the whole tick's budget"
        );
    }
}
