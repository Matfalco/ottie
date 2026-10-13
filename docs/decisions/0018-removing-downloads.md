# 0018: Removing downloads from the list

Accepted 2026-10-13.

**Decision:** `DELETE /jobs/{id}` (`Manager::remove`) takes a job off the list in any state.
- A running job is stopped first, the same way pause stops it, and the request answers once it has stopped.
- An unfinished job is cancelled on the way out, so its `.part.ottie` file is deleted.
- A completed file always stays on disk. Deleting it too would be a separate, explicit option, added later if it's wanted.
- The job's row and segments leave the database. Subscribers get `JobEvent::Removed` (API: `{"type": "removed", id}`).

**Why any state:** this is how uGet's Delete works, and it's one step instead of cancel-then-remove. Cancel stays available for clients that want to keep a "Cancelled" row.

**IDs:** removing the newest job must not let its ID come back (0006). The `settings` row keeps the highest ID handed out (migration 7, saved with each removal), and at start the generator continues after whichever is higher, that or the newest job left.

**Inside the manager:** `Stop::Remove` ranks above Cancel. Its target state is Cancelled, so a running job goes through the usual cancel path (stop, flush, delete the `.part.ottie` file) and is forgotten afterwards. If the job completes before it notices the stop, the file is kept, as with cancel. A pause or cancel request still waiting on that job gets `JobNotFound`.

**In the UI:** each row has Remove, and the toolbar removes the selected downloads. It asks first only when downloaded data would be lost. "Clear finished" removes the completed and cancelled jobs in the current view, with one request per job.
