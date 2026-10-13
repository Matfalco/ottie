# 0020: First-start setup, the Other category, and the category step when adding

Accepted 2026-10-13.

**First start:** the daemon starts with only General. The UI offers starter categories (Videos, Music, Images, Archives, Documents, Programs), named in the page's language, with the extensions of `filetypes.ts`, each in a subfolder of the downloads folder (General's folder). The user can untick, edit or add categories and change the downloads folder, then the UI creates them through the usual category endpoints. The daemon never creates them by itself: one daemon can serve front ends in several languages, and a NAS install may not want them.

**Remembering it:** `setup_done` in the `settings` table (migration 8), read with `GET /settings` (`{setupDone}`) and changed with `PATCH /settings` (only the fields given change). "Start" and "Skip" both set it, so the setup shows once per daemon, in whichever front end opens first. Databases that already have their own categories count as set up. `/settings` is the resource slice 4f grows (max concurrent, global limit, retry policy). A setup that stops halfway keeps what it created; trying again skips names that already exist.

**Other, and the folder deciding** (amends `docs/decisions/0011`):
- A second built-in category, **Other**, holds downloads saved in a folder that belongs to no category. Like General it can't be removed (`cannotRemoveBuiltIn`). It has only a name and limits: a folder or extensions sent for it are ignored. Migration 8 adds it with ID 2 on a new database and the next free ID on older ones (named "Other folders" if the user already has an "Other"), and marks both built-ins in a `built_in` column. The API shows it as `builtIn: "general" | "other" | null`.
- When a download names a folder but no category, the folder decides: the category the extension matches if that is its folder, else the first category with that folder, else Other. Folders compare by their parts (a trailing separator doesn't count; case doesn't on Windows). That category is fixed: the real file name doesn't re-match it.
- `POST /categories/match` also takes `destDir` and answers `byFolder`, so the add step can say "Automatic: Music (its folder)" or "Automatic: Other (a folder of no category)".
- General stays as it was: unmatched extensions, and its folder is the default one.

**Adding a link** takes a second step under the add bar: the category ("Automatic: Videos (.mp4)", from `POST /categories/match`, or any category) and an optional folder (the chosen category's folder by default). Enter accepts, Escape closes. Automatic sends no category: without a folder, the server's real file name can still move the download (`docs/decisions/0012`); with one, the folder decides (above). A picked category is the user's choice and stays. Other isn't offered as a choice: any folder of no category leads there.

**Alternatives considered:** a dedicated `GET /setup` + `POST /setup/done` (one more endpoint for a single flag), and no flag at all, showing the setup whenever only General exists (it would come back on every start for someone who wants only General). Creating the starter categories in the daemon was left out, see above.
