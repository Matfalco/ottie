# 0023: Filters and sorting on the Downloads page

Accepted 2026-10-13.

**Decision:** the Downloads pane follows mockup G (`docs/decisions/0021`): quick-filter pills above the list, a "Filters and sorting" panel that opens in their place, and the list grouped by status.

**Filters live in the address** (Mathieu's choice): `?status=retrying&status=failed&category=3&q=iso&added=today`. Each parameter may repeat; a missing one means "any". The back button, bookmarks and the Dashboard's links all open a filtered list. Changes replace the history entry rather than adding one.
- **Status** lists the job states, with Connecting counted as Downloading (it lasts only until the server answers). A list that is present but names nothing known (`?status=`) shows nothing. That is what unticking every box means, and it survives a reload. Ticking every box removes the parameter.
- **Category** lists IDs. **Name contains** matches the shown name, in any case. **Added** is `today` (since local midnight) or `week` (the past 7 days), read from the job's ID.
- **Quick filters** are fixed presets: All, Active (downloading or retrying), Needs attention (retrying or failed), Queued, Completed. Behind "…" are Paused, Added today and one pill per category that has downloads. A pill is lit when the filters are exactly its preset. Saved views (later, stored in the daemon) will add pills of their own; until then "Save as view…" is shown, disabled.

**Sorting and grouping are kept per device** (browser storage), so they stick between visits. Sort by date added (default, newest first), name (natural order, any case), size or progress. Each key starts in its own direction, and clicking a column header again turns it around. Unknown sizes and progress go last in both directions. Ties go to the newest.

**Four status groups**, on by default: Downloading, Needs attention (retrying, failed), Waiting (queued, paused) and Finished (completed, cancelled). The Dashboard's tiles are the same four, in the mockup's order, and link to them. That replaces the old All / Active / Queued / Stopped / Completed filters.

**Alternatives:**
- Everything in the address: fully bookmarkable, but opening Downloads from the tab bar would reset the sort each time.
- Everything per device: no links to a filtered list, so the Dashboard tiles would need another way in.
- Named presets in the address (`?status=attention`): shorter links, but two ways to say the same thing, and the panel's checkboxes could produce combinations no preset names.
