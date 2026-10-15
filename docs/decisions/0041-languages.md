# 0041: More languages, and the system's language by default

Accepted 2026-10-15.

**Why:** Ottie spoke English and French. A device always kept the first language it found: Paraglide saves the language it detects, and the first device to load the page shared its own through the daemon, so a French phone could end up in English because a PC set it first.

**Decision** (Mathieu's choices):
- **Languages:** those of the EU, Latin America, the major ones of Asia and Africa. No Russian for now. They come in batches, a slice each; Arabic comes last, in its own slice, because the whole layout has to be mirrored (right to left). The first batch: German, Spanish (Spain, `es-ES`), Latin American Spanish (`es-419`), Italian, Dutch, Polish, Portuguese (Portugal, `pt-PT`) and Brazilian Portuguese (`pt-BR`).
- **"System" is a choice, and the default.** The language picker (Settings, sign-in page) starts with "System (Français)": the page shows the language closest to the system's, else English. A language chosen on a device is kept by Ottie itself (`ottie.language` in the browser's storage, `ui/src/lib/languages.ts`), not by Paraglide: the page reads it through two custom strategies (`custom-chosen`, `custom-system`, in `ui/src/hooks.client.ts`) that save nothing, since Paraglide saves whatever its strategies return the first time.
- **Every window follows a choice made in another** (`followLanguage`, through the browser's `storage` event): the desktop app's tray menu stays loaded while hidden, so it reloads in the new language, as other tabs do.
- **The shared language can be "System" too:** in `PATCH /settings`, `"language": null` puts every following device back on its own system's language, and `null` in `GET /settings` means that. A following device no longer shares its own language when none is shared (it still does for the look).
- **Matching a system's language to ours** (`closestLanguage`, the system's order of preference): the exact tag, then a stand-in, then the bare language. `es-MX` and every other Spanish with a country besides Spain → `es-419`; plain `es` and `es-ES` → Spain's. `pt-BR` → Brazil's; plain `pt` and every other country (Angola, Mozambique…) → Portugal's. Chinese (later) by script: `zh-TW`, `zh-HK`, `zh-MO` and `zh-Hant` → traditional, the rest simplified. The tray (`app/src/texts.rs`) gets the page's own tag, so it takes the exact file, else the bare language.
- **Translations not yet checked by a native speaker say so:** under the picker, "No native speaker has checked this translation yet. Help improve it", linking to the project's issues (later a platform like Weblate). `REVIEWED` in `languages.ts` lists the checked ones (English, French). The desktop app opens Ottie's own pages on GitHub in the system's browser; no other link leaves the window.
- **Counts without plural forms:** languages like Polish need several plural forms ("2 pobrania", "5 pobrań"). Rather than plural variants (the page's messages can have them, the tray's Rust texts can't), translations say counts as a label: "Pobierane: {count}", "Zaznaczone pobierania: {count}", as English and French already mostly do.
- **Tests:** every file in `ui/messages/` has every English message with the same placeholders, a name in the picker, and a locale in `project.inlang`; every file in `app/messages/` has the same texts and placeholders as English, all the way down.
- **Fonts:** the first batch is in Latin script, which Outfit covers. Languages in other scripts (Greek, Cyrillic, Vietnamese, Asian and African scripts) will need the system's font through `:lang()`, or a second bundled font, when they come.

**Alternatives:**
- Paraglide's `localStorage` and `preferredLanguage` strategies: they can't tell "System" from a choice, since the first detected language is saved.
- Plural variants in messages: correct, but they need the raw number (counts are passed formatted), and the tray would need plural rules in Rust too.
- One Spanish and one Portuguese: fewer files, but Latin American and Brazilian users expect their own.
- Plain `es`/`pt` going to Latin America/Brazil (most speakers): Mathieu chose Spain's and Portugal's for the bare tags.
- Holding back languages until a native speaker checks them: they ship now, marked as unreviewed.
