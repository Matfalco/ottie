# 0015: The web UI: SvelteKit as a single-page app, built into the binary

Accepted 2026-10-12.

**Framework:** SvelteKit with `adapter-static` in single-page mode (`ssr = false`, `fallback: 'index.html'`). Everything renders in the browser, and the build is plain files. SvelteKit rather than plain Svelte + Vite gives file-based routes and layouts for the screens to come (settings, categories), and it's what Tauri recommends. Svelte 5 runes mode, TypeScript `strict`. Scaffolded with the official `sv` CLI. Add-ons: Prettier (formatting), Vitest (unit tests), the static adapter. No experimental features.

**Styling:** plain CSS. Each component has a scoped `<style>`; `src/app.css` defines colour variables taken from the Ottie icon (teal `#0E5A6B` / `#2B8A99`, amber `#FFC247`, warm neutrals), with a dark-mode set under `prefers-color-scheme`. Components use the variables only.

**Shipping:** the build goes to `crates/api/ui-dist/`, and `rust-embed` compiles it into `ottie-api`. So `cargo install ottie`, Docker or a NAS get the UI with the binary.
- In debug builds the files are read from disk, so a UI rebuild needs no Rust rebuild.
- A Rust build without a UI build still works, and serves a page explaining how to build the UI. That keeps `cargo build` and the Rust tests independent of Node.
- `ui-dist/` isn't in git, but `include` puts it in the published crate. Releases build the UI first.
- `ottie serve --ui-dir <folder>` serves another build. Paths are confined to that folder.
- UI files need no token; only `/api` does. Hashed files under `_app/immutable/` are cached for a year; `index.html` is never cached.

**Signing in:**
- `ottie serve` prints `http://127.0.0.1:16889/#token=…` when stdout is a terminal.
- `ottie open` opens the browser on it. It goes through a private redirect page in the config folder, so the token never appears on a browser's command line, where other users could see it in the process list.
- The UI reads the fragment, removes it from the address bar and history, and keeps the token in `localStorage`.
- Without a token, a login page asks for it (`ottie token`). A 401 signs out.

**Layout:** the page fills the window like an app; only the list scrolls.
- **Top row:** the logo and the add bar. The folder for a download is set in a small panel under a folder button. In the desktop app this row is the title bar, with the window buttons (`docs/decisions/0019`).
- **Left menu:** the app's sections: Dashboard (`/`) and Downloads (`/downloads`, with a badge counting the active ones). Categories and Settings join when their pages exist, never as empty pages. The connection status and the language sit at the bottom. On medium widths (under 56rem) the menu shows icons only, like VS Code's activity bar. On phones it becomes a bar along the bottom, as on Android.
- **One shell for every page** (`Shell.svelte`, in the root layout): the title bar, the menu, the live session (one events WebSocket for all pages) and the confirmation dialog. Pages get the session and the "act on downloads" function through a Svelte context (`context.ts`).
- **Downloads: status filter.** Chips above the toolbar: All, Active, Queued, Stopped, Completed, each with its count. "Stopped" means paused, failed or cancelled: whatever won't carry on by itself. The chosen status is in the address (`/downloads?status=stopped`), so it can be bookmarked and the dashboard can link to it. Adding a download while the filter would hide it switches back to All. Search and saved views will join this filter area.
- **Dashboard:** the total download speed in large type, with a graph of the last 5 minutes. That's one sample per second, kept by the page, so the graph starts when the page opens; its scale rounds to 1, 2 or 5 of the unit shown, and hovering shows a crosshair with the value and how long ago. Then a tile per status, each opening Downloads with that filter. Then "Needs attention" (failed and retrying downloads, with Resume and Remove) and the latest finished downloads (newest added first: the daemon doesn't record when a download finished).
- **Toolbar:** Resume, Pause, Cancel and Remove act on the selected downloads that each one applies to. It also shows the selection count and "Clear finished". Cancel and Remove ask first, in a dialog, only when downloaded data would be lost.
- **List:** a table with compact rows, one per download. Columns: a checkbox, Name (with a file-type icon), Size, Progress (bar and percent), Speed and Time left (one status text across both when the download isn't running: why it waits, failed or retries), and Added. Header widths are fixed, so columns don't jump as the numbers change. Narrower windows drop Added, then Size.
- **Selection** works as in a file manager. A click selects one row, Ctrl+click adds or removes one, and Shift+click selects a range. The checkboxes, Ctrl+A, Escape and Delete work too. Each row's own Pause/Resume and Remove buttons appear on hover (always on touch screens).
- **Icons:** Lucide (`@lucide/svelte`, ISC), imported one by one.
- **Phone-wide screens (under 40rem):** the add bar gets its own row, the views form a row that scrolls sideways, and each download shows its name and full status above its progress bar.

**Live data:**
- The page opens the events WebSocket first, then fetches `GET /jobs`. Events that arrive during the fetch are buffered and replayed after it. They're full states in order, so the replay always ends on the latest state.
- The WebSocket reconnects with growing waits (0.5 s → 10 s) and resyncs.
- Speed is computed in the browser from progress events (smoothed; shown as 0 after 3 s without progress).
- The list is newest first, so what you just added is at the top.

**Default folder:** at startup, if General has no folder, the daemon gives it the OS's Downloads folder (`~/Downloads` where the OS names none). A download can then be added with only a link, and a folder can still be given per download.

**Development:** `cargo run -p ottie -- serve`, then `pnpm --filter ui dev`. Vite serves the page and forwards `/api` (including the WebSocket) to the daemon (`OTTIE_DAEMON` to change it). Sign in on Vite's page with `ottie token`.

**Tooling:** pnpm workspace at the repo root (`ui`, `shared/api-types`). The generated types are the `@ottie/api-types` package, imported as `@ottie/api-types/Job`. Inside the UI, `#lib/…` imports name the file with `.js` (`#lib/format.js` → `format.ts`), as SvelteKit 3's subpath imports need.
