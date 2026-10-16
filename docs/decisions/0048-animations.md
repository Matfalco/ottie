# 0048: Animations

In progress (2026-10-16): the first pass below is in, but some animations are
still missing or barely visible in the desktop app; this needs further review
before the slice is done.

**Why:** things appeared, vanished and jumped: a dialog popped, a removed row
left a hole that closed at once, a download moved in the queue teleported.
Motion shows where things come from and go.

**Decision** (Mathieu's choices):
- **Quiet:** ~120 ms for what goes, ~180 ms for what comes, an ease that
  starts fast and settles. Dialogs grow from 96 % as they and their backdrop
  fade; menus and popups drop 4 px as they fade.
- **Lists slide** (`shuffle`, FLIP): rows move to their new places when the
  queue is reordered, the list is sorted or grouped, or a row comes or goes
  above them; a download that changes group slides there; new rows fade in,
  removed ones fade where they were. More than 8 at once (a filter, a page of
  links) appear and go without fading; a list of over 400 rows doesn't move.
  Downloads, the Dashboard, Categories and Schedule. An editor opening in place
  of a row uncovers itself as the rows under it slide down.
- **Also:** the details panel folds and the filters panel opens smoothly; the
  current tab's highlight and a segmented control's chosen one slide; a tab's
  page fades in; switches fade their colour.
- **Toggles** (second pass): the highlight also slides between the filter
  pills (saved views included) and the Settings section list; the pills behind
  "…" fade in as their row opens; hover, chosen, disabled and focus colours of
  links, buttons, labels, fields and table rows fade over 120 ms (one rule in
  `app.css`, which a component's own `transition` overrides); an expand/collapse
  chevron turns over instead of being swapped (`.chevron`, `.rises`); what
  appears where there was nothing (an error under a form, the fields a switch
  reveals, the tray's finish choices, a link field, the toolbar's count and
  "Clear finished") unfolds to its size (`unfold`, vertically or, in a row,
  sideways).
- **None when the system asks for less motion** (`prefers-reduced-motion`),
  unless this device says otherwise: Settings → Appearance → Animations
  (Follow the system, On, Off), kept on the device, not shared. `data-motion`
  on `<html>` turns off the styles' own transitions too.

**Alternatives:**
- Livelier (springs, ~250 ms): more presence, but a download manager is
  glanced at, and waiting for motion gets old.
- Following the system only: no way to turn it off on a device whose system
  allows motion, or on where a system setting turned it off globally.
