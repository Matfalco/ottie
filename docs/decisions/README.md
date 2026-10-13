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
| [0010](0010-speed-limits-and-priorities.md) | Speed limits per download, per category and in total, changeable live; four priorities that order the queue and share the bandwidth (Urgent first, then High 4 / Normal 2 / Low 1). |
| [0011](0011-categories.md) | Every download has a category (folder, extensions, limits); the built-in General takes what matches nothing. Amended by 0020 and 0028. |
| [0012](0012-content-disposition-names.md) | File names come from `Content-Disposition`, else the final URL; a category matched automatically is matched again by the real name. |
| [0013](0013-retries.md) | Errors that may go away are retried with growing waits; a retrying download gives its slot away after a few tries if another is waiting. |
| [0018](0018-removing-downloads.md) | `DELETE /jobs/{id}` takes a download off the list in any state; a finished file stays on disk, an unfinished one is deleted. |
| [0025](0025-ranges-in-progress.md) | Progress events carry each connection's range, so clients can show where every connection is. |
| [0028](0028-moving-files-with-their-category.md) | Moving a download to another category moves its file to that category's folder, even while it runs. |
| [0029](0029-play-while-downloading.md) | Play while downloading through short-lived play links that need no token; the download order follows the player, and a streaming mode keeps it in order. |

## API and daemon

| # | Decision |
|---|---|
| [0014](0014-api-and-daemon-files.md) | REST + WebSocket API under `/api/v1`, a token on every request (never in a URL), config and data in OS-standard folders, 127.0.0.1:24068 by default. The API as it is now: `docs/api/` (0030). |
| [0016](0016-translations-and-error-codes.md) | The UI is translated (English and French first, Paraglide); the daemon sends a code for each failure plus English text, and clients translate the code. |
| [0022](0022-settings-and-shared-appearance.md) | One settings resource (`GET`/`PATCH /settings`) with checked ranges; the look and language are shared by every device unless one opts out. |
| [0024](0024-changing-a-download.md) | `PATCH /jobs/{id}` changes a download's own speed limit and priority, live. |
| [0026](0026-saved-views.md) | Saved views (name, filters, sorting) are kept by the daemon, so every device has the same ones. |
| [0030](0030-api-documentation.md) | The API is documented in `docs/api/`: a written guide, and an OpenAPI description generated from the handlers (utoipa), also served at `/api/v1/openapi.json`. |

## Front ends

| # | Decision |
|---|---|
| [0015](0015-web-ui.md) | The web UI is a SvelteKit single-page app in plain CSS, built into the binary. |
| [0017](0017-desktop-app.md) | The desktop app (Tauri 2) is a window on the daemon's own page, and starts the daemon as a sidecar unless one already answers. |
| [0019](0019-own-title-bar.md) | The desktop window draws its own title bar, with its own window buttons. |
| [0020](0020-first-start-setup.md) | A first-start setup offers starter categories; a built-in Other category holds folders of no category; adding a download has a category step. |
| [0021](0021-themes-and-layout.md) | The UI follows the chosen mockups: four themes (Sea and Neutral palettes, each light and dark), tabs in the title bar. |
| [0023](0023-downloads-filters-and-sorting.md) | The Downloads page has quick-filter pills, a filters and sorting panel, and a list grouped by status. |
| [0027](0027-file-map-in-the-bar.md) | The detail panel's progress bar is the file map, showing each range where it lies in the file. Replaces 0025's separate map. |
