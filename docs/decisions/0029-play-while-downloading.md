# 0029: Play while downloading

Accepted 2026-10-13. Adds an exception to `docs/decisions/0014`: play links work without the token.

**Decision** (Mathieu's choice): a download's file, complete or not, can be played through a short-lived **play link**. The download order adapts in two ways: it **follows the player** (seeks included), and later a per-download **streaming mode** keeps a strict order from the start. They come as three slices:
- **5f1 (this one):** play links and the reader. The order doesn't change yet. One connection already arrives in order; with several, a player waits for the gap before its position to fill.
- **5f2, follow the player:** while a link is being read, a free connection goes to the offset the player reads (instead of splitting the range with the most left), and the last ~1 MiB is fetched early (many MP4s keep their index there). Connections flush more often while someone reads.
- **5f3, streaming mode:** a toggle saved with the download (when adding, or in the detail panel): in order from the start, last chunk first.

**Play links:**
- `POST /jobs/{id}/stream` (with the token) answers `{ "url": "/api/v1/stream/<ticket>/<file name>" }`. The ticket is 32 random bytes as hex. A job keeps the same link while it's in use.
- `GET /api/v1/stream/<ticket>/<name>` needs **no token**: players (VLC, mpv, a browser's `<video>`, Android players) can't send one. The ticket stands for it. The name is only there so players can show it.
- A link reads one job and nothing else. It expires once **unused for 12 hours**. Links live in the daemon's memory, so a restart ends them all. Anyone who has a link can play that file, and the UI says so.
- A complete job is served from its file, an unfinished one from its `.part.ottie`. `Range` works as usual (`206`, `416` past the end) when the size is known. With no size, the whole file is sent as it arrives, with `Accept-Ranges: none`.
- A read past what has arrived **waits** for the bytes, also while the download is queued or paused. It ends (a broken transfer, never wrong bytes) if the download fails, is cancelled or removed, or starts over because its file changed on the server. Shutdown ends open streams too.
- Responses carry `Content-Security-Policy: sandbox` and `X-Content-Type-Options: nosniff`. A downloaded web page or SVG opened through its link then can't run scripts as the daemon's own pages (which hold the token).

**The reader** (`ottie_core::Reader`): it hands out only bytes already on disk. With one connection, that's up to the `.part.ottie` length. With several, it's the flushed ranges; they're saved every second, so a reader is up to ~2 s behind (5f2 improves that). Once complete, it's the whole file. It keeps the file open while the download completes or moves. Windows allows the rename because Rust opens files with `FILE_SHARE_DELETE`, and a test covers it. It reopens the file when the job's path changes.

**In the UI:** the detail panel shows **Play** (a link to open in a new tab; the desktop app opens it in the system's browser) and **Copy link** (for VLC and other players), for videos and music that have a file.

**Alternatives:**
- A key derived from the token and the job ID (HMAC) in the URL: no state, and it survives restarts. But revoking one link means changing the token for every client, and it's a long-lived credential sitting in player histories.
- Header-only auth, playing through `fetch` + Media Source in our own page: no external players, and many formats wouldn't play.
