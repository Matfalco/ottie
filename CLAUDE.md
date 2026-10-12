# CLAUDE.md

**Ottie**: a cross-platform download manager meant to replace uGet (unmaintained). Licensed MPL-2.0. Repo: `github.com/Matfalco/ottie`.

## Naming

| Thing | Name |
|---|---|
| Product | Ottie. Store and extension listings: "Ottie – Download Manager" |
| Mascot / tagline | An otter. Sea otters stash food and a favourite stone in a skin pouch, so Ottie "tucks your files away" |
| Command / binary | `ottie` (one binary; `ottie serve` runs the daemon) |
| Cargo packages | `ottie` (the binary), `ottie-core`, `ottie-api`, `ottie-app` (Tauri) |
| App identifier (Tauri, = Android package) | `io.github.matfalco.ottie`. **Never change it after the first Android build**: a new identifier is a different app with no upgrade path |
| Tauri | `productName` "Ottie", window title "Ottie" |
| Config/data folders | `ottie` on Linux (`~/.config/ottie`, `~/.local/share/ottie`), `Ottie` on Windows/macOS |
| Environment variables | `OTTIE_` prefix |
| systemd unit / Docker image | `ottie.service` (`ExecStart=ottie serve`) / `ottie` |

## What we're building

- A background **daemon** that does all the downloading, with three front ends on top: a **web UI**, a **desktop app** (Windows, Linux, later macOS) and an **Android app**. iOS is out of scope.
- A **browser extension** (Chromium-based browsers and Firefox, desktop) that takes over downloads from the browser: intercepts them, hides the browser's own download UI where possible, and shows our download list in its popup.
- Must-haves: pause/resume (including after a restart or crash), multi-connection downloads, and speed limits at three levels (per download, per category, global), adjustable while downloads run.
- **Play while downloading** (uGet can): the API serves a partial file at `GET /jobs/{id}/stream` with range support, so the browser, VLC/mpv or an Android player can play a video before it finishes, locally or from a remote daemon. Needs an in-order download mode that also fetches the last chunk early (many MP4s keep their index at the end).
- uGet features to bring over time: categories (folder + file-type rules), queue with max concurrent downloads, opt-in clipboard monitoring, batch/URL-list import, scheduler.

## Stack

| Part | Tech |
|---|---|
| Download engine, daemon, API | Rust (edition 2024), tokio, reqwest 0.13, axum, rusqlite (`bundled`) |
| UI (shared by web, desktop, Android) | Svelte 5 + TypeScript + Vite, static build |
| Desktop + Android shell | Tauri 2 |
| Browser extension | TypeScript, WebExtension Manifest V3 |
| JS tooling | pnpm workspaces |

Why: one Rust engine compiles for every target including Android; one web frontend is reused by the web UI, the desktop app and the Android app; the extension and the UI share a language.

reqwest runs with `default-features = false` and features `rustls` (aws-lc-rs crypto, OS certificate store), `http2`, `system-proxy`. See `docs/decisions/0002`.

## Architecture

```
browser extension ──HTTP──┐
web UI (browser) ─────────┤
desktop app (Tauri) ──────┼──► crates/api (REST + WebSocket) ──► crates/core (engine)
Android app (Tauri) ──────┘    (on Android, core + api run inside the app)
```

- **The HTTP API is the only contract.** Every client talks to the engine through it. Tauri commands are only for OS integration (tray, notifications, file pickers, open folder, autostart), never for download operations.
- **One binary:** `ottie serve` runs the daemon (core + api); other `ottie` subcommands are thin clients of the HTTP API. See `docs/decisions/0004`.
- **Desktop:** the daemon is its own process (`ottie serve`), started by the app as a Tauri sidecar if none is answering, or installed as a service. The desktop app can also connect to a remote daemon (e.g. on a NAS) by URL + token.
- **Android:** no separate daemon. The app embeds `core` + `api` and serves the API on 127.0.0.1, so the UI code is unchanged.
- **Headless:** the daemon runs standalone (systemd user service, Docker) and serves the built web UI itself.

## Repo layout

Library packages are prefixed `ottie-` (a package named `core` would clash with Rust's built-in crate). See `docs/decisions/0003`.

```
crates/core/       ottie-core: engine library. No HTTP server, no UI.       [exists]
  src/manager.rs     Manager handle + the actor task that owns all jobs, queue
  src/job.rs         JobState + state machine (can_become), JobSnapshot, JobEvent
  src/id.rs          JobId (yyyy.mm.dd-hh.mm.ss-N, UTC) and its generator
  src/backend.rs     Backend trait, HttpBackend
  src/store.rs       SQLite on its own thread: schema migrations, save/load jobs
  src/http.rs        HTTP transfer: request, resume, stream to .part.ottie
  src/download.rs    Download state (what gets persisted), Event, Outcome
  src/filename.rs    sanitizing, collision-free names, .part.ottie → final rename
  tests/support/     in-process axum test server with fault injection
  examples/fetch.rs  manual CLI try-out
crates/api/        ottie-api: axum router: REST + WebSocket events, auth, static UI serving
crates/ottie/      ottie: the binary. `ottie serve` = config, logging, wiring core + api;  [exists, 0.0.1 stub]
                   other subcommands = API clients
app/               ottie-app: Tauri 2 shell (desktop + Android), depends on core + api
ui/                Svelte frontend shared by all front ends
extension/         WebExtension (Chrome + Firefox)
shared/api-types/  TypeScript types generated from Rust (ts-rs). Never edit by hand.
docs/decisions/    one short file per accepted decision (NNNN-title.md)       [exists]
```

## Commands

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
cargo run -p ottie-core --example fetch -- <url> [folder]   # Ctrl+C pauses, Enter resumes
cargo publish -p ottie --dry-run     # check a crates.io release (Mathieu does the real publish)

# planned, not scaffolded yet:
cargo run -p ottie -- serve          # daemon on 127.0.0.1
pnpm install
pnpm --filter ui dev                 # UI against a running daemon
pnpm --filter ui check               # svelte-check + tsc
pnpm --filter extension build
cargo tauri dev                      # from app/: desktop
cargo tauri android dev              # from app/: Android device/emulator
```

Before calling a task done: fmt, clippy, tests for the crates you touched, `check` for the TS packages you touched. Update this section when a part comes to exist.

Dev machine (Windows): Rust via rustup (stable, MSVC target) plus VS 2022 Build Tools with the C++ workload. Shells started before that install may lack `~/.cargo/bin` on `PATH`. Node 24 is installed; pnpm isn't yet.

## Engine rules (crates/core)

- **One owner of job state.** The actor task in `manager.rs` owns all jobs; the cloneable `Manager` handle sends it commands over an `mpsc` channel, each with a `oneshot` reply. Job tasks report back on an internal channel (progress at most every 250 ms). Events go out on a `broadcast` channel: `Changed(JobSnapshot)` on every state change (clients upsert by ID), plus `Progress` and `Restarted`. A subscriber that lags misses events and should resync with `list()`. No shared mutable job state behind ad-hoc locks.
- **Explicit state machine:** Queued → Connecting → Downloading → Paused | Completed | Failed | Cancelled; Paused/Failed → Queued on resume. The rules live in `JobState::can_become`, and every change goes through `Actor::transition`; invalid transitions are errors.
- **Pause/cancel of a running job** answers only once the task has really stopped and flushed. If the job completes first, it stays Completed. Cancel deletes the `.part.ottie`. Shutdown stops every running job, waits for them and flushes the database. Lowering `max_concurrent` never stops running jobs.
- **Job IDs:** see `docs/decisions/0006`. IDs only ever increase; `BTreeMap<JobId, _>` order = creation order = queue order.
- **Probe first:** learn size, `Accept-Ranges`, `ETag`/`Last-Modified` and the final URL after redirects before splitting into segments.
- **Resume safely:** `Range` + `If-Range` with a strong ETag (weak `W/` ETags are invalid in `If-Range`), else `Last-Modified`. No validator, a 200 instead of 206, or a `Content-Range` that doesn't match → restart from zero and emit `Event::Restarted(reason)`. Never splice bytes from two versions of a file.
- **The `.part.ottie` length is the truth** for the resume offset. Bytes in flight when a connection dies are lost, so flush on every exit path (pause, error, completion) to keep `downloaded` equal to the file length.
- **Resume re-requests the original URL,** not the redirect target: signed CDN links expire, and `If-Range` still guards consistency.
- **Request `Accept-Encoding: identity`** so byte ranges map to file offsets.
- **Throttling:** hierarchical token buckets, global → category → download. Every chunk read takes tokens from all three. All segments of a download share its bucket. Limits change live; 0 = unlimited.
- **Persistence** (`docs/decisions/0008`): SQLite holds jobs, and later categories, settings and per-segment progress. `STRICT` tables, numbered migrations in `store.rs` (never edit a released one). Saves happen on add, on every state change (inside `transition`), on `Event::Started` (file name + validator known) and progress at most every 2 s. **From 2c: flush file data before saving the progress that covers it.** On load, running jobs become Queued and `downloaded` is reset from the `.part.ottie` length.
- **Shutdown keeps running jobs running** in the database (it doesn't mark them Paused), so the next start continues them; user-paused jobs stay paused.
- **Files:** write to `<name>.part.ottie` (`PART_SUFFIX`, see `docs/decisions/0007`), rename on completion. Sanitize filenames from URLs and `Content-Disposition`; never write outside the target folder; resolve collisions with ` (1)`, ` (2)`. Reserve names with `create_new` so it's atomic. Close the file before renaming (Windows refuses to rename open files).
- **Cancellation:** `tokio_util::sync::CancellationToken`, raced against network reads with `tokio::select! { biased; … }`. Pausing must stop all segments promptly and leave state consistent.
- **Backends:** downloads go through the `Backend` trait (`backend.rs`); the manager is generic over it. The native HTTP(S) backend comes first; aria2 (FTP, BitTorrent, Metalink) and yt-dlp may plug in later, so keep HTTP assumptions inside the HTTP backend.

## Platform gotchas

- **TLS:** rustls everywhere. No OpenSSL/native-tls; it breaks Android and Windows builds. On Android, `rustls-platform-verifier` must be initialised with the app's JNI context and ships a small Kotlin component. Check its docs in step 6.
- **Android background work:** downloads must keep going when the app is in the background. Android 15 limits `dataSync` foreground services to 6 hours per 24 h; prefer user-initiated data transfer jobs. This needs a small Kotlin Tauri plugin. Check current Android docs before implementing.
- **Android storage:** the engine needs a seekable file. Write `.part.ottie` files to app storage and publish finished files to Downloads through MediaStore.
- **Android browser integration:** no reliable extension route (Chrome for Android has no extensions; Firefox for Android only partly supports the downloads API). Take URLs from the share sheet and "Open with" intent filters instead.
- **Extension:** Chrome and Firefox downloads APIs differ; keep browser-specific code behind a small adapter. Chrome can hide its own download UI with `downloads.setUiOptions` (needs the `downloads.ui` permission). If the daemon is unreachable, let the browser download normally. Never drop a download.

## Security

- The daemon binds 127.0.0.1 by default. Remote access is opt-in, always requires the token, and should sit behind a reverse proxy or VPN rather than be exposed directly.
- Every API request carries the token.
- Cookies and auth headers forwarded by the extension belong to that one job: never log them, never reuse them for another job.
- Logs and errors never contain query strings or credentials: log URLs through `redact()`, and wrap `reqwest::Error` with `without_url()` (done in `Error::from`).
- Only `http`/`https` (plus schemes a backend explicitly supports). The redirect policy in `build_client()` rejects anything else, and caps redirects at 10.

## Code conventions

- Rust: `thiserror` in library crates, `anyhow` only in binaries. `tracing` for logs. No `unwrap()`/`expect()` outside tests unless the invariant is explained in a comment. Clippy enforces this (`[workspace.lints]` + `clippy.toml`). Integration test files start with `#![allow(clippy::unwrap_used, clippy::expect_used)]`.
- New crates inherit `edition`, `license`, `repository` and `publish` (false) from `[workspace.package]`, and set `[lints] workspace = true`. Only `ottie` sets `publish = true`. Once it depends on `ottie-core`/`ottie-api`, those must be published too, and the dependencies need `path` + `version`.
- License header: published crates' entry files carry the MPL "Exhibit A" notice, because the root `LICENSE` file isn't packaged.
- TypeScript: `strict`, no `any`. API types come only from `shared/api-types/`.
- Tests: engine behaviour is tested against the in-process server in `crates/core/tests/support/` (dropped connections, no range support, slow chunks, changed ETag). Tests never hit the real internet. A resumed download must be byte-identical to a clean one. Don't assert exact byte counts after an injected connection drop; assert `.part.ottie` length == `downloaded` instead. Manager tests subscribe before adding jobs, and wait for several concurrent jobs with `wait_for_all` (any order), never one after another.
- Line endings: LF everywhere (`.gitattributes`).
- Commits: Conventional Commits (`feat(core): …`, `fix(extension): …`), small and focused.
- New dependency: say what it's for and why it beats the alternatives before adding it.

## Working with Mathieu

- Mathieu knows basic Python and is new to Rust and TypeScript. When code introduces a concept for the first time, explain it in two or three sentences in your reply, not in code comments. Already explained: borrowing (`&mut`), `Result`/`?`, async/await + `tokio::select!`, enums with data + `match`, closures (`impl FnMut`), traits + generics, `Arc`, channels (mpsc/oneshot/broadcast) and the actor pattern, `Box`, OS threads vs tokio tasks, SQL migrations. Still to come: `Mutex`, lifetimes, Svelte runes, etc.
- For decisions that are hard to reverse (data model, API shape, crate choices, architecture), present 2–3 options with trade-offs and let him choose. For small implementation details, decide and say what you chose.
- Work in small vertical slices that run end to end. Don't scaffold code ahead of need.
- Show the diff and wait for a go-ahead before committing.
- Record accepted decisions in `docs/decisions/` and update this file when commands, layout or rules change.

## Build order

1. ~~`core`: single-connection HTTP download with pause/resume, plus the test server.~~ Done.
2. Slices: ~~2a job manager + state machine, `Backend` trait, queue~~ → ~~2b SQLite persistence~~ → 2c segments (with an in-order mode for streaming) → 2d throttling → 2e categories → 2f `Content-Disposition` filenames, retry with backoff.
3. `api` + `ottie serve` (including the `/jobs/{id}/stream` endpoint), then `ui` as a web UI. Decide where the database and config live.
4. Browser extension.
5. Tauri desktop: tray, notifications, sidecar daemon.
6. Tauri Android: embedded core, background downloads, share-sheet intake.
7. Later: scheduler, clipboard monitoring, uGet import, aria2 / yt-dlp backends.

## Open decisions

- Extension pairing: copy-paste token vs a native messaging host (zero-config, can also launch the daemon).
- Whether to import uGet's existing queue and categories.
- aria2 / yt-dlp backends: if and when.
