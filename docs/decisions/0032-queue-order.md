# 0032: Moving downloads in the queue

Accepted 2026-10-13. Changes what `docs/decisions/0010` said about the order within a priority, and what `0023` said about the default sort and the Added column.

**Decision** (Mathieu's choice): every download has a **place** in the queue. Priority still comes first; the place orders downloads of the same priority, in place of their age.
- **New downloads go last** (after the highest place handed out). Migration 12 gives existing downloads their places in the order they were added, so nothing changes until someone moves one.
- **Moving** works on the waiting downloads, queued or paused, of the same priority. A paused download keeps its place, so it goes back there when resumed. Running and finished downloads keep their place but aren't moved.
- **Up / down** swaps with the next waiting download of the same priority; **top / bottom** goes past all of them; **before / after** a download puts them next to it (a drag and drop). Moving stops at the edge of its priority: to pass a High download, a Normal one has to become High. Several downloads move as a block and keep their order among themselves.
- A download that gave its slot away (`docs/decisions/0013`) still queues behind the others, whatever its place.
- The place is saved with the download (`queue_place`) and sent as `place` in the API. Only the order means something: moving swaps places among the downloads that move, so every other download keeps its own.

**API:** `POST /jobs/reorder` with `{ "ids": [...], "to": "up" | "down" | "top" | "bottom" | { "before": id } | { "after": id } }`. The daemon finds the neighbours, so a selection moves in one request and one actor command. Downloads that aren't waiting are left alone; next to a download, so are those of another priority than it (and all of them, if it isn't waiting). An unknown ID moves nothing (`jobNotFound`, 404); a malformed one is `invalidJobId` (400). Each download that moved is saved and sent in a `changed` event. Named "reorder" rather than "move", which already means moving to another category and folder.

**In the UI** (Mathieu's choices):
- **The Waiting group is the queue:** it's always in queue order, the next to start on top, whatever the list is sorted by. Changing a priority moves the download in it at once. The other groups follow the chosen sort. A list that isn't grouped doesn't show the queue.
- **Up and down** in the toolbar (shown disabled until now, `docs/decisions/0021`) move the selected downloads, each enabled only when it would change something.
- **Drag and drop:** waiting rows have a grip. Dragging it carries that download, or the selected waiting downloads of its priority if it's selected. A line shows where they'd land, only next to waiting downloads of the same priority. Near the list's edge the list scrolls. The arrow keys on a focused grip move it one place. Pointer events rather than HTML drag and drop, so it works with touch too. Phones hide the grips; up and down still work there.
- **Priority icons** beside the name: a flame for Urgent, a big up arrow for High, a big down arrow for Low, none for Normal. Unlike the toolbar's chevrons, which move downloads in the queue.
- **The list starts oldest first** (date added, ascending), and **the Added column** shows the local date and time to the second (`dateStyle: short, timeStyle: medium`). The detail panel says it the same way.
- The Dashboard's "Up next" follows the queue. Top and bottom exist in the API only for now.

**Alternatives:**
- **The place replaces priority** in the queue, as in uGet: what you see is the order. But priority would then only share bandwidth, and an Urgent download would no longer start next.
- **Moving past another priority takes it on** (a Normal moved above a High becomes High): one visible order, but changing the place would quietly change the bandwidth share too. Dropping is refused there instead.
- **A "Queue" sort key** for the whole list (tried first): the queue only showed when picked, and up/down had to switch the list to it.
- **`PATCH /jobs/{id}` with `before`**: the client has to know the priority rule, and a selection takes one request per download.
- **`PUT /queue`** with the whole order: the client computes everything, and a download added meanwhile could be lost from the order.
