# 0025: Each connection's range, live in progress events

Accepted 2026-10-13.

**Decision** (Mathieu's choice): a download over several connections tells clients where each connection is, in the events they already get.
- **`progress` events carry `ranges: [{ start, end, done }]`.** There is one range per part of the file: `end` is exclusive and `done` counts the bytes from `start` that have arrived. Ranges arrive in no set order. These are live: bytes may still be on their way to disk. They're sent at most every 200 ms, with the progress they belong to. A download over one connection sends `[]`.
- **`GET /jobs` and `changed` events carry the ranges as last saved** (flushed to disk, `docs/decisions/0009`). So a paused download's map shows its gaps, and a page that just opened draws it before the next progress event. A finished download has none.
- In the engine, `Event::Progress` and `JobEvent::Progress` gained `segments`, from each connection's write position. `JobSnapshot` has the saved `segments`.

**In the UI:**
- A row's progress bar is cut into one piece per range, sized by its share of the file.
- The detail panel's file map cuts the file into 48 blocks. Each block is received, arriving (a connection writes there right now), partly received, or left.
- One connection is drawn as one range from the start, from `downloaded`.
- The map needs the size; until it's known, it says so.

**Alternatives:**
- `GET /jobs/{id}/ranges`, polled by the panel: smaller events, but polling, and only the panel would see ranges.
- Subscribing to one download's ranges over the WebSocket: the leanest on the wire, but the WebSocket would become two-way.

A few dozen numbers per event is cheap next to that.
