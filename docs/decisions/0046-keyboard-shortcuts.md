# 0046: Keyboard shortcuts

Accepted 2026-10-16. Builds on `docs/decisions/0032` (queue order) and `0045` (Enter opens a file).

**Why:** a download manager is used in short bursts: paste a link, pause something, move one up the queue. Downloads already had Ctrl+A, Escape, Delete and Enter; everything else needed the mouse.

**Decision** (Mathieu's choices):
- **On every page of the main window:** **1–5** go to the tabs (by the key's place, `KeyboardEvent.code`, so AZERTY's top row works without Shift; Ctrl+1–5 stay the browser's), **Ctrl+V** adds what was copied, **/** puts the cursor in the add field, **?** shows the list of shortcuts. A paste holding one link puts it in the add field with the category step open; several links (or a sequence) open the list for adding several (`0036`); text without a link is left alone. From Categories, Schedule or Settings, the link goes to Downloads first; the Dashboard keeps its own add field.
- **In Downloads,** as in a file manager: **↑/↓** select the previous or next download, **Shift** extends from the anchor, **Home/End** the first or last (with no row reached yet, ↓ starts at the top and ↑ at the bottom); **Ctrl+A**, **Esc** (select none), **Space** (pauses the selected ones that can; when none can, resumes them), **Enter** (opens the finished file, desktop app), **Delete** (removes, asking first when data would be lost); **Alt+↑/↓** move the selection in the queue, **Alt+Shift+↑/↓** to its top or bottom (Alt+Home is the browser's home page); **F** shows or hides Filters and sorting.
- **Never while typing,** and never a key the focused control uses: text fields, Space and Enter on buttons and links, Space on checkboxes, the arrows in menus, radio groups and sliders. No shortcut while a dialog is open (its own keys are its), nor during the first-start setup. Characters are read from `key` (`?` and `/` wherever the layout puts them, AltGr included), digits from `code`.
- **The list** (`?`, and a Keyboard shortcuts section in Settings): key names in the page's language (Maj, Suppr, Échap in French; Strg, Entf, Pos1 in German), Apple's ⌘ ⌥ ⇧ on macOS. Modifiers held for several keys are written once: Alt + ↑ / ↓.
- **The arrows' cursor** is part of the selection (`Selection.cursor`, beside the anchor): the last row clicked or reached. Reaching a row scrolls it into view (below the sticky header) and takes the focus off a row's checkbox, so Space then pauses rather than ticks it. A download moved in the queue is scrolled to once the list shows it in its new place.

**Alternatives:**
- Ctrl+1–5 for the tabs: familiar from editors, but a browser keeps them for its own tabs.
- Ctrl+Shift+V instead of a plain paste: harder to set off by mistake, but pasting a link is the shortcut people try first.
- Space pausing and resuming each download on its own: a mixed selection would go both ways at once.
- A keyboard button in the title bar for the list: no room to spare there; Settings has a section instead.
