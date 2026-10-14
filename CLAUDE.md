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
- **Play while downloading** (uGet can): a short-lived play link (`POST /jobs/{id}/stream` → `GET /stream/{ticket}/{name}`, no token, range support) serves a partial file, so the browser, VLC/mpv or an Android player can play a video before it finishes, locally or from a remote daemon (`docs/decisions/0029`). The download order follows the player and, in a per-download streaming mode, stays in order and fetches the last chunk early (many MP4s keep their index at the end).
- **Translated UI:** English and French to start (more languages = one JSON file each, in `ui/messages/` and, for the tray, `app/messages/`); daemon messages reach clients as codes they translate. The `ottie` command line and logs stay English.
- uGet features to bring over time: categories (folder + file-type rules), queue with max concurrent downloads, opt-in clipboard monitoring, batch/URL-list import, scheduler.

## Stack

| Part | Tech |
|---|---|
| Download engine, daemon, API | Rust (edition 2024), tokio, reqwest 0.13, axum, rusqlite (`bundled`), utoipa (OpenAPI) |
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

- **The HTTP API is the only contract.** Every client talks to the engine through it. It's documented in `docs/api/` (`docs/decisions/0030`): a guide, and `openapi.json` generated from the handlers. Tauri commands are only for OS integration (tray, notifications, file pickers, open folder, autostart, the player), never for download operations. The app's own commands are listed in `app/build.rs` and granted to the daemon's pages in `PAGE_PERMISSIONS`; a page never passes a program or path to run.
- **One binary:** `ottie serve` runs the daemon (core + api); other `ottie` subcommands are thin clients of the HTTP API. See `docs/decisions/0004`.
- **Desktop** (`docs/decisions/0017`): the daemon is its own process. The app starts it as a Tauri sidecar (`ottie serve --managed`: prints `ready`, stops when its stdin closes) unless one already answers (a terminal, or installed as a service), and its window shows the daemon's own page, signed in via `ottie link`. Closing the window hides it to the tray unless a setting says quit (`docs/decisions/0034`). A right click on the tray icon opens Ottie's own menu, the daemon's `/tray` page in a small borderless window; the app also follows the daemon itself (events WebSocket + the usual endpoints) for the system's menu (Linux, daemon unreachable), the tooltip and notifications (finished: all at once by default, or each, or never; failed: a switch; `docs/decisions/0035`), with its own texts in `app/messages/`. Later the app can also connect to a remote daemon (e.g. on a NAS) by URL + token.
- **Android:** no separate daemon. The app embeds `core` + `api` and serves the API on 127.0.0.1, so the UI code is unchanged.
- **Headless:** the daemon runs standalone (systemd user service, Docker) and serves the built web UI itself.

## Repo layout

Library packages are prefixed `ottie-` (a package named `core` would clash with Rust's built-in crate). See `docs/decisions/0003`.

```
crates/core/       ottie-core: engine library. No HTTP server, no UI.       [exists]
  src/manager.rs     Manager handle + the actor task that owns all jobs, queue
  src/job.rs         JobState + state machine (can_become), JobSnapshot, JobEvent, Reorder
  src/id.rs          JobId (yyyy.mm.dd-hh.mm.ss-N, UTC) and its generator
  src/category.rs    CategoryId (General = 1), CategorySettings, extension matching
  src/backend.rs     Backend trait, HttpBackend
  src/store.rs       SQLite on its own thread: schema migrations, save/load jobs and settings
  src/settings.rs    Settings, RetryPolicy, Appearance, language, stream_media, max_batch; SettingsChange (applied all at once)
  src/view.rs        saved views: ViewSettings (filters + sort/grouping), ViewId, normalizing
  src/links.rs       the links in pasted text (find), LinkPreview: for adding several at once
  src/sequence.rs    sequences in one link, curl-style ([001-120], [a-z], {a,b}): count, then expand
  src/throttle.rs    speed limits: per-job token buckets, refill task, priority-weighted sharing, category caps
  src/http.rs        HTTP transfer: first request, single-connection stream, resume, restart rules, stall timeout
  src/segmented.rs   multi-connection transfer: coordinator, connection tasks, dynamic splitting
  src/download.rs    Download state (what gets persisted), Placement, Event, Outcome
  src/problem.rs     Problem: why a job failed, as a kind + details (English via Display)
  src/stream.rs      Reader: a job's bytes at any offset, waiting until they're on disk (play links)
  src/playback.rs    where players read each job (Playback, one head per reader) and whether it's in
                     streaming mode, for segmented.rs; which extensions are media
  src/filename.rs    names from URL / Content-Disposition, sanitizing, collision-free names, .part.ottie → final rename,
                     relocate (a job's file into another folder; copy + delete between drives)
  tests/support/     in-process axum test server with fault injection
  examples/fetch.rs  manual CLI try-out
crates/api/        ottie-api: axum router under /api/v1 (static UI serving from 3b)  [exists]
  src/lib.rs         router(manager, token, shutdown), AppState, JSON 404 fallback; routes come
                     from each handler's #[utoipa::path] (utoipa-axum), openapi()
  src/openapi.rs     the OpenAPI description: info, token scheme, 401/503 everywhere,
                     GET /openapi.json; its test writes docs/api/openapi.json
  src/auth.rs        Token, Bearer / WebSocket-protocol check (constant-time)
  src/types.rs       wire types (camelCase JSON) + ts-rs export, From<engine types>
  src/jobs.rs        GET/POST /jobs, POST /jobs/{id}/pause|resume|cancel|category,
                     PATCH /jobs/{id} (own speed limit, priority, streaming mode), DELETE /jobs/{id},
                     POST /jobs/reorder (move waiting jobs in the queue), POST /jobs/batch (several at
                     once), POST /jobs/preview (the links in some text; both take bodies up to 64 MB)
  src/categories.rs  GET/POST /categories, PUT/DELETE /categories/{id}, POST /categories/match
  src/settings.rs    GET/PATCH /settings: max concurrent, global limit, retry policy, shared
                     appearance + language, setupDone, streamMedia, maxBatch; the ranges people may choose (its tests
                     write them to shared/api-types/Ranges.ts)
  src/views.rs       GET/POST /views, PUT/DELETE /views/{id} (saved views)
  src/stream.rs      play links: POST /jobs/{id}/stream → GET /stream/{ticket}/{name} (no token, Range)
  src/events.rs      GET /events WebSocket: changed / progress / removed / categoryChanged /
                     categoryRemoved / settingsChanged / viewChanged / viewRemoved / resync
  src/error.rs       engine Error → status + {"error": "…"}
  src/ui.rs          the web UI: rust-embed of ui-dist/ (from disk in debug), --ui-dir, SPA fallback
  ui-dist/           built UI (git-ignored, but packaged for crates.io)
  tests/api.rs       end to end: real server + small file server, reqwest + tungstenite
crates/ottie/      ottie: the binary  [exists]
  src/serve.rs       `ottie serve`: logging, folders, token, database, General → Downloads folder,
                     axum::serve, graceful stop
  src/open.rs        `ottie open`: browser on the UI, signed in (via a private redirect page); `ottie link`
  src/config.rs      folders (OS defaults / flags / OTTIE_* env), config.toml, listen address
  src/token.rs       token file: created on first use, `ottie token` prints it
  tests/cli.rs       runs the real binary
app/               ottie-app: Tauri 2 shell (desktop; Android later, then with core + api)  [exists]
  src/main.rs        frameless window on the daemon's page (it draws its own title bar, 0019),
                     single instance, closing hides it (or quits, per app.json), stops the daemon it
                     started on exit
  src/daemon.rs      find or start `ottie serve --managed` (the sidecar next to the app), ready / stop
  src/client.rs      the app as an API client: events WebSocket, jobs, settings, pause/resume, limit
  src/status.rs      follows the events (reconnects), sums them up once a second for the tray (counts, speed, progress of those of known size),
                     and passes downloads that finished or failed to notify.rs
  src/notify.rs      notifications (0035): what to tell (each, all at once, failed), a click opens the window
  src/tray.rs        tray icon (white/black by taskbar, size by scale), clicks, the system's menu and its
                     actions (Linux, or while the daemon is unreachable) (0034)
  src/popup.rs       Ottie's own tray menu: the /tray page in a borderless window beside the click, hidden
                     on blur; commands menu_ready / show_main / quit_app
  src/texts.rs       the tray's and notifications' texts from messages/{en,fr}.json, speeds, counts and
                     failure reasons per language
  src/prefs.rs       app.json: close to tray, notifications, the page's language, last total limit; commands
                     desktop / set_close_to_tray / set_notify / set_notify_failed / set_language
  src/config.rs      load/save the app's JSON settings files in its config folder
  src/player.rs      "Play with" (0031): system's player via a .m3u, browser, or a picked program;
                     saved in the app's config folder; commands player / set_play_with / choose_player
  messages/          the tray's and notifications' texts, one JSON file per language
  build.rs           lists the app's commands, so Tauri generates their permissions (allow-…)
  tauri.conf.json    product, identifier, icons; tauri.bundle.json adds the sidecar for installers only
  scripts/          sidecar.mjs (UI + release ottie → binaries/), ico.mjs (icons/icon.ico from res/icons)
  package.json       pnpm package `app`: the Tauri CLI
ui/                SvelteKit SPA shared by all front ends (pnpm package `ui`)  [exists]
  src/routes/        +layout.svelte (fonts, app.css, theme, favicon, token → Login | Shell; /tray without),
                     +layout.ts (ssr = false), +page.svelte (Dashboard), downloads/+page.svelte
                     (Downloads, ?status= filter), categories/+page.svelte, settings/+page.svelte,
                     tray/+page.svelte (the desktop app's tray menu: TrayMenu)
  src/lib/api.ts     fetch + WebSocket client for /api/v1 (Unauthorized, ApiFailure)
  src/lib/session.svelte.ts  live job list: events WebSocket, reconnect, buffered resync
  src/lib/context.ts what every page shares (session, run): Svelte createContext
  src/lib/jobs.ts, categories.ts, status.ts, selection.ts, bulk.ts, filetypes.ts, speed.ts,
                     filters.ts (Downloads filters ↔ address, quick filters, the four status groups),
                     arrange.ts (sort + group by status, saved per device),
                     details.ts (the selected download's panel: main action, facts, priorities),
                     filemap.ts (ranges → a row's split bar, and the detail bar that maps the file, 0027),
                     views.ts (saved views ↔ filters + arrangement, which pill is lit), format.ts, token.ts, adding.ts (the add step), setup.ts (starter categories),
                     theme.ts (light/dark/system + palette → theme, saved per device),
                     settings.ts (settings fields: kB/MB/GB per second, seconds, ranges; last limit
                     per device), sync.ts (following the shared look + language),
                     playlist.ts (a play link as a .m3u, saved through the browser, 0031),
                     tray.ts (the tray menu's status line, what Pause all / Resume all act on),
                     queue.ts (queue order, up next, whether up/down would move anything, what a drag
                     carries and where it may drop, 0032),
                     batch.ts (several links at once: when a paste opens the list, its rows, what to
                     add, what was refused, 0036)
                     plain logic, unit-tested (*.test.ts, Vitest)
  src/lib/appearance.svelte.ts  the appearance chosen on this device (state shared by layout, shell
                     (follows the shared one, docs/decisions/0022) and Settings)
  src/lib/window.ts  the desktop app: its window (minimize/maximize/close), the folder picker,
                     and how Play opens a player (the app's commands, 0031)
  src/lib/explain.ts   daemon codes (Problem, Reason) → the page's language; English fallback
  src/lib/testing.ts   test helpers (sample job)
  src/lib/components/  Shell (TitleBar with NavMenu tabs + SpeedChip (limit popup: LimitControl + RatePresets), page,
                       dialog; Setup on first start),
                       Dashboard (AddCard, tiles, SpeedChart, RunningList, JobSummary),
                       Downloads (AddCard, then one pane: Toolbar + MoveMenu, QuickFilters (with the
                       saved views' pills + their menu) or FilterPanel, ViewDialog (name a view),
                       JobTable (group rows, sortable headers, drag in the Waiting group) + JobRow,
                       JobDetails under it for the one selected download (its bar maps the file)),
                       CategoriesPage (LinkTester, rows, CategoryEditor inline in place of a row),
                       SettingsPage (section list; appearance + sync, language: LanguagePicker; downloads;
                       desktop app, notifications + playing (desktop app only); retries; connection + sign-in link), Switch, NumberField, LimitControl,
                       TitleBar (+ WindowControls), AddCard (AddBar: link, then category + folder step;
                       BatchDialog: several links, from a paste, the list button or a file),
                       FileIcon, ConfirmDialog, Login, TrayMenu (the desktop app's tray menu)
  messages/{en,fr}.json  every UI text, per language (Paraglide); project.inlang/ its settings
  src/app.css        the four themes as variables under [data-theme] (docs/decisions/0021), fonts, radii
  vite.config.ts     SvelteKit + adapter-static → ../crates/api/ui-dist; Paraglide; dev proxy /api → daemon
extension/         WebExtension (Chrome + Firefox)
shared/api-types/  TypeScript types generated from Rust (ts-rs, by `cargo test -p ottie-api`;
                   output folder set in .cargo/config.toml), plus Ranges.ts (settings' ranges).
                   Never edit by hand.  [exists]
docs/api/          README.md (how to talk to the daemon) + openapi.json (generated) [exists]
docs/decisions/    one short file per accepted decision (NNNN-title.md), and README.md,
                   one line per decision                                      [exists]
res/icons/         the drawn Ottie icons (app, tray), PNG + SVG               [exists]
```

## Commands

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
cargo run -p ottie-core --example fetch -- <url> [folder]   # Ctrl+C pauses, Enter resumes
cargo publish --workspace --dry-run  # check a crates.io release of core → api → ottie (the maintainer publishes)
cargo run -p ottie -- serve          # daemon on 127.0.0.1:24068 (OTTIE_LOG=debug for more logs)
cargo run -p ottie -- token          # print the API token
cargo test -p ottie-api              # also regenerates shared/api-types/*.ts and docs/api/openapi.json: commit them
cargo run -p ottie -- open           # the web UI in a browser, signed in
cargo run -p ottie -- link           # print that signed-in address

pnpm install                         # JS dependencies (workspace: ui, shared/api-types)
pnpm --filter ui dev                 # UI with live reload on :5173, /api forwarded to the daemon
pnpm --filter ui build               # → crates/api/ui-dist (then rebuild ottie to embed it)
pnpm --filter ui check               # svelte-check + tsc (fails on warnings)
pnpm --filter ui test                # Vitest unit tests
pnpm --filter ui lint                # Prettier check (`format` fixes)
pnpm --filter ui i18n                # recompile messages/*.json (check, dev, build and test do it too)
pnpm --filter app dev                # desktop app (builds the UI and ottie first; starts it unless one answers)
pnpm --filter app build              # installer: UI → release ottie → app (target/release/bundle/)
pnpm --filter app icon               # rebuild app/icons/icon.ico after changing res/icons

# planned, not scaffolded yet:
pnpm --filter extension build
pnpm --filter app tauri android dev  # Android device/emulator
```

Before calling a task done: fmt, clippy, tests for the crates you touched; for TS packages you touched, `check`, `test` and `lint`. A release builds the UI before the Rust binary. Update this section when a part comes to exist.

Development on Windows: Rust via rustup (stable, MSVC target) plus VS 2022 Build Tools with the C++ workload. Shells started before that install may lack `~/.cargo/bin` on `PATH`. Node 24 (nvm-windows) and pnpm 12 (`npm install -g pnpm`) are installed. The workspace includes the Tauri app, so `cargo build/test/clippy --workspace` compile Tauri too (WebView2 comes with Windows 11; Linux needs the webkit2gtk 4.1 dev packages, and libayatana-appindicator3 for the tray). In a dev run, set `OTTIE_CONFIG_DIR`/`OTTIE_DATA_DIR`/`OTTIE_PORT` to keep away from your real downloads: the app passes them on to the daemon.

## Engine rules (crates/core)

- **One owner of job state.** The actor task in `manager.rs` owns all jobs; the cloneable `Manager` handle sends it commands over an `mpsc` channel, each with a `oneshot` reply. Job tasks report back on an internal channel (progress at most every 250 ms). Events go out on a `broadcast` channel: `Changed(JobSnapshot)` on every state change (clients upsert by ID), plus `Progress` (multi-connection: each range as received so far, `docs/decisions/0025`) and `Restarted`. A subscriber that lags misses events and should resync with `list()`. No shared mutable job state behind ad-hoc locks.
- **Explicit state machine:** Queued → Connecting → Downloading → Paused | Completed | Failed | Cancelled | Retrying; Retrying → Connecting | Queued (gave way) | Paused | Cancelled; Paused/Failed → Queued on resume. The rules live in `JobState::can_become`, and every change goes through `Actor::transition`; invalid transitions are errors.
- **Pause/cancel of a running job** answers only once the task has really stopped and flushed. If the job completes first, it stays Completed. Cancel deletes the `.part.ottie`. Remove (`docs/decisions/0018`) works in any state: cancel unless completed, then forget the job (a completed file stays); the highest ID handed out is saved so a removed job's ID never comes back. Shutdown stops every running job, waits for them and flushes the database. Lowering `max_concurrent` never stops running jobs.
- **Job IDs:** see `docs/decisions/0006`. IDs only ever increase; `BTreeMap<JobId, _>` order = creation order = queue order.
- **Probe with the first request:** it asks for `bytes=0-`; a `206` proves ranges work, and that response becomes the first range. Split only with a 206, a validator, no `Accept-Ranges: none` and at least 2 × `min_split` bytes (`docs/decisions/0009`); otherwise one connection.
- **Multi-connection** (`segmented.rs`): the coordinator owns the ranges; connections share `end`/`pos`/`durable` atomics with it. Full-size `.part.ottie` up front. Extra requests are closed ranges with `If-Range`. A changed file restarts the run from zero at most once. Idle connections take an unfinished range, else half of the busy range with the most left. A connection failing with a retryable error while others work doesn't stop the run: its range waits for a free connection and the run allows one fewer; the run fails only when the last one does (`docs/decisions/0033`). A connection the limit holds back over `HttpConfig::starved_after` (20 s) ends its request, and a connection waits for budget before requesting, so servers never wait on an unread request. Known cost: a connection whose range ends (or is split) discards what it had buffered past the end, up to ~400 KiB (hyper's read buffer fills while throttled). That's <1% on large files but ~20% on a 2 MiB one; it counts against speed limits. Possible fix later: re-request the first range closed once the size is known. While players read (`Playback`, `docs/decisions/0029`), every 200 ms each point they need next (from each head, and the last 1 MiB) gets its own range unless a connection is within `min_split` of it; a free connection takes it first, else the busy one serving no player with the most left stops where it is. Connections then also hand their bytes to the system every 200 ms (no `sync_data`: a disk sync can take seconds on Windows) and readers see those ranges through `Playback`; only synced ranges are saved. **Streaming mode** (a job's `streaming`, live through `Playback`): the same as if a player read from the start, and idle connections take the next range in order, cut to a piece (~2 s of one connection at the measured speed, `HttpConfig::piece_time`, at least 4 × `min_split`); busy ranges over 4 pieces are cut to one. New jobs get it from `NewJob::streaming`, else for media extensions if `Settings::stream_media`, decided again by the real name at the first response unless `streaming_chosen`.
- **Resume safely:** `Range` + `If-Range` with a strong ETag (weak `W/` ETags are invalid in `If-Range`), else `Last-Modified`. No validator, a 200 instead of 206, or a `Content-Range` that doesn't match → restart from zero and emit `Event::Restarted(reason)`. Never splice bytes from two versions of a file.
- **Single connection: the `.part.ottie` length is the truth** for the resume offset (multi-connection: the flushed ranges are). Bytes in flight when a connection dies are lost, so flush on every exit path (pause, error, completion) to keep `downloaded` equal to the file length.
- **Resume re-requests the original URL,** not the redirect target: signed CDN links expire, and `If-Range` still guards consistency.
- **Request `Accept-Encoding: identity`** so byte ranges map to file offsets.
- **Speed limits and priorities** (`docs/decisions/0010`): one token bucket per running job, shared by its connections, spent after each chunk (pause still interrupts the wait). A 50 ms refill task shares the global limit: Urgent first, then weights High 4 / Normal 2 / Low 1, capped by each job's own limit, unused share redistributed (`share()`). Per download, not per connection. A category's limit caps its running jobs together (`allocate()`). Limits change live; 0 = unlimited. A running job that owes budget and gets none for 1 s is reported to the manager (`Limiter::new`'s callback), which sends `waiting_for_bandwidth` in a `Changed` (not saved).
- **Several at once** (`docs/decisions/0036`): `Manager::add_many` adds each job as `add` would (the good ones even if others are refused), saves them in one transaction and schedules once; more than `Settings::max_batch` refuses them all (`TooManyLinks`). `Manager::preview` finds the links in text (`links::find`, in the caller's task) and tells, for each, its name, the category it would get and whether a job has that URL already. Sequences in a link (`docs/decisions/0037`, `sequence.rs`) give each link they hold; `find` counts them before making any, so the cap refuses a huge one at once.
- **Queue order** (`docs/decisions/0032`): highest priority first, then the lowest `place` (new jobs go last). Urgent waits for a free slot (no preemption). `Manager::reorder(ids, Reorder)` (up, down, top, bottom, before/after a job) moves waiting (queued/paused) jobs within their priority by swapping places among them, so other jobs keep theirs; each moved job is saved and gets a `Changed`.
- **Retries** (`docs/decisions/0013`): only `Error::is_retryable()` errors (network, drop, stall, 5xx, 408, 429). A retrying job keeps its slot for `retries_before_yield` retries, then gives way if a queued job is waiting (queues behind every job that never gave way); fails after `max_retries`. Waits double from `first_wait` up to 60 s; `Retry-After` (seconds) can ask for up to 10 min. The count resets when bytes arrive or on a manual resume. Stalls: `tokio::time::timeout` around each network wait (`HttpConfig::stall_timeout`), never reqwest's `read_timeout` (it would fire while our throttle waits). Each run of a job (start, resume, retry) gets its own reqwest `Client` (`HttpBackend`), so it never reuses a pooled connection that died silently when the network changed (VPN, Wi-Fi); HTTP/2 connections are also pinged after 10 s of silence.
- **Failures are `Problem`s** (`docs/decisions/0016`), never bare strings: a kind plus details, saved in the `problem_*` columns. Add a kind = a migration (CHECK list), a `Display` wording, an API `Problem` variant and UI messages.
- **Settings** live in the single-row `settings` table; `Manager::start(backend, store)` loads them. They change through `Manager::change_settings` (several at once): one save and one `JobEvent::SettingsChanged` per change, none if nothing changed (`docs/decisions/0022`). A job's own settings (speed limit, priority) change the same way through `Manager::change_job(id, JobChange)`: one save and one `Changed` (`docs/decisions/0024`).
- **Categories** (`docs/decisions/0011`, `0020`): every job has one. Two built-ins can't be removed: General (ID 1: unmatched extensions, default folder) and Other (folders of no category; no folder or extensions of its own; ID in `Actor::other`). `add(NewJob)` uses the caller's category, else, if the caller gave a folder, the category whose folder it is (preferring the extension's), else Other; both are fixed (`category_chosen`, never re-matched). Without either: the category owning the URL's extension, else General. Each extension belongs to one category at most. The job's folder is copied at add (caller's, else the category's, else `NoFolder`). Removing a category moves its jobs to General first (saved before the row is deleted); their files stay where they are. Moving a job to another category moves its file to that category's folder (`docs/decisions/0028`): a running job stops (`Stop::Move`, state and slot kept), its file moves in a task of its own (`Job::moving`, which refuses pause/resume/cancel/remove), then it carries on; a job that can't resume is refused while running; a category without a folder leaves the file in place. A category's `max_concurrent` caps its running jobs under the global cap. At the first response, an auto-matched job is re-matched by its real name; its folder follows (via `Placement`, before the file is created) only if it was in its category's folder (`docs/decisions/0012`). Migrations run with foreign keys off and check them before committing. The daemon never creates starter categories itself; the UI's first-start setup offers them (`docs/decisions/0020`).
- **Saved views** (`docs/decisions/0026`): name + filters + sort/grouping, kept by the actor like categories (`Manager::add_view` …, `ViewChanged`/`ViewRemoved`). The engine checks and normalizes them but never filters jobs by them; front ends do. Removing a category replaces it with General in every view (saved before the category row goes).
- **Persistence** (`docs/decisions/0008`): SQLite holds jobs, and later categories, settings and per-segment progress. `STRICT` tables, numbered migrations in `store.rs` (never edit a released one). Saves happen on add, on every state change (inside `transition`), on `Event::Started` (file name + validator known) and progress at most every 2 s. **From 2c: flush file data before saving the progress that covers it.** On load, running jobs become Queued and `downloaded` is reset from the `.part.ottie` length.
- **Shutdown keeps running jobs running** in the database (it doesn't mark them Paused), so the next start continues them; user-paused jobs stay paused.
- **Files:** write to `<name>.part.ottie` (`PART_SUFFIX`, see `docs/decisions/0007`), rename on completion. Names come from `Content-Disposition` (`filename*`, then `filename`), else the final URL; chosen once at the first response. Sanitize them; never write outside the target folder; resolve collisions with ` (1)`, ` (2)`. Reserve names with `create_new` so it's atomic. Close the file before renaming (Windows refuses to rename open files).
- **Cancellation:** `tokio_util::sync::CancellationToken`, raced against network reads with `tokio::select! { biased; … }`. Pausing must stop all segments promptly and leave state consistent.
- **Backends:** downloads go through the `Backend` trait (`backend.rs`); the manager is generic over it. The native HTTP(S) backend comes first; aria2 (FTP, BitTorrent, Metalink) and yt-dlp may plug in later, so keep HTTP assumptions inside the HTTP backend.

## Platform gotchas

- **TLS:** rustls everywhere. No OpenSSL/native-tls; it breaks Android and Windows builds. On Android, `rustls-platform-verifier` must be initialised with the app's JNI context and ships a small Kotlin component. Check its docs in the Android step (8).
- **Android background work:** downloads must keep going when the app is in the background. Android 15 limits `dataSync` foreground services to 6 hours per 24 h; prefer user-initiated data transfer jobs. This needs a small Kotlin Tauri plugin. Check current Android docs before implementing.
- **Android storage:** the engine needs a seekable file. Write `.part.ottie` files to app storage and publish finished files to Downloads through MediaStore.
- **Android browser integration:** no reliable extension route (Chrome for Android has no extensions; Firefox for Android only partly supports the downloads API). Take URLs from the share sheet and "Open with" intent filters instead.
- **Extension:** Chrome and Firefox downloads APIs differ; keep browser-specific code behind a small adapter. Chrome can hide its own download UI with `downloads.setUiOptions` (needs the `downloads.ui` permission). If the daemon is unreachable, let the browser download normally. Never drop a download.

## Security

- The daemon binds 127.0.0.1:24068 by default (`address`/`port` in config.toml, `--address`/`--port`, `OTTIE_ADDRESS`/`OTTIE_PORT`; flags win over the file). Remote access is opt-in, always requires the token, and should sit behind a reverse proxy or VPN rather than be exposed directly.
- Every API request carries the token (`docs/decisions/0014`): `Authorization: Bearer`, or for the events WebSocket the offered protocol `ottie.token.<token>` (never in a URL or a cookie). The token (32 random bytes, hex) lives in `<config>/token`, created on first use; `ottie serve` mentions where only when stdout is a terminal, never in logs. Unknown API paths also need it.
- Play links (`/stream/{ticket}/…`, `docs/decisions/0029`) are the one exception: the random ticket reads one job's file and expires after 12 h unused. They're served with `Content-Security-Policy: sandbox`, so a downloaded page or SVG can't run scripts as the daemon's pages.
- Cookies and auth headers forwarded by the extension belong to that one job: never log them, never reuse them for another job.
- Logs and errors never contain query strings or credentials: log URLs through `redact()`, and wrap `reqwest::Error` with `without_url()` (done in `Error::from`).
- Only `http`/`https` (plus schemes a backend explicitly supports). The redirect policy in `build_client()` rejects anything else, and caps redirects at 10.

## Code conventions

- Rust: `thiserror` in library crates, `anyhow` only in binaries. `tracing` for logs. No `unwrap()`/`expect()` outside tests unless the invariant is explained in a comment. Clippy enforces this (`[workspace.lints]` + `clippy.toml`). Integration test files start with `#![allow(clippy::unwrap_used, clippy::expect_used)]`.
- New crates inherit `edition`, `license`, `repository` and `publish` (false) from `[workspace.package]`, and set `[lints] workspace = true`. `ottie`, `ottie-core` and `ottie-api` set `publish = true` (the binary depends on the other two, so they're published first); their dependencies on each other need `path` + `version`. The Tauri app stays unpublished.
- License header: published crates' entry files carry the MPL "Exhibit A" notice, because the root `LICENSE` file isn't packaged.
- TypeScript: `strict`, no `any`. API types come only from `shared/api-types/` (`import type { Job } from '@ottie/api-types/Job'`). In the UI, `#lib/…` imports of TS files end in `.js` (`#lib/format.js`). Logic lives in plain `.ts` files with unit tests; components stay thin. Prettier formats (tabs, single quotes).
- UI (`docs/decisions/0015`, `0021`): runes mode; styles in each component's `<style>` using the variables in `app.css` (colours, `--font`/`--mono`, `--radius-s/m/l`), never raw colours; the page talks to the daemon only through `api.ts`. Every theme must keep working: check light and dark when adding colours.
- Translations (`docs/decisions/0016`): no visible text in components or logic: every string is a message in `messages/en.json` *and* `fr.json` (French typography: no-break space before `: ? !` and inside « »); numbers, sizes and durations go through `format.ts` (`Intl`); daemon text is shown through `explain.ts`. Functions that build text take a `locale` parameter (default `getLocale()`) so tests can check both languages.
- API docs (`docs/decisions/0030`): every handler has a `#[utoipa::path]` (route, body, answers, the `Reason` codes it can refuse with) and is registered with `routes!`; wire types derive `ToSchema` beside `TS`. Fields inside the tagged enums (`Problem`, `Reason`, `Event`) stay one word (utoipa ignores `rename_all_fields`; a test checks). A change in how to use the API also updates `docs/api/README.md`.
- API errors (`docs/decisions/0016`): every refusal has a `Reason` code (`crates/api/src/types.rs`) besides the English `error`; a new engine `Error` variant gets a code in `error.rs`.
- Tests: engine behaviour is tested against the in-process server in `crates/core/tests/support/` (dropped connections, no range support, slow chunks, changed ETag, `unplug()`: open connections go silent as after a network change). Speed-limit tests (`tests/throttle.rs`) assert shares and minimum durations with wide margins, never exact speeds; progress snapshots lag up to ~0.5 s, so don't measure rates over short windows. Multi-connection tests (`tests/segmented.rs`) use `HttpConfig { connections: 4, min_split: 64 KiB }`; the other test files pin one connection or use files too small to split. Tests never hit the real internet. A resumed download must be byte-identical to a clean one. Don't assert exact byte counts after an injected connection drop; assert `.part.ottie` length == `downloaded` instead. Manager tests subscribe before adding jobs, and wait for several concurrent jobs with `wait_for_all` (any order), never one after another.
- Line endings: LF everywhere (`.gitattributes`).
- Commits: Conventional Commits (`feat(core): …`, `fix(extension): …`), one per slice: the code, its tests and its docs (decision record + its line in `docs/decisions/README.md`, CLAUDE.md) together. Unrelated changes (e.g. a test fix found along the way) get their own.
- New dependency: say what it's for and why it beats the alternatives before adding it.

## How we work

- For decisions that are hard to reverse (data model, API shape, crate choices, architecture), present 2–3 options with trade-offs and let the maintainer choose. For small implementation details, decide and say what you chose.
- Work in small vertical slices that run end to end. Don't scaffold code ahead of need.
- Show the diff and wait for a go-ahead before committing.
- Record each accepted decision in `docs/decisions/` (one short file, `NNNN-title.md`) and add a line for it to `docs/decisions/README.md`, which sums up every decision. Update this file when commands, layout or rules change.

## Build order

Done: ✅ and struck through.

1. ✅ ~~`core`: single-connection HTTP download with pause/resume, plus the test server.~~
2. ✅ Engine slices:
   - ✅ ~~2a job manager + state machine, `Backend` trait, queue~~
   - ✅ ~~2b SQLite persistence~~
   - ✅ ~~2c multi-connection~~ (streaming mode comes in 5f3)
   - ✅ ~~2d speed limits + priorities~~
   - ✅ ~~2e categories~~
   - ✅ ~~2f `Content-Disposition` filenames + category re-match~~
   - ✅ ~~2g retry with backoff~~
3. ✅ Usable app, early:
   - ✅ ~~3a minimal `ottie serve` (jobs API + events WebSocket, token, folders)~~
   - ✅ ~~3b first web UI (SvelteKit SPA built into the binary; list, add, pause/resume/cancel; token link + login page; `ottie open`)~~
   - ✅ ~~3b2 translations (en, fr; Paraglide) + error codes from the daemon~~
   - ✅ ~~3c first desktop app (Tauri window on the same UI, starts the daemon as a sidecar, Ottie icons from `res/icons/`)~~
4. ✅ Features grow through API, UI and desktop app together, one slice each:
   - ✅ ~~4a list layout (sidebar views, remove, clear finished)~~
   - ✅ ~~4b table list with selection + toolbar, own title bar~~
   - ✅ ~~4c left menu (Dashboard, Downloads), status filter in Downloads, dashboard~~
   - ✅ ~~4d categories (API, Categories page, folder picker, Category column, move to category)~~
   - ✅ ~~4e category step when adding + first-start setup with editable starter categories~~
5. The redesign from the mockups (`docs/decisions/0021`), then more features the same way:
   - ✅ ~~5a look (four themes, tabs in the title bar, add card, the four tabs laid out as in the mockups, Settings with appearance + language)~~
   - ✅ ~~5b settings (API + page: max concurrent, global limit with the title-bar chip, retry policy, appearance sync)~~
   - ✅ ~~5c1 Downloads filters (quick-filter pills, Filters and sorting panel, group by status; `docs/decisions/0023`)~~
   - ✅ ~~5c2 detail panel with per-download speed limit + priority (`PATCH /jobs/{id}`, `docs/decisions/0024`)~~
   - ✅ ~~5d per-connection progress + file map (ranges in progress events, `docs/decisions/0025`)~~
   - ✅ ~~5e saved views (stored in the daemon, `docs/decisions/0026`), the detail bar as file map (`0027`), files move with their category (`0028`)~~
   - ✅ 5f play while downloading (`docs/decisions/0029`):
     - ✅ ~~5f1 play links + reader~~
     - ✅ ~~5f2 follow the player (seek, last chunk early)~~
     - ✅ ~~5f3 streaming mode per download (on by itself for videos and music, a setting turns that off)~~
   - ✅ ~~5g queue reorder (`docs/decisions/0032`)~~
   - ✅ ~~5h tray (`docs/decisions/0034`)~~
   - ✅ ~~5i notifications (`docs/decisions/0035`)~~
   - ✅ ~~5j several links at once (`docs/decisions/0036`)~~
   - ✅ ~~5k sequences in one link (`[001-120]`, curl-style; `docs/decisions/0037`)~~
   - 5l autostart
6. Scheduler, clipboard monitoring, uGet import, aria2 / yt-dlp backends.
7. Browser extension.
8. Tauri Android, last, once everything else is done: embedded core, background downloads, share-sheet intake.

## Open decisions

- Extension pairing: copy-paste token vs a native messaging host (zero-config, can also launch the daemon).
- Whether to import uGet's existing queue and categories.
- aria2 / yt-dlp backends: if and when.
