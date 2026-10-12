# CLAUDE.md

Cross-platform download manager meant to replace uGet (unmaintained). Working name: TBD.

## What we're building

- A background **daemon** that does all the downloading, with three front ends on top: a **web UI**, a **desktop app** (Windows, Linux, later macOS) and an **Android app**. iOS is out of scope.
- A **browser extension** (Chromium-based browsers and Firefox, desktop) that takes over downloads from the browser: intercepts them, hides the browser's own download UI where possible, and shows our download list in its popup.
- Must-haves: pause/resume (including after a restart or crash), multi-connection downloads, and speed limits at three levels (per download, per category, global), adjustable while downloads run.
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
- **Desktop:** the daemon is its own process, started by the app as a Tauri sidecar if none is answering, or installed as a service. The desktop app can also connect to a remote daemon (e.g. on a NAS) by URL + token.
- **Android:** no separate daemon. The app embeds `core` + `api` and serves the API on 127.0.0.1, so the UI code is unchanged.
- **Headless:** the daemon runs standalone (systemd user service, Docker) and serves the built web UI itself.

## Repo layout

Cargo packages are prefixed `dm-` (`dm-core`, `dm-api`, `dm-daemon`): a package named `core` clashes with Rust's built-in crate. See `docs/decisions/0001`.

```
crates/core/       dm-core: engine library. No HTTP server, no UI.          [exists]
  src/http.rs        HTTP transfer: request, resume, stream to .part
  src/download.rs    Download state (what gets persisted), Event, Outcome
  src/filename.rs    sanitizing, collision-free names, .part → final rename
  tests/support/     in-process axum test server with fault injection
  examples/fetch.rs  manual CLI try-out
crates/api/        axum router: REST + WebSocket events, auth, static UI serving
crates/daemon/     thin binary: config, logging, wiring core + api
app/               Tauri 2 shell (desktop + Android), depends on core + api
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
cargo run -p dm-core --example fetch -- <url> [folder]   # Ctrl+C pauses, Enter resumes

# planned, not scaffolded yet:
cargo run -p dm-daemon               # daemon on 127.0.0.1
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

- **One owner of job state.** A single manager task owns all job state; everything else sends it commands over channels. Progress goes out as events on a broadcast channel. No shared mutable job state behind ad-hoc locks.
- **Explicit state machine:** Queued → Connecting → Downloading → Paused | Completed | Failed | Cancelled. All transitions go through one function; invalid transitions are errors.
- **Probe first:** learn size, `Accept-Ranges`, `ETag`/`Last-Modified` and the final URL after redirects before splitting into segments.
- **Resume safely:** `Range` + `If-Range` with a strong ETag (weak `W/` ETags are invalid in `If-Range`), else `Last-Modified`. No validator, a 200 instead of 206, or a `Content-Range` that doesn't match → restart from zero and emit `Event::Restarted(reason)`. Never splice bytes from two versions of a file.
- **The `.part` length is the truth** for the resume offset. Bytes in flight when a connection dies are lost, so flush on every exit path (pause, error, completion) to keep `downloaded` equal to the file length.
- **Resume re-requests the original URL,** not the redirect target: signed CDN links expire, and `If-Range` still guards consistency.
- **Request `Accept-Encoding: identity`** so byte ranges map to file offsets.
- **Throttling:** hierarchical token buckets, global → category → download. Every chunk read takes tokens from all three. All segments of a download share its bucket. Limits change live; 0 = unlimited.
- **Persistence:** SQLite holds jobs, categories, settings and per-segment progress. Save progress periodically and on pause/shutdown; on startup, reconcile `.part` files with the DB.
- **Files:** write to `<name>.part`, rename on completion. Sanitize filenames from URLs and `Content-Disposition`; never write outside the target folder; resolve collisions with ` (1)`, ` (2)`. Reserve names with `create_new` so it's atomic. Close the file before renaming (Windows refuses to rename open files).
- **Cancellation:** `tokio_util::sync::CancellationToken`, raced against network reads with `tokio::select! { biased; … }`. Pausing must stop all segments promptly and leave state consistent.
- **Backends:** downloads go through a `Backend` trait (introduce it with the manager in step 2). The native HTTP(S) backend comes first; aria2 (FTP, BitTorrent, Metalink) and yt-dlp may plug in later, so keep HTTP assumptions inside the HTTP backend.

## Platform gotchas

- **TLS:** rustls everywhere. No OpenSSL/native-tls; it breaks Android and Windows builds. On Android, `rustls-platform-verifier` must be initialised with the app's JNI context and ships a small Kotlin component. Check its docs in step 6.
- **Android background work:** downloads must keep going when the app is in the background. Android 15 limits `dataSync` foreground services to 6 hours per 24 h; prefer user-initiated data transfer jobs. This needs a small Kotlin Tauri plugin. Check current Android docs before implementing.
- **Android storage:** the engine needs a seekable file. Write `.part` files to app storage and publish finished files to Downloads through MediaStore.
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
- New crates inherit `edition`/`publish` from `[workspace.package]` and set `[lints] workspace = true`.
- TypeScript: `strict`, no `any`. API types come only from `shared/api-types/`.
- Tests: engine behaviour is tested against the in-process server in `crates/core/tests/support/` (dropped connections, no range support, slow chunks, changed ETag). Tests never hit the real internet. A resumed download must be byte-identical to a clean one. Don't assert exact byte counts after an injected connection drop; assert `.part` length == `downloaded` instead.
- Line endings: LF everywhere (`.gitattributes`).
- Commits: Conventional Commits (`feat(core): …`, `fix(extension): …`), small and focused.
- New dependency: say what it's for and why it beats the alternatives before adding it.

## Working with Mathieu

- Mathieu knows basic Python and is new to Rust and TypeScript. When code introduces a concept for the first time, explain it in two or three sentences in your reply, not in code comments. Already explained: borrowing (`&mut`), `Result`/`?`, async/await + `tokio::select!`, enums with data + `match`, closures (`impl FnMut`). Still to come: traits, `Arc<Mutex<…>>`/channels, lifetimes, Svelte runes, etc.
- For decisions that are hard to reverse (data model, API shape, crate choices, architecture), present 2–3 options with trade-offs and let him choose. For small implementation details, decide and say what you chose.
- Work in small vertical slices that run end to end. Don't scaffold code ahead of need.
- Show the diff and wait for a go-ahead before committing.
- Record accepted decisions in `docs/decisions/` and update this file when commands, layout or rules change.

## Build order

1. ~~`core`: single-connection HTTP download with pause/resume, plus the test server.~~ Done.
2. Segments, throttling, SQLite persistence, categories, queue. Also: job manager + state machine, `Backend` trait, `Content-Disposition` filenames, retry with backoff.
3. `api` + `daemon`, then `ui` as a web UI.
4. Browser extension.
5. Tauri desktop: tray, notifications, sidecar daemon.
6. Tauri Android: embedded core, background downloads, share-sheet intake.
7. Later: scheduler, clipboard monitoring, uGet import, aria2 / yt-dlp backends.

## Open decisions

- Project name and license (then rename the `dm-` prefix if wanted).
- Extension pairing: copy-paste token vs a native messaging host (zero-config, can also launch the daemon).
- Whether to import uGet's existing queue and categories.
- aria2 / yt-dlp backends: if and when.
