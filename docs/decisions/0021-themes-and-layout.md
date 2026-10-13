# 0021: Four themes, tabs in the title bar

Accepted 2026-10-13.

**Decision:** the UI follows the mockups Mathieu chose (Design canvas "Ottie UI explorations", mockup G and the theme rounds).
- **Themes:** two pairs, each a light and a dark theme: **Sea** (Seafoam light, Lagoon dark; the default) and **Neutral** (Pearl light, Slate dark). They differ only in colours. All of them use the **Outfit** font (DM Mono for numbers in fixed columns), sentence-case labels and rounder corners (8 / 12 / 18 px). The colours live in `ui/src/app.css` as variables under `[data-theme='…']`. The page sets `data-theme` on `<html>`, and any element can set it to show another theme inside it (the previews in Settings).
- **Appearance per device:** light / dark / follow the system, plus the palette, are saved in the browser (`localStorage`). A phone can be dark while the desktop is light. An option to sync the choice across devices through the daemon comes with the settings API (slice 2). Then the daemon holds a shared appearance, and each device chooses whether to follow it.
- **Fonts are bundled** (`@fontsource-variable/outfit`, `@fontsource/dm-mono`). They work offline and nothing is fetched from Google. Both are under the SIL Open Font License.
- **Layout:** the sections (Dashboard, Downloads, Categories, Settings) are tabs in the title bar. Medium widths show icons only; phones get a bar along the bottom. The left menu is gone. The title bar has no line under it. The total speed sits on its right (the global limit joins it in slice 2). The add bar is a card of its own at the top of the Dashboard and the Downloads page only.
- **Features the daemon can't do yet** (queue up/down, Save as view, Play while downloading, the file map) are shown disabled where the mockups put them, rather than hidden. This replaces the earlier "no coming-soon pages" rule for these controls only. Whole pages still appear only when they work.

**Alternatives:**
- One theme with a dark variant (the earlier cream/teal look): judged too austere.
- Google Fonts: needs internet, and tells Google each time the UI opens.
- Saving the appearance only in the daemon: one choice for every device, and the page would have to load settings before picking colours.
