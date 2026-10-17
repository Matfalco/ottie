# 0014: The HTTP API, the token, and where the daemon keeps its files

Accepted 2026-10-12.

**Files** (OS-standard folders):
- Config folder (`%APPDATA%\Ottie`, `~/.config/ottie`, macOS `~/Library/Application Support/Ottie`) holds `config.toml` and `token`.
- Data folder (`%LOCALAPPDATA%\Ottie`, `~/.local/share/ottie`) holds `ottie.db`.
- `--config-dir` / `--data-dir` or `OTTIE_CONFIG_DIR` / `OTTIE_DATA_DIR` move them, e.g. to one Docker volume.

**`config.toml`** holds the daemon's own options only: `address` and `port`. Unknown keys are refused, so a typo is caught rather than ignored. Download settings stay in the database, where the API can change them. Precedence: flag, then `OTTIE_*` variable, then the file, then the default. A missing file means all defaults.

**Address:** 127.0.0.1:24068. Any non-loopback address logs a warning: remote access belongs behind a VPN or reverse proxy.
- *Changed 2026-10-13* (Mathieu's choice): the port was 16889 ("OTTY", 6889 on a phone keypad, with a 1 in front to keep clear of BitTorrent's 6881–6889 and aria2's 6800, 6881–6999). 24068 stays clear of those too, and is below the ports systems hand out for outgoing connections (Linux from 32768, Windows from 49152). A daemon set to another port in `config.toml` or `OTTIE_PORT` keeps it.

**Token:**
- 32 random bytes as hex in `<config>/token`, created on first use (`ottie serve` or `ottie token`). On Unix it's readable only by the owner; on Windows `%APPDATA%` is already private to the user.
- `ottie token` prints it. `ottie serve` says where it is only when stdout is a terminal, never into logs (systemd, Docker).
- It's sent as `Authorization: Bearer <token>`, compared in constant time.
- The events WebSocket can't carry headers from a browser, so the client offers two protocols, `ottie.v1` and `ottie.token.<token>`; the server picks `ottie.v1`. The token never goes in a URL, where it would reach logs and history, and never in a cookie, which other sites could make the browser send to 127.0.0.1.
- Unknown paths under `/api/v1` also need it.
- The web UI (3b) gets it from a printed link (`…/#token=…`, the part after `#` never reaches the server) or a login page. The desktop app reads the file.

**API shape (v1, under `/api/v1`):**
- `GET /jobs` lists every job; `POST /jobs` adds one (`{url, destDir?, category?}` → 201 `{id}`); `POST /jobs/{id}/pause|resume|cancel` → 204; `DELETE /jobs/{id}` → 204 removes it (`docs/decisions/0018`).
- `destDir` must be absolute.
- `GET /events` is a WebSocket of `{"type": "changed", job}`, `{"type": "progress", id, downloaded, total}`, `{"type": "removed", id}`, `{"type": "categoryChanged", category}`, `{"type": "categoryRemoved", id}` and `{"type": "resync"}` (events were missed: fetch the list again). Clients connect first, then fetch `GET /jobs`, then apply events in order.
- Errors are `{"error": "…"}`: 400 bad input, 401 no token, 404 unknown job, 409 not possible in this state, 422 missing fields, 503 stopping.
- JSON is camelCase. IDs are strings. Byte counts and times (Unix milliseconds) are numbers.
- The API has its own types (`crates/api/src/types.rs`), converted from the engine's, so engine changes don't break clients. ts-rs writes them to `shared/api-types/` on `cargo test -p ottie-api`.
- Categories (slice 4d): `GET /categories`; `POST /categories` (`{name, folder, extensions, speedLimit, maxConcurrent}` → 201 `{id}`); `PUT /categories/{id}` (the same body: it replaces every setting); `DELETE /categories/{id}` (its jobs move to General first, and a `changed` event goes out for each); `POST /jobs/{id}/category` (`{category}`, the user's choice, never re-matched). Folders must be absolute.
- `POST /categories/match` (`{url, destDir?}` → `{category, extension, byFolder}`): where a link would go if added without a category (with `destDir`, the folder decides, `docs/decisions/0020`), so a client can show it before adding. It's a POST rather than `GET ?url=`, so links, which may carry tokens, never sit in an address or a log. The daemon stays the only place that knows the matching rule.
- Settings (slice 4e, `docs/decisions/0020`): `GET /settings` → `{setupDone}`; `PATCH /settings` changes only the fields given → 204. Slice 4f adds the download settings to the same object.
- Limits, priorities and streaming get their endpoints with their UI slices.

**Crates:** `ottie-api` (axum, ts-rs) and the `ottie` binary (clap for the command line, `directories` for the OS folders, `toml` for the config file, `getrandom` for the token, `tracing-subscriber` for logs on stderr, filtered by `OTTIE_LOG`). `ottie-core` and `ottie-api` are now published with `ottie`.

**Stopping:** on Ctrl+C or SIGTERM, event streams close, the server finishes its requests, then `Manager::shutdown` saves every download so the next start continues it.

**Later:** other devices get tokens of their own, and may only write into some folders (`docs/decisions/0052`).
