# 0024: Changing one download's own settings

Accepted 2026-10-13.

**Decision:** `PATCH /jobs/{id}` changes a download's own settings: `{ "speedLimit"?: bytes per second (0 = none), "priority"?: "low" | "normal" | "high" | "urgent" }`. A field left out stays as it is. It works in any state and takes effect at once, even while the download runs. The answer is 204; an unknown job is `jobNotFound` (404), and a body it can't read is `invalidBody` (422).
- **One change, one save, one event** (as `PATCH /settings`, `docs/decisions/0022`). The engine applies the fields in one actor command, `Manager::change_job(id, JobChange)`. If anything differs, it saves the job once and broadcasts one `Changed`. A change that changes nothing saves and announces nothing. `set_speed_limit` and `set_priority` stay as one-line wrappers.
- **Moving to a category stays `POST /jobs/{id}/category`.** It does more than set a field: it marks the category as the user's choice and may also move the download to that category's folder.
- Later fields (a new name, another folder before the download starts) can join the same body.

**In the UI:** a panel under the Downloads list shows the one selected download (mockup G). It has the main action for its state, progress and a few numbers, its category, its folder, its own speed limit and its priority. A download's limit remembers its last value per device, apart from the total limit's, so setting one never changes the other. "Play while downloading" and "Show in folder" are shown disabled until they work (`docs/decisions/0021`). The file map works since slice 5d (`docs/decisions/0025`).

**Alternatives:** one `POST` route per property, like `/category`. More routes, and two saves and two events when both change; `0022` turned down the same for settings (Mathieu's choice).
