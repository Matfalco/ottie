# 0006: Job IDs are `yyyy.mm.dd-hh.mm.ss-N` (UTC)

Accepted 2026-10-12.

**Decision:** a job's ID is its creation time in UTC, followed by a counter that restarts at 1 every second. For example, `2026.10.12-14.30.05-1` is followed by `2026.10.12-14.30.05-2` if a second job is added in the same second. The API, CLI, UI and logs only accept and show the full ID; there's no number-only shortcut.

**Why**
- IDs sort chronologically as plain text, and they show when a job was added.
- UTC because local time repeats an hour when daylight saving ends, which would break the ordering. It also stays consistent for a daemon running in another time zone, such as on a NAS. UIs can still show local time alongside.

**Rules**
- IDs only ever increase. If the clock goes backwards (NTP correction, manual change), new IDs reuse the last issued second and keep counting, so an old ID never comes back.
- Parsing is strict: the text must re-format to exactly itself. That rejects `2025.02.30`, missing zero padding, a counter of `0` or `01`, and similar.
- Alternatives considered: plain integers (short, CLI-friendly) and UUIDv7 (globally unique, but long).

**Implementation:** `crates/core/src/id.rs`. Dates use Howard Hinnant's public-domain day/civil conversions, with no date-library dependency. Persistence (step 2b) must store `secs` and `seq` and seed the generator with the highest stored ID.
