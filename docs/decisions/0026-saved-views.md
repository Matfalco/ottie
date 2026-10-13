# 0026: Saved views

Accepted 2026-10-13.

**Decision:** a saved view is a name, the Downloads filters (`docs/decisions/0023`) and the list's arrangement (sort key, direction, grouped by status). The daemon keeps views, so every device and front end has the same ones. On the Downloads page, each view is a pill after the fixed quick filters.

**Stored as typed columns** (Mathieu's choice): a `views` table (migration 10) with a column per filter and per arrangement setting, plus `view_states` and `view_categories` for the two lists. `any_state` and `any_category` tell "any" apart from an empty list, which shows nothing. The daemon checks what it saves:
- Names are trimmed, must not be empty, and are unique ignoring case (`viewNameTaken`).
- Categories must exist (`categoryNotFound`).
- `connecting` is saved as `downloading`, the same as the page's Status filter does.
- Lists are kept in a fixed order without repeats, and the name text is trimmed.

**A view includes its sorting** (Mathieu's choice). Clicking its pill applies its filters and its arrangement. That arrangement then becomes the device's arrangement, as if picked by hand. A pill is lit only when both match, so re-sorting by a column header shows that the list no longer is the view.

**API:** `GET/POST /views`, `PUT/DELETE /views/{id}`. A PUT replaces the whole view; renaming sends the same filters with a new name. `viewChanged` / `viewRemoved` events keep every page in step. IDs are plain numbers, given out like category IDs. Views are listed oldest first, which is the order of their pills.

**Removing a category** replaces it with General in every view that lists it, because its downloads move there too. The view is saved before the category row is deleted. The foreign key would otherwise drop the category from the view, and a list left empty would show nothing.

**On the page:** "Save as view…" in the Filters and sorting panel asks for a name, saves what is shown, and closes the panel so the new pill appears lit. The lit view's pill has a "…" menu with Rename and Delete; deleting asks first. Reordering views can come later.

**Alternatives:**
- Saving the filter part of the address as text: less code, and new filter fields would need no migration. But the daemon couldn't check views or fix them when a category goes, and every other client would have to parse the address format.
- A list of views inside the settings row: fewest new parts. But every rename rewrites all the settings, and two devices editing at once overwrite each other.
- Filters only, with sorting kept per device: one place for sorting, but a view like "biggest first" couldn't be saved.
