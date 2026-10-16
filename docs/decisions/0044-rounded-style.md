# 0044: The rounded style, legacy palettes, and an hour of speed

Accepted 2026-10-16.

**Why:** a redesign kept every page's layout from the mockups of 0021 but reworked the shapes, from reference images Mathieu picked: pills instead of boxes, tabs and choices in a sunken track with the chosen one raised, tactile switches, the page on a sheet inset from the window, the link field holding its buttons. Along the way the Dashboard got a new arrangement and the speed graph a longer reach. The mockups are the Design canvas "Ottie Lagoon redesign".

**Decision** (Mathieu's choices):
- **Two styles, chosen as palettes.** The rounded style is the default; the look from before stays as two more palettes, Sea and Neutral tagged **Legacy** in Settings → Appearance, for now. `Palette` gains `seaLegacy` and `neutralLegacy` (API, `appearance_palette`; migration 16 rebuilds `settings` for its CHECK), so the choice is shared between devices like the rest of the look (0022). The page sets `data-style="rounded"` or `"legacy"` on `<html>` beside `data-theme`.
- **All four themes get the new shapes.** The rounded style's own surfaces are variables per theme in `app.css` (`--frame` around the page, `--sunken` and `--raised` with their shadows, `--glow`), and the shape variables change with the style (`--radius-*` larger, `--radius-pill` for buttons and fields, `--edge`: cards lose their outline).
- **Legacy is today's CSS, untouched.** Each component's own styles are the legacy look; the rounded one is added on top under `:global([data-style='rounded'])`. Dropping legacy later means folding those rules in and deleting what they override.
- **Layout and features are the same in every palette.** The Dashboard puts the speed graph across the whole width, then the Downloads page's four groups two by two, each with its icon, count and first five downloads: running ones fill their row as they arrive, waiting ones in queue order with Resume and up/down (this replaces "Up next"), then those needing attention and the latest finished. A row's progress bar in the download list is a small copy of the details panel's (0027), and a finished one is green.
- **The speed graph shows 5 min, 15 min, 30 min or 1 h**, chosen above it and remembered per device. Longer spans average neighbouring seconds, so it draws at most 300 points.
- **The daemon keeps the last hour of total speed** (`GET /speed`: bytes per second, oldest first, at most 3600), so a page just opened shows the whole hour. Its actor estimates each running job's speed from its progress reports exactly as the pages do (a moving average of what arrived between reports) and adds them up each second; in memory only, so it starts again with the daemon. A page fetches it with the lists, then adds its own samples from the progress events, so both parts of the line look alike.

**Alternatives:**
- A separate Style setting (rounded or legacy, any palette): one more choice in Settings for a look meant to go away; legacy palettes say the same in the palette list.
- Legacy keeping today's layouts too: every page whose arrangement changed would exist twice until legacy goes.
- Only Lagoon in the rounded style, the others legacy until they get mockups: the shapes don't depend on the colours, and a light theme without them would look older than the dark one.
- Keeping the hour in the page: no API change, but a page just opened would start empty, and the tray and a phone each from nothing.
- Counting the bytes of each whole second (where the speed limiter sees every chunk): exact, but a download held to a limit receives its bytes in chunks of a few hundred kB, so seconds caught none, one or two and the line zigzagged around the real speed; smoothing that on the page only lagged the live part too.
