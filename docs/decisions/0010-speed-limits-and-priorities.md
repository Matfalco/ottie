# 0010: Speed limits, priorities and the settings table

Accepted 2026-10-12.

**Priorities:** four levels, `Low`, `Normal` (default), `High` and `Urgent`, stored per job.
- **Queue:** when a slot frees up, the highest-priority queued job starts; within a level, the oldest (since `docs/decisions/0032`: the lowest place in the queue, which starts as the oldest and can be changed). Urgent jobs still wait for a free slot; they don't stop a running job.
- **Bandwidth under the global limit:** Urgent jobs are served first, sharing equally among themselves. Everyone else shares what's left by weight: High 4, Normal 2, Low 1 (twice and half a Normal share). Budget a job can't use (its server is slow, or its own limit is lower) goes to the others. The share is per download, not per connection: a 4-connection download gets the same share as a 1-connection one.

**Limits:** bytes per second, 0 = unlimited, changeable while downloading.
- A global limit (a setting) and a per-job limit (`speed_limit`, saved with the job).
- Categories (2e) add a middle level: global → category → job.

**How it works** (`crates/core/src/throttle.rs`)
- Each running job has a token bucket: a byte budget shared by all its connections. Connections spend from it after writing each chunk, and wait (in 50 ms steps, still interruptible by pause) while it's negative.
- A refill task runs every 50 ms. With a global limit, it splits `global × elapsed` by "water filling" (`share()`): Urgent group first, then the weighted rest, each capped by its own limit and by its bucket having room. Without a global limit, each job gets its own limit's worth.
- A bucket holds at most 0.25 s of its rate (at least 64 KiB), so speed stays smooth rather than bursting.
- Shared state is a `std::sync::Mutex`, held only for a few arithmetic steps and never across an `.await`. This is throttling state, not job state, so the one-owner rule for jobs still holds.

**Settings table:** a single typed row (`CHECK (id = 1)`), one real column per setting: `global_limit`, `max_concurrent`. Adding a setting is a one-line migration. Settings are saved when changed and loaded at start, which is why `ManagerConfig` was removed: `Manager::start(backend, store)` reads them from the database.

A download that gets no bandwidth (others take it all) lets go of its requests and says it waits: `docs/decisions/0033`.

**Alternatives considered:** first-come-first-served sharing (lets multi-connection downloads crowd out others), "served first" for every priority level (Low and Normal could starve), and a key/value settings table (no type checks).
