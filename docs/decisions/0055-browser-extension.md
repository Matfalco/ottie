# 0055: The browser extension

Accepted 2026-10-17. Builds on `docs/decisions/0014` (tokens), `0029` (play links), `0036` (several links at once), `0041` (languages), `0052` (a token per device) and `0053` (a page always matches its daemon).

**Why:** a download manager is used mostly through the browser: downloads should go to Ottie without copying links by hand, with the cookies a site behind a sign-in needs.

**Decision** (Mathieu's choices: the popup is the daemon's page; every download taken over, with exceptions; a native messaging host when Ottie is installed, else a sign-in link; a commit per slice):
- **Chromium-based browsers and Firefox, desktop**, Manifest V3, one TypeScript code base (`extension/`, pnpm package `extension`, Vite + Svelte) built twice: `dist/chrome` (a service worker, `downloads.ui`) and `dist/firefox` (a background script, the fixed ID `ottie@matfalco.github.io`). Both use `chrome.*` with promises, which Firefox also has in Manifest V3, so no polyfill.
- **What the browser sent goes with the download** (8a): `POST /jobs` takes `cookie` (the `Cookie` header the browser has for that link), `referrer` and `userAgent`. The engine keeps them with the job (`Download::headers`, migration 21), sends them with every HTTP request of that download (first, resumed, each connection's range) and no other; reqwest leaves the cookie out on a redirect to another server. The API never gives them back, logs never show them (`RequestHeaders`'s `Debug` only says which are set). A value with a line break or over 32 KiB is refused (`invalidHeader`).
- **Taking over** (8b): on `downloads.onCreated`, a download goes to Ottie unless taking over is off, the extension isn't signed in, it's in a private window, its link isn't http(s) (blob:, data:…), an extension started it, it's no longer running, it's on Ottie's own address, the site says it's smaller than the chosen size (0 by default: none are), its site is one left to the browser (each covers its subdomains), or it came from a form's POST (the extension notes POSTs of pages and frames for a minute: Ottie would send a GET). Ottie gets the original link with the browser's cookies for it (from the store it came from: Firefox's containers have their own), the referrer and the browser's user agent.
- **Never dropped:** the browser's download carries on until Ottie has accepted the new one; only then is it cancelled and erased from the browser's list. Ottie unreachable, the token refused or the job refused: the browser keeps it. If the browser finished it first (a small file), it keeps it and Ottie's job is removed. Chromium-based browsers' download bubble is turned off while one is taken over (`downloads.setUiOptions`), then on again.
- **Signing in by link** (8b): the options page takes a sign-in link (`http://host:port/#token=…`, a device's from Settings → Connection, or `ottie link`'s) and keeps the address and token only once Ottie answers with it. It says whether Ottie answers, and opens Ottie's page signed in as this browser. Firefox may withhold the access to every site the manifest asks for: the page then offers to grant it.
- **The extension's pages** use the UI's code from `ui/src/lib` (`#ui/…`: components, `format`, `languages`, `theme`) and its texts (`ext_*` keys in `ui/messages`, in all 43 languages, so they're translated and checked with the rest). Their language is Ottie's shared one, else the browser's closest; their look is Ottie's shared one (kept by the extension from `GET /settings`), else the system's light or dark.
- **The extension's settings** (`chrome.storage.local`, this device only): the pairing (address, token), taking over on or off, the smallest size, the sites left to the browser.

**Known limits (so far):**
- Firefox opens its downloads panel when a download starts; the extension can't stop that (the entry is erased at once).
- A download whose link redirects to another server gets no cookies on that server (the browser may have had some).
- Firefox with first-party isolation on gives no cookies without the first party's name, which the downloads API doesn't say: such downloads go without cookies.
- Holding a key while clicking can't keep one download in the browser (the downloads API doesn't say which keys were held); the sites list or the switch does.

**Alternatives:**
- A popup of the extension's own (shows something offline, but drifts from the daemon's version and doubles the texts): the daemon's page was chosen (8c).
- Only categories' file types (as clipboard watching, `0043`): fewer surprises, but many "Download" buttons give links without an extension. Asking every time: one click more per file.
- `webextension-polyfill`: not needed now that both browsers have promises on `chrome.*`; `@types/chrome` gives the types (a dev dependency only).
- Pausing the browser's download while Ottie decides: Firefox can't resume one the server doesn't allow ranges for, which would drop it.

**In progress:** 8c (the popup), 8d (the native messaging host), 8e (the context menu).

**Tried so far** (in a Linux cloud session, 2026-10-17): the extension's unit tests; the Chrome build in Playwright's Chromium (headless, the extension loaded) against a test daemon and a small site that sets a cookie: a download behind the cookie went to Ottie, which fetched it with the cookie, the referrer and the browser's user agent, and the browser's list was empty after; a small file and a form's download stayed in the browser. The cookie isn't in the daemon's log.

**Still to check:** Firefox (not available in the cloud session); real Chrome and Edge's download bubble while one is taken over; a site behind a real sign-in; the new texts in the other 41 languages were drafted, not reviewed.
