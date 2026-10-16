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
- **Third pass**, after a try in the desktop app: the pills' track grows to
  show the pills behind "…" and shrinks back (`reshapes`: growing, it's laid
  out at its new size and uncovered by a clip, so no pill changes line on the
  way); the details panel and its empty state take each other's place,
  growing or shrinking from the old height as the new one fades in (`swaps`);
  a settings section whose values arrive after it's shown grows to them; a
  list filled at once (the downloads arriving as the page starts) fades in as
  a whole. A segmented control's focus ring shows only from the keyboard (on
  a click it appeared at once, ahead of the sliding highlight). Changing the
  theme turns transitions off for that frame, so the whole page changes
  colour together.
- **Fixes** after another try: a sliding highlight sits in its control's own
  layer, under every item's text (each item was its own layer, so the
  highlight covered the labels it passed); a tab's page fades in from half
  seen, not from nothing (a blank frame between the pages); the details'
  Play and Copy link stay in place while the next download's link arrives,
  rather than going and coming back (the buttons beside them jumped).
- **None when the system asks for less motion** (`prefers-reduced-motion`),
  unless this device says otherwise: Settings → Appearance → Animations
  (Follow the system, On, Off), kept on the device, not shared. `data-motion`
  on `<html>` turns off the styles' own transitions too.

**Alternatives:**
- Livelier (springs, ~250 ms): more presence, but a download manager is
  glanced at, and waiting for motion gets old.
- Following the system only: no way to turn it off on a device whose system
  allows motion, or on where a system setting turned it off globally.
