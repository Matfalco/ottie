# 0042: Pages written right to left

Accepted 2026-10-15.

**Why:** Arabic, Persian, Hebrew and Urdu are read from the right, so apps in those languages mirror their whole layout, as Android, Apple's and Windows' guidelines say. Japanese, Chinese and Korean are written left to right on screen and need nothing of this.

**Decision** (Mathieu's choices):
- **Languages** (slice L6): Arabic (`ar`, Modern Standard), Persian (`fa`), Hebrew (`he`) and Urdu (`ur`). «…» quotes in Arabic and Persian, straight quotes in Hebrew and Urdu; Arabic and Persian question marks and commas (؟ ،), Urdu's full stop (۔); Hebrew in a gender-neutral style (buttons as nouns, "יש ל…" for instructions). Counts as labels, as before (0041): Arabic has six plural forms.
- **Everything mirrors**, as Material Design does: the page gets `dir="rtl"` (`direction()` in `ui/src/lib/languages.ts`, set with `lang` in `+layout.svelte`), and every left/right in the CSS is a logical property (`margin-inline-start`, `inset-inline-end`, `text-align: start`…), so layout, tables, menus, the title bar and the tabs flip on their own. Bars fill from the right, the file map starts on the right, the week view starts on the right, and the speed chart has "now" on the left (its drawing is flipped with `scaleX(-1)`, its pointer read by `secondsAt`). The window's buttons go on the left, as in Windows in those languages. What a page reads from the pointer stays right: the week view's drag measures the gap between two day columns, which turns negative.
- **Names, links and folders keep their own direction.** An English name in a right-to-left line would otherwise be reordered ("file (1)" → "(file (1"): elements showing them, and fields for them, have `dir="auto"`, and a global `text-align: match-parent` keeps them lined up with the text around them. In the four message files, every placeholder sits between U+2068 and U+2069 (an isolate), and so do codes like `http://` and `[001-120]`; a test checks every placeholder.
- **Digits:** Arabic in Western digits (123), as most Arabic software today, in every browser (`ar-u-nu-latn` in `INTL`); Persian in its own (۱۲۳) and its own calendar, as `Intl` writes it; Hebrew and Urdu in Western digits. HTTP codes stay as they are (404).
- **Fonts:** the system's (Outfit draws neither script), for numbers in columns too: a monospace font can't join Arabic letters.
- **The tray** (`app/src/texts.rs`): Windows draws its tooltip and menus left to right, so each text of these languages comes wrapped in a right-to-left embedding (U+202B … U+202C), once its parts are joined. Persian counts in its own digits (`digits` in its file).

**Alternatives:**
- Mirroring only the layout, with bars, file map, chart and week view left to right like a media timeline: Mathieu chose to mirror them too.
- The window's buttons always on the right, as Windows puts them in other languages.
- Arabic-Indic digits (١٢٣), traditional in Egypt and the Levant.
- Isolating names in code (a function around every value put in a message): many call sites, and every language would get the marks; the marks in the right-to-left files touch nothing else.
- Arabic only in this slice: Persian, Hebrew and Urdu came with it, as the layout work is the same.
