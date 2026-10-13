# 0022: The settings API, and the look and language shared by all devices

Accepted 2026-10-13.

**One resource:** `GET /settings` answers every setting: `maxConcurrent`, `globalLimit` (bytes per second, 0 = none), `maxRetries`, `retriesBeforeYield`, `firstRetryWaitMs`, `setupDone`, `appearance` and `language`. `PATCH /settings` takes any of them. The engine applies a change in one actor command (`Manager::change_settings`). If anything changed, it saves the row once, puts the change to use (the limiter, the queue) and broadcasts `JobEvent::SettingsChanged` with all the settings. The API sends that as a `settingsChanged` event, so the title-bar chip and the Settings page stay current in every window and on every device. The older setters (`set_global_limit`…) stay as one-line wrappers.

**Ranges:** the daemon accepts what the UI offers, and nothing else: downloads at once 1–50, retries before giving up 0–1000, retries before giving way 0–100, first wait 0.5 s–1 h. The API refuses anything outside with `outOfRange { setting, min, max }` (400). The whole change is refused, so nothing is half-applied. The ranges are written once, in `crates/api/src/settings.rs`. The tests that generate the TypeScript types also write them to `shared/api-types/Ranges.ts` (Mathieu's choice), so the UI's fields check them before sending anything. That is safe because the page always comes from the daemon it talks to: both come from the same build. Other clients learn them from `outOfRange` refusals. The engine itself doesn't check them: its tests retry after 20 ms.

**Shared look and language** (completes `docs/decisions/0021`):
- The daemon keeps one look, `{ mode: system | light | dark, palette: sea | neutral }`, and one language, both null until a device shares its own. Mathieu chose typed fields for the look rather than a blob for the UI to fill: two nullable columns with `CHECK` lists (migration 9), and the API refuses unknown values (422, like any body it can't read). A new palette means a small migration and one enum variant. The language is a tag like `en` or `pt-BR`, checked only for its shape (`invalidLanguage`, 400; a `CHECK` on length and characters). That way adding a language stays one JSON file in the UI, and an older daemon accepts a newer UI's language. The engine only stores them.
- **Every device follows them by default** (Mathieu's choice). A "Same on all devices" switch in the header of the Appearance card turns that off for one device, which then keeps its current look and language. The switch is saved in the browser with the device's own look.
- A following device shares its look and language while none are shared. Otherwise it takes on the shared ones: the look at once, the language by reloading the page. A shared language the page doesn't have leaves the device in its own. Changing either on a following device sends it to the daemon first.

**Speed units:** limits are typed in kB/s, MB/s or GB/s (the total limit, categories). A unit is 1024 of the one below, as everywhere on the page, and is labelled kB/MB/GB, which is what Windows, uGet and browsers show. Mathieu kept that over 1000-based MB (sizes would differ from Explorer) and MiB labels (exact, but technical). A saved limit shows in the largest unit it has at least one of. The value shown while the total limit is off, and set again when it's switched back on, is remembered per device (browser storage, 10 MB/s the first time).

**Alternatives:**
- A JSON blob the daemon stores for the front ends without reading it: later UI preferences would sync with no daemon change, but nothing would be checked, and every client would have to cope with junk.
- A typed list of languages: every new language would need a migration and a new daemon.
- Following the shared look only after switching it on: Mathieu wanted one setup for all devices unless a device opts out.
- One endpoint per setting: more routes, and several saves and events for one change.
- Ranges in the `GET /settings` answer: any client could read them, but they'd be one more field in the contract, and the page would have to wait for the daemon before checking a value.
