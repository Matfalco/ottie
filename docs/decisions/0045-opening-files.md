# 0045: Opening a download's file and folder

Accepted 2026-10-16. Builds on `docs/decisions/0017` (the desktop app) and `0031` (opening a player).

**Why:** a download manager's last step is opening what it downloaded. The details panel had a "Show in folder" button waiting for it, and there was no way to open a finished file at all. A web page can't open local files or folders, so it's the desktop app's job; and since the page may one day come from another daemon (7i), the app must not run whatever a page names.

**Decision** (Mathieu's choices):
- **Two app commands that take a job ID, never a path:** `open_file` and `show_in_folder`. The app asks the daemon where the file is (the new `GET /jobs/{id}`) with its own token and builds the path from the job's folder and name, refusing a name that's more than a name (`..`, a separator, a Windows stream). A page can only open what Ottie downloaded. The IDs are checked before they reach a URL.
- **Open** (finished downloads only): the system's app for the file's type (`open`). The details panel's main button for a finished download, a **double-click** on its row, and **Enter** with that one download selected (not while typing, nor on a focused button). On a download that isn't finished, a double-click does nothing more than a click.
- **A program asks first, in a dialog of the app's own** (native, over the main window, in the page's language from `app/messages/`), which no page can answer for: by extension (`.exe`, `.msi`, `.bat`, `.ps1`, `.js`, `.lnk`, `.sh`, `.AppImage`, `.deb`, `.py`…, one list for every system since files travel), and on Linux and macOS also any file allowed to run. Run or Cancel.
- **Show in folder:** the system's file manager with the file selected (`tauri-plugin-opener`'s `reveal_item_in_dir`, used from Rust only: Explorer on Windows, Finder on macOS, the file manager's D-Bus interface on Linux). An unfinished download selects its `.part.ottie`; without a file yet, just the folder. Neither there: the page says the folder isn't there any more.
- **In a browser,** neither button shows (nor does Enter or a double-click do anything).
- **The Mark of the Web:** on Windows the engine marks each finished file as from the Internet, as browsers do (a `Zone.Identifier` stream: zone 3 and the link without its query, fragment or credentials). Windows then warns before running an unknown program (SmartScreen), and Office opens documents in Protected View. Renames and moves keep the mark. The "Do not preserve zone information" policy turns it off, as it does for browsers; drives without streams (FAT32, exFAT) just don't get it.
- **Pause all / Resume all** move from the tray menu alone to the Downloads toolbar too (at its end, icons only when narrow). They act on every download, not only those shown: pausing the waiting ones first, then the running ones; resuming the paused ones (failed ones need a look first).

**Alternatives:**
- The page's own confirmation (the dialog used for Cancel and Remove): in the page's language with no new app texts, but the app would run whatever a page says was confirmed.
- Paths from the page, checked by the app against the daemon's folders: more for the app to get right, and the folders a job may use aren't limited yet (7h).
- `opener`'s `reveal` instead of the Tauri plugin: the same calls, but on Linux through `dbus` (the C library) rather than `zbus` (pure Rust).
- Writing the mark through Windows' `IAttachmentExecute` (what browsers call): it picks the zone per site and runs the antivirus, but needs COM and the window's handle in the engine, which runs without one.
- Double-click toggling pause/resume on unfinished downloads: one gesture doing two different things.
