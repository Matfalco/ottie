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
- **Translated UI:** in the system's language by default (a device or every device can choose another), English as the fallback. English and French, then languages in batches (EU, Latin America, Asia, Africa; `docs/decisions/0041`): de, es-ES, es-419, it, nl, pl, pt-PT, pt-BR, then the rest of the EU in Latin script (sv, ro, cs, hu, da, fi, sk, hr, sl, lt, lv, et, ga, mt), Greek and Cyrillic (el, bg, uk), Asia (zh-Hans, zh-Hant, ja, ko, id, hi, vi, tr), Africa (sw, am, ha, yo, zu), and right to left, with the whole page mirrored (ar, fa, ur; `docs/decisions/0042`). Each is one JSON file in `ui/messages/` and, for the tray, one in `app/messages/`; daemon messages reach clients as codes they translate. The `ottie` command line and logs stay English.
- uGet features to bring over time: categories (folder + file-type rules), queue with max concurrent downloads, opt-in clipboard monitoring (desktop app, `docs/decisions/0043`), batch/URL-list import, scheduler.

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
- **Desktop** (`docs/decisions/0017`): the daemon is its own process. The app starts it as a Tauri sidecar (`ottie serve --managed`: prints `ready`, stops when its stdin closes) unless one already answers (a terminal, or installed as a service), and its window shows the daemon's own page, signed in via `ottie link`. Closing the window hides it to the tray unless a setting says quit (`docs/decisions/0034`). At sign-in (`0039`) the system starts the app with `--autostart`: hidden in the tray, or, per `app.json`, it only starts `ottie serve` on its own (log in the app's log folder) and quits. A right click on the tray icon opens Ottie's own menu, the daemon's `/tray` page in a small borderless window; the app also follows the daemon itself (events WebSocket + the usual endpoints) for the system's menu (Linux, daemon unreachable), the tooltip and notifications (finished: all at once by default, or each, or never; failed: a switch; `docs/decisions/0035`; with their sounds: the system's, silent, a set of Ottie's or one for each kind, `0038`), with its own texts in `app/messages/`. When everything has finished (`0040`) it can quit, or put the computer to sleep, hibernate it or shut it down after a countdown (a small window on the daemon's `/finish` page, or a notification), chosen every time in Settings or for once from the tray menu or the Downloads page. It can watch the clipboard (`0043`, opt-in): a link copied elsewhere to a file type a category lists is offered in a small window that never takes the focus (the daemon's `/caught` page; Download, More… opens the main window's list, Ignore), or added at once with a notification. Later the app can also connect to a remote daemon (e.g. on a NAS) by URL + token.
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
  src/slot.rs        time slots: SlotSettings (target, days or date, hours, effects), whether one covers
                     a moment and its next change (jiff, the system's time zone), Effects (what the
                     active ones change, highest first)
  src/links.rs       the links in pasted text (find), LinkPreview: for adding several at once
  src/sequence.rs    sequences in one link, curl-style ([001-120], [a-z], {a,b}): count, then expand
  src/throttle.rs    speed limits: per-job token buckets, refill task, priority-weighted sharing, category caps
  src/speed.rs       the last hour of total speed, one sample per second, from each job's smoothed rate (0044)
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
                     PATCH /jobs/{id} (own speed limit, priority, streaming mode, start/stop time),
                     DELETE /jobs/{id},
                     POST /jobs/reorder (move waiting jobs in the queue), POST /jobs/batch (several at
                     once), POST /jobs/preview (the links in some text; both take bodies up to 64 MB)
  src/categories.rs  GET/POST /categories, PUT/DELETE /categories/{id}, POST /categories/match
  src/settings.rs    GET/PATCH /settings: max concurrent, global limit, retry policy, shared
                     appearance + language, setupDone, streamMedia, maxBatch; the ranges people may choose (its tests
                     write them to shared/api-types/Ranges.ts)
  src/views.rs       GET/POST /views, PUT/DELETE /views/{id} (saved views)
  src/slots.rs       GET/POST /slots (with the time zone), PUT/DELETE /slots/{id}, POST /slots/reorder
  src/speed.rs       GET /speed: the last hour of total speed, for the graph (0044)
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
  src/daemon.rs      find or start `ottie serve --managed` (the sidecar next to the app), ready / stop; at
                     sign-in, `ottie serve` on its own
  src/client.rs      the app as an API client: events WebSocket, jobs, settings, pause/resume, limit,
                     categories' file types, links in a text (preview), adding several
  src/status.rs      follows the events (reconnects), sums them up once a second for the tray (counts, speed, progress of those of known size),
                     and passes downloads that finished or failed to notify.rs
  src/notify.rs      notifications (0035): what to tell (each, all at once, failed), with which sound, a click
                     opens the window
  src/sound.rs       notification sounds (0038): the system's, silent, one of Ottie's sets (sounds/, played with
                     rodio) or one for each kind
  src/tray.rs        tray icon (white/black by taskbar, size by scale), clicks, the system's menu and its
                     actions (Linux, or while the daemon is unreachable) (0034)
  src/autostart.rs   starting at sign-in (0039): the system's entry (auto-launch, path quoted;
                     rewritten at start only if it starts another path: security tools pause an app that
                     rewrites it each time), what it starts
                     (tray or daemon only), `--autostart`
  src/popup.rs       Ottie's own tray menu: the /tray page in a borderless window beside the click, hidden
                     on blur; commands menu_ready / show_main / quit_app
  src/finish.rs      when everything has finished (0040): what happens (every time, for once), when the last
                     download ends (fed by status.rs), the countdown (its window on /finish, or a notification),
                     Cancel / now; commands finish / set_finish / set_finish_once / set_countdown /
                     set_countdown_in / set_finish_waits / cancel_finish / finish_now / countdown_ready; the
                     pages hear changes as the `finish` event. OTTIE_POWER_DRY_RUN: the countdown only prints
  src/clipboard.rs   clipboard monitoring (0043): a thread looks twice a second (arboard; on Windows only
                     after the change counter moves, never what password managers mark private), the daemon
                     says which links count, then the window on /caught (in the corner nearest the tray
                     icon, not focusable) or added at once; commands caught / caught_ready / caught_done /
                     caught_more, events `caught` (its page) and `add-links` (the main window)
  src/power.rs       sleep, hibernate, shut down and whether the computer can: the system's calls (windows-sys on
                     Windows, logind via systemctl/busctl on Linux)
  src/texts.rs       the tray's and notifications' texts from messages/<tag>.json (the page's tag, else
                     the bare language), speeds, counts and failure reasons per language; right-to-left
                     ones embedded as such (0042)
  src/prefs.rs       app.json: close to tray, notifications and their sounds, what sign-in starts, what happens
                     when everything has finished, watching the clipboard and what a copied link does, the
                     page's language, last total limit; commands desktop / set_startup / set_close_to_tray /
                     set_watch_clipboard / set_copied / set_notify / set_notify_failed / set_sounds /
                     set_sound / play_sound / set_language; the pages hear changes as the `desktop` event
  src/config.rs      load/save the app's JSON settings files in its config folder
  src/player.rs      "Play with" (0031): system's player via a .m3u, browser, or a picked program;
                     saved in the app's config folder; commands player / set_play_with / choose_player
  messages/          the tray's and notifications' texts, one JSON file per language
  sounds/            Ottie's notification sound sets (Ogg), built into the app; README.md: how each was made,
                     its licence, and how loud a new one should be
  sounds/tools/      Python scripts that made them (synthesis, CC0 sources, loudness, a local page to choose
                     pairs by ear, Ogg export); tools for the maintainer, not part of the build
  build.rs           lists the app's commands, so Tauri generates their permissions (allow-…)
  tauri.conf.json    product, identifier, icons; tauri.bundle.json adds the sidecar for installers only
  scripts/          sidecar.mjs (UI + release ottie → binaries/), ico.mjs (icons/icon.ico from res/icons)
  package.json       pnpm package `app`: the Tauri CLI
ui/                SvelteKit SPA shared by all front ends (pnpm package `ui`)  [exists]
  src/hooks.client.ts  which language the page shows: Paraglide strategies that save nothing (0041)
  src/routes/        +layout.svelte (fonts, app.css, theme, favicon, token → Login | Shell; /tray, /finish
                     and /caught without),
                     +layout.ts (ssr = false), +page.svelte (Dashboard), downloads/+page.svelte
                     (Downloads, ?status= filter), categories/+page.svelte, schedule/+page.svelte,
                     settings/+page.svelte, tray/+page.svelte (the desktop app's tray menu: TrayMenu),
                     finish/+page.svelte (its countdown window: Countdown), caught/+page.svelte (its
                     window for links copied elsewhere: Caught)
  src/lib/api.ts     fetch + WebSocket client for /api/v1 (Unauthorized, ApiFailure)
  src/lib/session.svelte.ts  live job list: events WebSocket, reconnect, buffered resync
  src/lib/context.ts what every page shares (session, run): Svelte createContext
  src/lib/jobs.ts, categories.ts, status.ts, selection.ts, bulk.ts, filetypes.ts,
                     speed.ts (the speed history: the graph's spans, 5 min to 1 h, saved per device, 0044),
                     filters.ts (Downloads filters ↔ address, quick filters, the four status groups),
                     arrange.ts (sort + group by status, saved per device),
                     details.ts (the selected download's panel: main action, facts, priorities),
                     filemap.ts (ranges → the bar that maps the file, in the details and each row, 0027),
                     hint.ts (where an (i)'s explanation goes: above its button, inside the window),
                     views.ts (saved views ↔ filters + arrangement, which pill is lit),
                     languages.ts (each language's name, which are reviewed, the system's closest, the
                     device's choice, 0041; which way it's written, 0042), format.ts, token.ts, adding.ts (the add step), setup.ts (starter categories),
                     theme.ts (light/dark/system + palette → theme and style (rounded or legacy, 0044),
                     saved per device),
                     settings.ts (settings fields: kB/MB/GB per second, seconds, ranges; last limit
                     per device), sync.ts (following the shared look + language),
                     playlist.ts (a play link as a .m3u, saved through the browser, 0031),
                     tray.ts (the tray menu's status line, what Pause all / Resume all act on),
                     queue.ts (queue order, the waiting ones in it, whether up/down would move anything, what a drag
                     carries and where it may drop, 0032),
                     startup.ts (the sign-in choices, for Settings and Setup, 0039),
                     schedule.ts (a download's start and stop time: date-and-time fields ↔
                     milliseconds, "today at 23:00", 0040),
                     slots.ts (time slots: their order, in words, the week view's blocks and drags, conflicts,
                     what applies to all downloads now, the editor's draft ↔ body, 0040),
                     batch.ts (several links at once: when a paste opens the list, its rows, what to
                     add, what was refused, 0036), sounds.ts (the notification sounds' choices: sets, or one for each kind, 0038),
                     finish.ts (when everything has finished: the choices in words, what this computer
                     can't do, the countdown's text, 0040),
                     caught.ts (links copied elsewhere: the window's rows and title, how long it stays,
                     what was added or refused, 0043)
                     plain logic, unit-tested (*.test.ts, Vitest)
  src/lib/appearance.svelte.ts  the appearance chosen on this device (state shared by layout, shell
                     (follows the shared one, docs/decisions/0022) and Settings)
  src/lib/window.ts  the desktop app: its window (minimize/maximize/close), the folder picker,
                     and how Play opens a player (the app's commands, 0031), and what happens when
                     everything has finished (0040), and the clipboard's links (0043)
  src/lib/finish.svelte.ts  that, live in a page: asked once, then the app's `finish` events, the countdown's clock
  src/lib/explain.ts   daemon codes (Problem, Reason) → the page's language; English fallback
  src/lib/testing.ts   test helpers (sample job)
  src/lib/components/  Shell (TitleBar with NavMenu tabs + SpeedChip (limit popup: LimitControl + RatePresets), page,
                       dialog, BatchDialog for links sent by More… in the clipboard's window; Setup on first
                       start, with the sign-in and clipboard choices in the desktop app),
                       Dashboard (AddCard, SpeedChart across the top with its span, then the four groups two
                       by two: running rows filling up, waiting ones in queue order, JobSummary rows),
                       Downloads (AddCard, then one pane: Toolbar + MoveMenu + FinishMenu (desktop app), QuickFilters (with the
                       saved views' pills + their menu) or FilterPanel, ViewDialog (name a view),
                       JobTable (group rows, sortable headers, drag in the Waiting group) + JobRow,
                       JobDetails under it for the one selected download (header bar; its facts, priority and
                       limit beside the bar that maps the file (FileBar, as in each row); a strip for category,
                       folder, streaming, start and stop; at most half the pane, giving way to the list's
                       few rows), or DetailsEmpty in its place for none or several, one line),
                       Hint (an (i) beside a setting: its explanation on hover, click or tap),
                       CategoriesPage (LinkTester, rows, CategoryEditor inline in place of a row),
                       SchedulePage (WeekView: drag a slot or its edges, click to edit; then the
                       slots' rows with SlotEditor inline),
                       SettingsPage (section list; appearance + sync, language: LanguagePicker; downloads;
                       desktop app (sign-in, closing), clipboard, notifications + playing (desktop app only); retries; connection + sign-in link), Switch, NumberField, LimitControl,
                       TitleBar (+ WindowControls; folds to fit: tab icons, slot chip, logo name), AddCard (AddBar: link, then category + folder step;
                       BatchDialog: several links, from a paste, the list button or a file),
                       FileIcon, ConfirmDialog, Login, TrayMenu (the desktop app's tray menu, its RunningList), Countdown (its
                       countdown window before sleep, hibernate or shut down), Caught (its window for links
                       copied elsewhere)
  messages/<tag>.json  every UI text, per language (Paraglide); project.inlang/ its settings (the locales)
  src/app.css        the four themes as variables under [data-theme] (docs/decisions/0021), fonts (Outfit +
                     DM Mono, else the system's; see "Languages and fonts" below, 0041), radii; the
                     rounded style's surfaces and shapes under [data-style='rounded'] (0044)
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

Development on Windows: Rust via rustup (stable, MSVC target) plus VS 2022 Build Tools with the C++ workload. Shells started before that install may lack `~/.cargo/bin` on `PATH`. Node 24 (nvm-windows) and pnpm 12 (`npm install -g pnpm`) are installed. The workspace includes the Tauri app, so `cargo build/test/clippy --workspace` compile Tauri too (WebView2 comes with Windows 11; Linux needs the webkit2gtk 4.1 dev packages, libayatana-appindicator3 for the tray, and libasound2 (ALSA) dev for notification sounds). In a dev run, set `OTTIE_CONFIG_DIR`/`OTTIE_DATA_DIR`/`OTTIE_PORT` to keep away from your real downloads: the app passes them on to the daemon. The dev app still reads and writes the real `app.json` (`%APPDATA%\io.github.matfalco.ottie` on Windows): back it up first. `OTTIE_POWER_DRY_RUN=1` keeps the computer on when trying "when everything has finished".

## Engine rules (crates/core)

- **One owner of job state.** The actor task in `manager.rs` owns all jobs; the cloneable `Manager` handle sends it commands over an `mpsc` channel, each with a `oneshot` reply. Job tasks report back on an internal channel (progress at most every 250 ms). Events go out on a `broadcast` channel: `Changed(JobSnapshot)` on every state change (clients upsert by ID), plus `Progress` (multi-connection: each range as received so far, `docs/decisions/0025`) and `Restarted`. A subscriber that lags misses events and should resync with `list()`. No shared mutable job state behind ad-hoc locks.
- **Explicit state machine:** Queued → Connecting → Downloading → Paused | Completed | Failed | Cancelled | Retrying; Retrying → Connecting | Queued (gave way) | Paused | Cancelled; Paused/Failed → Queued on resume. Scheduled (waiting for its start time): Queued | Paused | Failed | Retrying | running → Scheduled; Scheduled → Queued | Paused | Cancelled. The rules live in `JobState::can_become`, and every change goes through `Actor::transition`; invalid transitions are errors.
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
- **Queue order** (`docs/decisions/0032`): highest priority first, then the lowest `place` (new jobs go last). Urgent waits for a free slot (no preemption). `Manager::reorder(ids, Reorder)` (up, down, top, bottom, before/after a job) moves waiting (queued/scheduled/paused) jobs within their priority by swapping places among them, so other jobs keep theirs; each moved job is saved and gets a `Changed`.
- **Start and stop times** (`docs/decisions/0040`): a job's own `start_at`/`stop_at` (`NewJob`, `JobChange`; saved in ms since 1970, UTC). A start time to come makes it `Scheduled`; a running job stops first (`Stop::Hold`). At its start time it joins the queue at its place; at its stop time it pauses wherever it is. Each time is used once, then cleared (`transition` clears the start time on leaving for the queue or a stop). The actor's loop sleeps until the next time (`Actor::wake_at`, `check_times`), at most `CLOCK_CHECK` (30 s), and compares with the wall clock, so sleep or a clock change can't make it miss one. Resume on a scheduled job queues it now.
- **Time slots** (`docs/decisions/0040`, `slot.rs`): kept by the actor like views (`Manager::add_slot` …, `SlotChanged`/`SlotRemoved`, also when one starts or stops applying), in list order (`place`). `check_times` works out the active ones (`update_slots`; a slot that happens once is removed once over) and what they change (`Effects`: per effect, the highest slot that sets it, for the queue and each category), then applies it: limits through the limiter (`apply_limits`: a slot's, else the setting's), caps in `schedule`/`has_room`, and holds in `settle` (a queued job a slot stops becomes `Scheduled` with `held_by`; a running one gets `Stop::Hold` unless the slot lets it finish; released when no slot holds it and its start time has passed: to the queue, or `Paused` if the slot that held it (`held_by`, saved) ended at its time and keeps them paused (`stay_paused`; `keeps_paused`, `Actor::ended` for one that happened once). Settings are never changed by a slot. A job resumed by hand while held gets `ignores_slots` (saved) until it pauses, fails or ends. Wake-ups include the slots' next changes.
- **Retries** (`docs/decisions/0013`): only `Error::is_retryable()` errors (network, drop, stall, 5xx, 408, 429). A retrying job keeps its slot for `retries_before_yield` retries, then gives way if a queued job is waiting (queues behind every job that never gave way); fails after `max_retries`. Waits double from `first_wait` up to 60 s; `Retry-After` (seconds) can ask for up to 10 min. The count resets when bytes arrive or on a manual resume. Stalls: `tokio::time::timeout` around each network wait (`HttpConfig::stall_timeout`), never reqwest's `read_timeout` (it would fire while our throttle waits). Each run of a job (start, resume, retry) gets its own reqwest `Client` (`HttpBackend`), so it never reuses a pooled connection that died silently when the network changed (VPN, Wi-Fi); HTTP/2 connections are also pinged after 10 s of silence.
- **Failures are `Problem`s** (`docs/decisions/0016`), never bare strings: a kind plus details, saved in the `problem_*` columns. Add a kind = a migration (CHECK list), a `Display` wording, an API `Problem` variant and UI messages.
- **Speed history** (`docs/decisions/0044`): the actor feeds each `Event::Progress` to `SpeedHistory`, which keeps each job's speed as the UI's `nextRate` does (a moving average between reports), and adds them up once a second (an hour, in memory): `Manager::speed_history`.
- **Settings** live in the single-row `settings` table; `Manager::start(backend, store)` loads them. They change through `Manager::change_settings` (several at once): one save and one `JobEvent::SettingsChanged` per change, none if nothing changed (`docs/decisions/0022`). A job's own settings (speed limit, priority) change the same way through `Manager::change_job(id, JobChange)`: one save and one `Changed` (`docs/decisions/0024`).
- **Categories** (`docs/decisions/0011`, `0020`): every job has one. Two built-ins can't be removed: General (ID 1: unmatched extensions, default folder) and Other (folders of no category; no folder or extensions of its own; ID in `Actor::other`). `add(NewJob)` uses the caller's category, else, if the caller gave a folder, the category whose folder it is (preferring the extension's), else Other; both are fixed (`category_chosen`, never re-matched). Without either: the category owning the URL's extension, else General. Each extension belongs to one category at most. The job's folder is copied at add (caller's, else the category's, else `NoFolder`). Removing a category moves its jobs to General first (saved before the row is deleted); their files stay where they are. Moving a job to another category moves its file to that category's folder (`docs/decisions/0028`): a running job stops (`Stop::Move`, state and slot kept), its file moves in a task of its own (`Job::moving`, which refuses pause/resume/cancel/remove), then it carries on; a job that can't resume is refused while running; a category without a folder leaves the file in place. A category's `max_concurrent` caps its running jobs under the global cap. At the first response, an auto-matched job is re-matched by its real name; its folder follows (via `Placement`, before the file is created) only if it was in its category's folder (`docs/decisions/0012`). Migrations run with foreign keys off and check them before committing. The daemon never creates starter categories itself; the UI's first-start setup offers them (`docs/decisions/0020`).
- **Saved views** (`docs/decisions/0026`): name + filters + sort/grouping, kept by the actor like categories (`Manager::add_view` …, `ViewChanged`/`ViewRemoved`). The engine checks and normalizes them but never filters jobs by them; front ends do. Removing a category replaces it with General in every view (saved before the category row goes).
- **Persistence** (`docs/decisions/0008`): SQLite holds jobs, and later categories, settings and per-segment progress. `STRICT` tables, numbered migrations in `store.rs` (never edit a released one). Saves happen on add, on every state change (inside `transition`), on `Event::Started` (file name + validator known) and progress at most every 2 s. **From 2c: flush file data before saving the progress that covers it.** On load, running jobs become Queued and `downloaded` is reset from the `.part.ottie` length.
- **Shutdown keeps running jobs running** in the database (it doesn't mark them Paused), so the next start continues them; user-paused jobs stay paused.
- **Files:** write to `<name>.part.ottie` (`PART_SUFFIX`, see `docs/decisions/0007`), rename on completion. Names come from `Content-Disposition` (`filename*`, then `filename`), else the final URL; chosen once at the first response. Sanitize them; never write outside the target folder; resolve collisions with ` (1)`, ` (2)`. Reserve names with `create_new` so it's atomic. Close the file before renaming (Windows refuses to rename open files).
- **Cancellation:** `tokio_util::sync::CancellationToken`, raced against network reads with `tokio::select! { biased; … }`. Pausing must stop all segments promptly and leave state consistent.
- **Backends:** downloads go through the `Backend` trait (`backend.rs`); the manager is generic over it. The native HTTP(S) backend comes first; native FTP/SFTP (7j) and yt-dlp (step 9) plug in beside it, so keep HTTP assumptions inside the HTTP backend.

## Platform gotchas

- **TLS:** rustls everywhere. No OpenSSL/native-tls; it breaks Android and Windows builds. On Android, `rustls-platform-verifier` must be initialised with the app's JNI context and ships a small Kotlin component. Check its docs in the Android step (10).
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
- UI (`docs/decisions/0015`, `0021`): runes mode; styles in each component's `<style>` using the variables in `app.css` (colours, `--font`/`--mono`, `--radius-s/m/l`, `--radius-pill`, `--edge`), never raw colours; the page talks to the daemon only through `api.ts`. Every theme must keep working in both styles (`docs/decisions/0044`): a component's own styles are the legacy look, the rounded one goes on top under `:global([data-style='rounded'])` (with `--sunken`, `--raised` and their shadows); check light and dark, rounded and legacy, when adding colours or shapes. Every page must also work at the desktop app's minimum size (720 × 480, `min_inner_size` in `app/src/main.rs`) and on a phone (360 px wide and up; under 40rem the tabs are a bar along the bottom; there and in windows under 40rem tall, the download list keeps a height of its own with the details panel under it, and the page scrolls). Pages are mirrored for languages written right to left (`docs/decisions/0042`): use logical properties (`margin-inline-start`, `inset-inline-end`, `text-align: start`, `style:inset-inline-start=…`), never left/right; a name, link or folder shown or typed gets `dir="auto"`; something drawn in a direction (a chart, an arrow) flips under `:dir(rtl)`. Two traps: a visually hidden `position: absolute` input needs a positioned parent, or it hangs from the window and makes it scroll; and a flex item with `overflow: hidden` shrinks below its content unless it's `flex: none`.
- Translations (`docs/decisions/0016`): no visible text in components or logic: every string is a message in `messages/en.json` *and* `fr.json` (French typography: no-break space before `: ? !` and inside « »); numbers, sizes and durations go through `format.ts` (`Intl`); daemon text is shown through `explain.ts`. Functions that build text take a `locale` parameter (default `getLocale()`) so tests can check both languages. Other languages (`docs/decisions/0041`): write a count as a label ("Pobierane: {count}"), not with plural forms; a new language is a file in `ui/messages/` and `app/messages/`, its tag in `project.inlang/settings.json`, its name in `NAMES` (`languages.ts`) and its file in `LANGUAGES` (`app/src/texts.rs`); once a native speaker has checked it, add it to `REVIEWED`. A language Outfit can't fully draw goes in the `:lang()` list in `app.css` (the system's font throughout; a test fails until it does) and in the table under "Languages and fonts". A language written right to left goes in `RIGHT_TO_LEFT` in `languages.ts` and `app/src/texts.rs`, and every placeholder in its file sits between U+2068 and U+2069 (a test checks). A language the browsers lack `Intl` data for (check in Edge: `Intl.NumberFormat.supportedLocalesOf`) gets a stand-in in `INTL` (`languages.ts`), and its tray texts follow the stand-in.
- API docs (`docs/decisions/0030`): every handler has a `#[utoipa::path]` (route, body, answers, the `Reason` codes it can refuse with) and is registered with `routes!`; wire types derive `ToSchema` beside `TS`. Fields inside the tagged enums (`Problem`, `Reason`, `Event`) stay one word (utoipa ignores `rename_all_fields`; a test checks). A change in how to use the API also updates `docs/api/README.md`.
- API errors (`docs/decisions/0016`): every refusal has a `Reason` code (`crates/api/src/types.rs`) besides the English `error`; a new engine `Error` variant gets a code in `error.rs`.
- Tests: engine behaviour is tested against the in-process server in `crates/core/tests/support/` (dropped connections, no range support, slow chunks, changed ETag, `unplug()`: open connections go silent as after a network change). Speed-limit tests (`tests/throttle.rs`) assert shares and minimum durations with wide margins, never exact speeds; progress snapshots lag up to ~0.5 s, so don't measure rates over short windows. Multi-connection tests (`tests/segmented.rs`) use `HttpConfig { connections: 4, min_split: 64 KiB }`; the other test files pin one connection or use files too small to split. Tests never hit the real internet. A resumed download must be byte-identical to a clean one. Don't assert exact byte counts after an injected connection drop; assert `.part.ottie` length == `downloaded` instead. Manager tests subscribe before adding jobs, and wait for several concurrent jobs with `wait_for_all` (any order), never one after another.
- Line endings: LF everywhere (`.gitattributes`).
- Commits: Conventional Commits (`feat(core): …`, `fix(extension): …`), one per slice: the code, its tests and its docs (decision record + its line in `docs/decisions/README.md`, CLAUDE.md) together. Unrelated changes (e.g. a test fix found along the way) get their own.
- New dependency: say what it's for and why it beats the alternatives before adding it.

## Languages and fonts

Ottie embeds two fonts, Outfit and DM Mono for numbers in columns (`docs/decisions/0041`). A page in a language Outfit draws in full uses them; a page in any other language uses the system's fonts throughout, monospace included, picked by the browser for the page's `lang` (`app.css`; a test checks the list against Outfit's characters). "Numbers and dates" says what `Intl` writes them in when the browser has no data for the language (Chrome, Edge, the desktop app).

| Language | Tag | Drawn in | Numbers and dates |
|---|---|---|---|
| English | `en` | Outfit | own |
| French | `fr` | Outfit | own |
| German | `de` | Outfit | own |
| Spanish (Spain) | `es-ES` | Outfit | own |
| Spanish (Latin America) | `es-419` | Outfit | own |
| Italian | `it` | Outfit | own |
| Dutch | `nl` | Outfit | own |
| Polish | `pl` | Outfit | own |
| Portuguese (Portugal) | `pt-PT` | Outfit | own |
| Portuguese (Brazil) | `pt-BR` | Outfit | own |
| Swedish | `sv` | Outfit | own |
| Romanian | `ro` | Outfit | own |
| Czech | `cs` | Outfit | own |
| Hungarian | `hu` | Outfit | own |
| Danish | `da` | Outfit | own |
| Finnish | `fi` | Outfit | own |
| Slovak | `sk` | Outfit | own |
| Croatian | `hr` | Outfit | own |
| Slovene | `sl` | Outfit | own |
| Lithuanian | `lt` | Outfit | own |
| Latvian | `lv` | Outfit | own |
| Estonian | `et` | Outfit | own |
| Irish | `ga` | Outfit | own |
| Maltese | `mt` | Outfit | own |
| Indonesian | `id` | Outfit | own |
| Turkish | `tr` | Outfit | own |
| Swahili | `sw` | Outfit | own |
| Zulu | `zu` | Outfit | South African English (`en-ZA`) |
| Greek | `el` | system font (Greek script) | own |
| Bulgarian | `bg` | system font (Cyrillic) | own |
| Ukrainian | `uk` | system font (Cyrillic) | own |
| Vietnamese | `vi` | system font (ạ, ư, ổ…) | own |
| Hindi | `hi` | system font (Devanagari) | own |
| Chinese (simplified) | `zh-Hans` | system font (Han) | own |
| Chinese (traditional) | `zh-Hant` | system font (Han) | own |
| Japanese | `ja` | system font (kana, Han) | own |
| Korean | `ko` | system font (Hangul) | own |
| Amharic | `am` | system font (Ethiopic) | own |
| Hausa | `ha` | system font (ɓ, ɗ, ƙ) | Nigerian English (`en-NG`) |
| Yoruba | `yo` | system font (ẹ, ọ, ṣ, tone marks) | Nigerian English (`en-NG`) |
| Arabic | `ar` | system font (Arabic script), monospace too | own, Western digits (`ar-u-nu-latn`) |
| Persian | `fa` | system font (Arabic script), monospace too | own (Persian digits and calendar) |
| Urdu | `ur` | system font (Arabic script), monospace too | own |

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
   - ✅ ~~5l starting at sign-in (`docs/decisions/0039`)~~
6. ✅ Scheduler (`docs/decisions/0040`):
   - ✅ ~~6a a download's own start and stop time (the `Scheduled` state)~~
   - ✅ ~~6b time slots + the Schedule tab~~
   - ✅ ~~6c when everything has finished (desktop app)~~
7. More features:
   - ✅ ~~7a clipboard monitoring (desktop app, `docs/decisions/0043`)~~
   - ✅ ~~the rounded style, the look from before as legacy palettes, the new Dashboard and an hour of speed (`docs/decisions/0044`)~~
   - 7b open the file and its folder (desktop app: app commands that take a job ID, never a path; a
     confirmation before running a program; double-click and Enter open too), Pause all / Resume all
     in the main window
   - 7c keyboard shortcuts (tabs, add/paste a link, Space, arrows, Enter, Delete; a "?" list of them;
     never while typing)
   - 7d animations (dialogs, menus, rows coming and going, the queue's reorder), none under
     `prefers-reduced-motion`
   - 7e a file already on disk: the add step says so (preview and at the first response) and offers
     rename, overwrite or skip; a setting decides when nobody is asked (extension, clipboard), rename
     by default as today
   - 7f onboarding in several steps, each skippable (and the whole of it): e.g. language + look,
     starter categories, downloads folder, sign-in and closing (desktop app), notifications, later the
     extension and other devices
   - 7g Ottie as a service only (no app): `ottie service install|uninstall|status`. Per user by default
     (systemd user unit, LaunchAgent, Task Scheduler at sign-in: the user's own folders), system-wide
     for headless machines (system unit, Windows service, Docker image: another account, so other
     folders and file owners). People connect as today: the daemon's page in a browser (`ottie
     open`/`ottie link`), the desktop app (uses a daemon that answers), the extension
   - 7h remote access, documented: `tailscale serve` (stays on 127.0.0.1, HTTPS within the tailnet), a
     VPN, or a reverse proxy with HTTPS; never the port open to the internet. Hardening: folders a
     client may write to limited to a list in config.toml (the token can name any folder today, so it
     can write anywhere the daemon's user can, e.g. a Startup folder); a token per device, revocable,
     and rotation (maybe read-only ones); a system service under its own locked-down user
   - 7i several daemons from one place: in the desktop app, a list (name, URL, token in the OS
     keychain) and a switcher, with fewer app commands for a remote daemon's page (no open folder, no
     play with); in the web UI, a switcher first (needs CORS on the daemon, safe since the token isn't
     a cookie), maybe later one list across daemons (IDs clash, one event stream each)
   - 7j FTP/SFTP as a native backend
8. Browser extension.
9. yt-dlp backend (its own updates, apart from the app's).
10. Tauri Android, last, once everything else is done: embedded core, background downloads, share-sheet intake.

Languages, a batch per slice, between the steps above (`docs/decisions/0041`; no Russian or Hebrew for now):
- ✅ ~~L1 "System" by default (also as the shared language), matching the system's tag, unreviewed note; de, es-ES, es-419, it, nl, pl, pt-PT, pt-BR~~
- ✅ ~~L2 the rest of the EU in Latin script: sv, ro, cs, hu, da, fi, sk, hr, sl, lt, lv, et, ga, mt~~
- ✅ ~~L3 Greek and Cyrillic: el, bg, uk (Manrope + JetBrains Mono, bundled)~~
- ✅ ~~L4 Asia: zh-Hans, zh-Hant, ja, ko, id, hi, vi, tr (Manrope for vi; Noto cut to our messages for hi and CJK)~~
- ✅ ~~L5 Africa: sw, am, ha, yo, zu (only Outfit + DM Mono embedded from then on, the system's fonts for the rest; ha, yo, zu numbers and dates in their country's English)~~
- ✅ ~~L6 Arabic, Persian and Urdu, with the whole layout mirrored (right to left; `docs/decisions/0042`)~~

Each translation is drafted, then checked by a native speaker when one helps (a platform like Weblate); checked ones go in `REVIEWED`.

## Open decisions

- Extension pairing: both, a native messaging host when installed (zero-config, can start the daemon) and the token as the fallback (remote daemon, portable setup). Details in step 8.
- yt-dlp: bundled or found, how it updates, ffmpeg, format choice.

Settled: no uGet import; no aria2 (its HTTP strengths are in our engine already) and no BitTorrent/magnet links.

## Later (TODO)

- Metalink? (one file from several mirrors, checked by checksum).
