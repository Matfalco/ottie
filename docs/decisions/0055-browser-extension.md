# 0055: The browser extension

Accepted 2026-10-17. Builds on `docs/decisions/0014` (tokens), `0029` (play links), `0036` (several links at once), `0052` (a token per device) and `0053` (a page always matches its daemon).

**Why:** a download manager is used mostly through the browser: downloads should go to Ottie without copying links by hand, with the cookies a site behind a sign-in needs.

**Decision** (Mathieu's choices: the popup is the daemon's page; every download taken over, with exceptions; a native messaging host when Ottie is installed, else a sign-in link; a commit per slice):
- **Chromium-based browsers and Firefox, desktop**, Manifest V3, one TypeScript code base (`extension/`, pnpm package `extension`) built twice: `dist/chrome` and `dist/firefox`, which differ only in their manifest.
- **What the browser sent goes with the download** (slice 8a): `POST /jobs` takes `cookie` (the `Cookie` header the browser has for that link), `referrer` and `userAgent`. The engine keeps them with the job (`Download::headers`, migration 21), sends them with every HTTP request of that download (first, resumed, each connection's range) and no other; reqwest leaves the cookie out on a redirect to another server. The API never gives them back, logs never show them (`RequestHeaders`'s `Debug` only says which are set). A value with a line break or over 32 KiB is refused (`invalidHeader`).

**In progress:** the steps after 8a are being built; this record grows with them.
