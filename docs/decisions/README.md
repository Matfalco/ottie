# Decisions

Each accepted decision has its own short file here (`NNNN-title.md`): what was decided, why, and the alternatives. This page sums them up in one line each. When a later decision changes an earlier one, both say so.

## Project

| # | Decision |
|---|---|
| [0001](0001-package-names.md) | Cargo packages with a `dm-` prefix. **Superseded by 0003.** |
| [0002](0002-http-client-tls.md) | reqwest 0.13 without default features: rustls (aws-lc-rs) with the OS certificate store, HTTP/2, system proxy. No OpenSSL. |
| [0003](0003-project-name-ottie.md) | The project is called Ottie (an otter mascot); package, folder and variable names follow, and the app identifier `io.github.matfalco.ottie` never changes. |
| [0004](0004-single-binary.md) | One `ottie` binary: `ottie serve` runs the daemon, other subcommands are thin clients of the HTTP API. |
| [0005](0005-license-mpl-2.0.md) | Everything is licensed MPL-2.0. |

## Engine

| # | Decision |
|---|---|
| [0006](0006-job-ids.md) | Job IDs are the creation time in UTC plus a per-second counter: `yyyy.mm.dd-hh.mm.ss-N`. |
| [0007](0007-partial-file-extension.md) | Unfinished downloads are written to `<name>.part.ottie` and renamed when complete. |
| [0008](0008-persistence.md) | SQLite with `STRICT` tables and numbered migrations; data is flushed before the progress covering it is saved; running downloads continue after a restart. |
| [0009](0009-multi-connection.md) | Multi-connection downloads with dynamic splitting: an idle connection takes an unfinished range or half of the busiest one. Only with a `206` and a validator. |
| [0010](0010-speed-limits-and-priorities.md) | Speed limits per download, per category and in total, changeable live; four priorities that order the queue and share the bandwidth (Urgent first, then High 4 / Normal 2 / Low 1). Amended by 0032. |
| [0011](0011-categories.md) | Every download has a category (folder, extensions, limits); the built-in General takes what matches nothing. Amended by 0020 and 0028. |
| [0012](0012-content-disposition-names.md) | File names come from `Content-Disposition`, else the final URL; a category matched automatically is matched again by the real name. |
| [0013](0013-retries.md) | Errors that may go away are retried with growing waits; a retrying download gives its slot away after a few tries if another is waiting. |
| [0018](0018-removing-downloads.md) | `DELETE /jobs/{id}` takes a download off the list in any state; a finished file stays on disk, an unfinished one is deleted. |
| [0025](0025-ranges-in-progress.md) | Progress events carry each connection's range, so clients can show where every connection is. |
| [0028](0028-moving-files-with-their-category.md) | Moving a download to another category moves its file to that category's folder, even while it runs. |
| [0029](0029-play-while-downloading.md) | Play while downloading through short-lived play links that need no token; the download order follows the player, and a streaming mode (on by itself for videos and music) keeps it in order. |
| [0032](0032-queue-order.md) | Each download has a place in the queue; buttons and drag and drop move waiting downloads within their priority (`POST /jobs/reorder`); the Waiting group shows the queue. |
| [0033](0033-connection-failures-and-starved-downloads.md) | One connection's failure no longer stops a multi-connection download; a connection the speed limit starves lets go of its request; a download that gets no bandwidth says so (`waitingForBandwidth`). |
| [0040](0040-scheduler.md) | Time slots (whole queue or a category, weekly or once; allow or stop, speed limit, max concurrent; higher in the list wins; a Schedule tab with a week view; `/slots`, one resource each), a download's own start and stop time (a new `Scheduled` state), and an action once everything has finished (quit, sleep, hibernate, shut down; desktop app: for once from the tray menu or a Downloads toolbar button, every time in Settings; a countdown window or notification with Cancel; the system's own calls, what it can't do greyed out). Times follow the daemon's clock and time zone, through `jiff`. |

## API and daemon

| # | Decision |
|---|---|
| [0014](0014-api-and-daemon-files.md) | REST + WebSocket API under `/api/v1`, a token on every request (never in a URL), config and data in OS-standard folders, 127.0.0.1:24068 by default. The API as it is now: `docs/api/` (0030). |
| [0016](0016-translations-and-error-codes.md) | The UI is translated (English and French first, Paraglide); the daemon sends a code for each failure plus English text, and clients translate the code. |
| [0022](0022-settings-and-shared-appearance.md) | One settings resource (`GET`/`PATCH /settings`) with checked ranges; the look and language are shared by every device unless one opts out. |
| [0024](0024-changing-a-download.md) | `PATCH /jobs/{id}` changes a download's own speed limit and priority, live. |
| [0026](0026-saved-views.md) | Saved views (name, filters, sorting) are kept by the daemon, so every device has the same ones. |
| [0030](0030-api-documentation.md) | The API is documented in `docs/api/`: a written guide, and an OpenAPI description generated from the handlers (utoipa), also served at `/api/v1/openapi.json`. |
| [0036](0036-several-links-at-once.md) | Several links at once, each its own download: the daemon finds the links in any text (`POST /jobs/preview`), a dialog lists them (tick boxes, a category each), and those that can be are added together (`POST /jobs/batch`); a setting caps how many. Sequences in one link come in 0037. |
| [0037](0037-sequences-in-one-link.md) | Sequences in one link, curl-style (`[001-120]`, `[a-z]`, `[0-100:10]`, `{small,large}`, every combination): the daemon expands them in the same preview, counting first so the cap refuses a huge one at once; only well-formed ones count, `\[` escapes. |

## Front ends

| # | Decision |
|---|---|
| [0015](0015-web-ui.md) | The web UI is a SvelteKit single-page app in plain CSS, built into the binary. |
| [0017](0017-desktop-app.md) | The desktop app (Tauri 2) is a window on the daemon's own page, and starts the daemon as a sidecar unless one already answers. Amended by 0034. |
| [0019](0019-own-title-bar.md) | The desktop window draws its own title bar, with its own window buttons. |
| [0020](0020-first-start-setup.md) | A first-start setup offers starter categories; a built-in Other category holds folders of no category; adding a download has a category step. |
| [0021](0021-themes-and-layout.md) | The UI follows the chosen mockups: four themes (Sea and Neutral palettes, each light and dark), tabs in the title bar. |
| [0023](0023-downloads-filters-and-sorting.md) | The Downloads page has quick-filter pills, a filters and sorting panel, and a list grouped by status. Amended by 0032 (oldest first by default). |
| [0027](0027-file-map-in-the-bar.md) | The detail panel's progress bar is the file map, showing each range where it lies in the file. Replaces 0025's separate map. |
| [0031](0031-opening-a-player.md) | Play opens a media player: in the desktop app as set in Settings (the system's player through a playlist, the browser, or a picked program); in a browser, "Open in player" saves a playlist. |
| [0034](0034-tray.md) | Closing the window keeps Ottie in the tray (a setting can make it quit). A right click opens Ottie's own menu (the daemon's `/tray` page in a small window): status, pause or resume everything, the total limit's field with standard rates; the system's menu stays for Linux and while the daemon is unreachable. |
| [0035](0035-notifications.md) | The desktop app notifies finished downloads once everything has finished (or one by one, or never, set in Settings) and failed downloads (a switch). Settings are this computer's; a click opens the window. Through `tauri-plugin-notification`. |
| [0038](0038-notification-sounds.md) | Notifications sound like the system (default), nothing, one of Ottie's ten sets (a sound for a finished download and one for a failed one, `app/sounds/`), or a sound chosen for each. The app plays Ottie's itself with `rodio` (a Windows toast can only play the system's); Ogg Vorbis, a little quieter than the system's sound. |
| [0039](0039-starting-at-sign-in.md) | At sign-in, Ottie starts in the tray (the app and its daemon), only the daemon (nothing on screen), or not at all, chosen in Settings and offered by the first-start setup (in the tray, chosen). One system entry (`--autostart`, through `auto-launch`, path quoted); `app.json` says what it starts. |
| [0041](0041-languages.md) | More languages in batches (EU, Latin America, Asia, Africa; Arabic last; no Russian or Hebrew for now): the first eight, the rest of the EU in Latin script, Greek, Bulgarian and Ukrainian, then Chinese (simplified, traditional), Japanese, Korean, Indonesian, Hindi, Vietnamese and Turkish, then Swahili, Amharic, Hausa, Yoruba and Zulu (numbers and dates of the last three in their country's English: browsers lack their data), then Arabic, Persian and Urdu (0042). Only Outfit and DM Mono are embedded: a language Outfit can't fully draw gets the system's fonts throughout. "System" is a language choice and the default, also as the shared one (`null`); `es-MX` → Latin American Spanish, `pt-BR` → Brazilian, other `es`/`pt` → Spain's and Portugal's. Unchecked translations say so; counts are written as labels, without plural forms. |
| [0042](0042-right-to-left.md) | Arabic, Persian and Urdu pages are mirrored (`dir="rtl"`, logical CSS properties): bars, file map, week view and speed chart too, the window's buttons on the left. Names, links and folders keep their own direction (`dir="auto"`, isolated placeholders). Arabic in Western digits, Persian in its own; the system's fonts, monospace included. The tray embeds their texts right to left. |
| [0043](0043-clipboard-monitoring.md) | The desktop app can watch the clipboard (opt-in: Settings, the tray menu, the first-start setup): a link copied elsewhere to a file type a category lists is offered in a small window in the corner that never takes the focus (Download, More… for categories and folder, Ignore), or added at once with a notification. Not what's copied in Ottie, nor what password managers mark as private (Windows). Through `arboard`; the daemon says which links count (`POST /jobs/preview`). |
| [0044](0044-rounded-style.md) | The rounded style (pills, sunken tracks with the chosen one raised, tactile switches, the page on a sheet) is the default in all four themes; the look from before stays as two palettes tagged Legacy, shared like the rest of the look. Legacy is each component's own CSS; the rounded style is added under `[data-style='rounded']`. Every palette gets the new Dashboard (the speed graph across the top, the four groups two by two with their first downloads) and a row bar that's a small copy of the details panel's. The graph shows up to an hour (5, 15, 30 min, 1 h): the daemon keeps the last hour of total speed (`GET /speed`), counted by the speed limiter. |
| [0045](0045-opening-files.md) | The desktop app opens a finished download's file (the details' main button, a double-click, Enter) and shows its folder with the file selected, through two app commands that take a job ID and ask the daemon for the path (`GET /jobs/{id}`). A program asks first in a native dialog of the app's own. On Windows, finished files get the Mark of the Web, as from a browser. Pause all and Resume all join the Downloads toolbar. |
| [0046](0046-keyboard-shortcuts.md) | Keyboard shortcuts in the main window: 1–5 for the tabs, Ctrl+V adds a copied link (several open the list), / the add field, ? the list of them (also in Settings). In Downloads, as in a file manager: arrows, Shift, Home/End, Ctrl+A, Esc, Space (pause or resume), Enter, Delete; Alt+arrows move in the queue; F the filters. Never while typing, nor a key the focused control uses, nor with a dialog open. |
| [0047](0047-one-look.md) | The legacy palettes go: Sea and Neutral, rounded, are the only ones. A saved legacy palette becomes its pair (migration 17, and on each device). The rounded rules are folded into each component's own CSS: no `data-style`, no `[data-style='rounded']`; checked to draw exactly as before. |
