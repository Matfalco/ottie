# 0053: Ottie on other computers: a list and a switcher

Accepted 2026-10-17. Builds on `docs/decisions/0017` (the desktop app on the daemon's page), `0034` (the tray), `0040` (when everything has finished), `0043` (the clipboard), `0045` (opening files) and `0052` (a token for each device).

**Why:** with Ottie on a NAS or a server (`0051`, `0052`), people still sat at their own computer: the desktop app only showed this computer's Ottie, and a browser had one page per address with no way between them.

**Decision** (Mathieu's choices: a browser goes to the other page; the tray follows the one shown; tokens in the system's keychain; this computer's Ottie always starts unless a setting says otherwise):
- **The desktop app** keeps a list in `daemons.json` (its config folder): a name, the address (its root, no token) and an ID never given again. Each token goes in the system's keychain (Credential Manager, Keychain, Secret Service; the `keyring` crate), under `daemon-<id>`; no page gets it back. A page adds one from a sign-in link (`https://nas…/#token=…`, as Settings → Connection or `ottie device add` give them): the app adds it once it answers and takes the token, else says why (nothing answered, not Ottie, token refused, keychain). Switching (`switch_daemon`) checks the same, then moves the main window to that Ottie's own page, signed in; removing the one shown goes back to this computer's first. The window opens on the one shown last, or this computer's if it doesn't answer (a notification says so). While another's is shown, the tray's system menu (there whenever the one shown can't be reached) has "Show this computer's Ottie": a way back even when that page can't load.
- **Its pages, only as trustworthy as that computer:** another computer's Ottie's pages get a capability of their own with the window, its title bar, the tray menu, the window for copied links, the tray's language and switching, never this computer's files, programs, settings, sounds or power (`REMOTE_PERMISSIONS`). The app tells every page this computer's Ottie's address (`window.__OTTIE_LOCAL__`), so another's leaves out what it can't do: the folder picker, Open and Show in folder, Settings' desktop sections, "When finished", the tray menu's choices and clipboard switch. Play still goes to the player chosen here (a play link needs no token).
- **Follows the one shown:** the tray's counts, speed, tooltip (with its name) and actions, notifications, and copied links (checked and added there, its window that Ottie's page). "When everything has finished" stays with this computer's downloads, whatever the window shows: the app follows this computer's Ottie for it all along, and another's only while shown.
- **This computer's Ottie** starts with the app as before, so its downloads carry on. Settings → Desktop app can say "only once shown": the app then opens on the one shown last and starts this computer's when switched to (until it quits).
- **HTTPS:** the app's client and its events stream speak HTTPS with the system's certificates (rustls with the platform verifier, as the engine), for Tailscale and proxies; another computer's Ottie may be behind a proxy, this computer's never.
- **In a browser,** each address keeps its own list in the browser (`localStorage`): a name and an address. Switching goes to the other Ottie's own page, which remembers its own token; a sign-in link's token is kept only until it's first opened. No CORS, and each page always matches its own daemon's version.
- **Where:** a switcher beside the logo once there's another one (which one this is, the others, "Manage the list…"), folding to its icon with the title bar; the list in Settings → Connection ("Ottie on other computers").

**Known limits:**
- Each address keeps its own page storage: the look and language chosen on this device apply per Ottie shown (the shared ones follow each daemon's settings).
- A page of another computer's Ottie that reloads while it's unreachable shows the webview's error, without the title bar or the switcher: the tray's "Show this computer's Ottie" is the way back.
- Another computer's Ottie must have this version or later for its page to have the switcher (none released lacks it).

**Alternatives:**
- One page using another daemon's API (CORS): breaks when the two run different versions of Ottie, and an HTTPS page can't reach an HTTP daemon.
- Tokens in a file beside `app.json`, like the token file: no new dependency, but in a plain file, and in backups.
- Notifications and a summed tray from every daemon: one stream each and much more in `status.rs`; one at a time is clearer.
- "When everything has finished" following the one shown: it's this computer that sleeps or shuts down.
- Never starting this computer's Ottie while another is shown: its downloads would wait unseen.
- Carrying a browser's list (and tokens) from page to page: another computer's page would get this computer's token.

**Tried so far** (in a Linux cloud session, 2026-10-17):
- Unit tests: sign-in links (root and token; not `file:`, `javascript:`, a spoiled token), names, IDs never coming back, the list file; the switcher's rows in the app and in a browser, a browser's list (adding, this page's own address refused, the token kept until opened), refusals in words (en, fr).
- The pages in headless Chromium on a test daemon, in a browser: Settings → Connection (adding by link and by address, this page's own refused), the switcher open, at phone width (folded), in Arabic.
- `cargo clippy -D warnings` and the app's tests; the UI's check, lint and tests (one test needs Node 24's `Intl.Locale#getTextInfo`, missing from the session's Node 22).

**Still to check** (the desktop app can't be seen in the cloud session):
- The desktop app on Windows: adding another computer's Ottie (Tailscale, or another `ottie serve --address` on the network with a device's link), switching both ways from the title bar and from Settings, removing the one shown, the window after a restart (it opens on the one shown last; unreachable: a notification, then this computer's).
- On another computer's page in the app: no folder picker, no Open or Show in folder, no desktop sections in Settings, no "When finished"; the tray menu its downloads only; Play going to the player chosen here; the window's buttons and dragging.
- The tray's "Show this computer's Ottie" with the other computer's Ottie stopped (its system menu, on a right click), also once its page has reloaded onto the error.
- Another computer's Ottie added with a read-only device link: its pages read-only, the tray menu's Pause all, Resume all and limit greyed out.
- The tray, notifications and copied links following the one shown; "When everything has finished" still acting on this computer's downloads while another is shown (`OTTIE_POWER_DRY_RUN=1`).
- The Credential Manager entries (`daemon-<id>.io.github.matfalco.ottie` or similar), and whether the antivirus minds the app reading them; on Linux, with GNOME Keyring or KWallet running.
- "Only once shown": starting with the window on another computer's, then switching to this one (its daemon starts then).
- An HTTPS address with a real certificate (Tailscale), and the events stream over it.
- The new texts in the other 41 languages (and the app's four) were drafted, not reviewed.
