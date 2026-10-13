# Talking to the Ottie daemon

Everything Ottie does goes through the daemon's HTTP API: the web UI, the desktop app and the browser extension use it, and so can your scripts. This guide covers how to sign in, the conventions, live events, errors and play links. The full reference, every endpoint and field, is the OpenAPI description:

- [`openapi.json`](openapi.json) in this folder (open it in any OpenAPI viewer, such as Swagger Editor or Redocly), or
- `GET /api/v1/openapi.json` on a running daemon, which describes exactly that version.

Both are generated from the code (`cargo test -p ottie-api` rewrites the file), so they match what the daemon answers. TypeScript clients can use the generated types in [`shared/api-types/`](../../shared/api-types/) instead of writing their own.

## Where it is

`ottie serve` listens on **`http://127.0.0.1:24068`**, on this computer only. Change it with `--address`/`--port`, `OTTIE_ADDRESS`/`OTTIE_PORT`, or `address`/`port` in `config.toml`. Everything is under **`/api/v1`**; every other path is the web UI.

Reaching it from another device is up to you. Put it behind a reverse proxy or a VPN (with HTTPS) rather than exposing the port: the token below is the only thing guarding it.

## Signing in

Every request needs the API **token**: 64 hex characters, created the first time the daemon runs.

```bash
ottie token        # prints it
```

It's stored in the `token` file of Ottie's config folder (`%APPDATA%\Ottie` on Windows, `~/.config/ottie` on Linux, `~/Library/Application Support/Ottie` on macOS). Anyone who has it can control your downloads, so keep it private.

Send it as a bearer token:

```bash
TOKEN=$(ottie token)
curl -H "Authorization: Bearer $TOKEN" http://127.0.0.1:24068/api/v1/jobs
```

Without it, or with the wrong one, every path under `/api/v1` answers `401` (even unknown paths, so they don't reveal what exists). The token never goes in a URL or a cookie.

There are two exceptions. The events WebSocket takes the token in a different way (see [Live events](#live-events)), and play links need no token at all (see [Play while downloading](#play-while-downloading)).

## Conventions

- JSON, with `camelCase` field names. Send `Content-Type: application/json` with a body.
- **Job IDs** are strings like `2026.10.13-14.28.59-1`: the creation time in UTC plus a counter. Sorting them sorts by creation. Category and view IDs are numbers; General is category `1`.
- Sizes and speeds are in **bytes** (speeds in bytes per second); `0` means "no limit". Times are milliseconds since 1970 (UTC).
- Folders are **absolute paths on the daemon's computer**.
- Changes answer `204 No Content`; creations answer `201` with the new ID.
- Fields marked optional in a body may be left out. In `PATCH` bodies, a field left out stays as it is.

## A short tour

```bash
API=http://127.0.0.1:24068/api/v1
AUTH="Authorization: Bearer $(ottie token)"

# Add a download (to its category's folder; add "destDir" or "category" to choose)
curl -H "$AUTH" -H "Content-Type: application/json" \
     -d '{"url": "https://example.com/file.iso"}' $API/jobs
# → 201 {"id": "2026.10.13-14.28.59-1"}

# Every download, oldest first
curl -H "$AUTH" $API/jobs

# Pause it (answers once it has really stopped), then resume it
curl -H "$AUTH" -X POST $API/jobs/2026.10.13-14.28.59-1/pause
curl -H "$AUTH" -X POST $API/jobs/2026.10.13-14.28.59-1/resume

# Its own speed limit (1 MiB/s) and priority
curl -H "$AUTH" -H "Content-Type: application/json" -X PATCH \
     -d '{"speedLimit": 1048576, "priority": "high"}' $API/jobs/2026.10.13-14.28.59-1

# Take it off the list (a finished file stays on disk)
curl -H "$AUTH" -X DELETE $API/jobs/2026.10.13-14.28.59-1

# A total limit for all downloads, and at most 3 at once
curl -H "$AUTH" -H "Content-Type: application/json" -X PATCH \
     -d '{"globalLimit": 5242880, "maxConcurrent": 3}' $API/settings
```

A download's `state` is one of `queued`, `connecting`, `downloading`, `paused`, `completed`, `failed`, `cancelled` and `retrying`. Downloads wait in `queued` until a slot is free. `failed` and `paused` downloads can be resumed. `completed` and `cancelled` are final.

## Live events

`GET /api/v1/events` is a WebSocket that pushes every change as a JSON message. Browsers can't set headers on a WebSocket, so the token travels as an offered **protocol**: offer both `ottie.v1` and `ottie.token.<token>`, and the server picks `ottie.v1`.

```js
const socket = new WebSocket('ws://127.0.0.1:24068/api/v1/events', [
	'ottie.v1',
	`ottie.token.${token}`
]);
```

To keep an up-to-date copy of the list:

1. **Connect first**, then fetch `GET /jobs` (and `/categories`, `/views`, `/settings` if you show them). Connecting first means no change can slip between the two.
2. Apply each message in order. The `type` field says what it is:

| `type` | Fields | What to do |
|---|---|---|
| `changed` | `job` | Insert or replace the job with that `id` |
| `progress` | `id`, `downloaded`, `total`, `ranges` | Update its byte counts (a few times a second while it runs) |
| `removed` | `id` | Drop the job |
| `categoryChanged` / `categoryRemoved` | `category` / `id` | The same, for categories |
| `viewChanged` / `viewRemoved` | `view` / `id` | The same, for saved views |
| `settingsChanged` | `settings` | Replace the settings: all of them, as they are now |
| `resync` | | You fell behind and missed events: fetch the lists again |

3. If the socket closes, reconnect and start again at step 1.

`ranges` lists, for a download over several connections, each connection's part of the file (`start`, exclusive `end`, and `done` bytes from `start`). It's empty for one connection.

## Errors

A refusal answers a status and a JSON body with English text and a **code**:

```json
{ "error": "no job with ID 2026.10.13-14.28.59-9", "code": "jobNotFound", "id": "2026.10.13-14.28.59-9" }
```

Show users a message chosen by `code` (translated, with the extra fields filled in), and fall back to `error` for codes you don't know: a newer daemon may add some. Every code is listed under the `Reason` schema in `openapi.json`, and each endpoint lists the ones it can answer. In short:

| Status | Means | Examples |
|---|---|---|
| 400 | Something in the request is wrong | `invalidUrl`, `relativeFolder`, `outOfRange` |
| 401 | No token, or the wrong one | `unauthorized` |
| 404 | No such thing | `jobNotFound`, `categoryNotFound`, `noSuchEndpoint` |
| 409 | Not possible right now | `invalidTransition` (resuming a finished download), `jobMoving` |
| 422 | The body isn't the JSON expected | `invalidBody` |
| 503 | The daemon is shutting down | `stopping` |

When a download fails, the job itself says why: `error` in English, and `problem` as a code (`network`, `httpStatus`, `stalled`…) to translate the same way.

## Play while downloading

Players such as VLC, mpv or a browser's `<video>` can't send the token. So a download, finished or not, gets a **play link** of its own:

```bash
curl -H "$AUTH" -X POST $API/jobs/2026.10.13-14.28.59-1/stream
# → {"url": "/api/v1/stream/9f2c…e1/movie.mp4"}
vlc "http://127.0.0.1:24068/api/v1/stream/9f2c…e1/movie.mp4"
```

- The link needs **no token**: the random ticket in it is the permission. It reads that one file, nothing else. Anyone who has it can play the file, so share it only as you would the file itself.
- It expires after **12 hours unused**, and when the daemon restarts. The same download keeps the same link while it's in use.
- `Range` requests work as usual (`206`, or `416` past the end) when the size is known. If the server never said the size, the whole file is sent as it arrives.
- A read past what has arrived **waits** for the bytes, even while the download is queued or paused. The answer breaks off if the download fails, is cancelled or removed, or has to start over because the file changed on the server.

## Versions

The `v1` in the path changes only if the API changes in a way that breaks existing clients. New endpoints, new fields and new codes can appear at any time, so ignore fields and codes you don't know rather than failing on them.
