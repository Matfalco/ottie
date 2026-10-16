# 0047: One look: the legacy palettes go

Accepted 2026-10-16. Ends the legacy part of `docs/decisions/0044`.

**Why:** the look from before the rounded style was kept as two Legacy palettes "for now". Nobody needs it, and every component carried two sets of styles: the legacy ones underneath, the rounded ones on top under `[data-style='rounded']`, each change checked in both.

**Decision** (Mathieu's choice):
- **Two palettes again,** Sea and Neutral, in the rounded style. `Palette` loses `seaLegacy` and `neutralLegacy` (core, API, generated types). Migration 17 turns a saved legacy palette into its pair (`sea-legacy` → `sea`, `neutral-legacy` → `neutral`) and rebuilds `settings` for the narrower CHECK; a device that saved one in its own storage gets its pair too.
- **The CSS is folded,** not left underneath: each component's rounded rules became its plain rules, and what they overrode is gone. No `data-style` on `<html>`, no `[data-style='rounded']` selectors; the shape variables in `app.css` (`--radius-*`, `--radius-pill`, `--edge`) hold the rounded values.
- **Nothing changes on screen.** The fold was checked by recording the computed style of every element (and its `::before`/`::after`) in each page and state, in two themes and on a phone, before and after: identical. States that recording can't reach (hover, focus, the add step) were checked against the old cascade by hand.

**Alternatives:**
- Dropping the two palettes but keeping the legacy CSS underneath: a small diff, but dead styles in every component and a rule ("check both styles") that no longer means anything.
