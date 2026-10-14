# 0035: Notifications

Accepted 2026-10-14.

**Why:** downloads run for minutes or hours, often with the window hidden in the tray (`docs/decisions/0034`). Without a notification, people have to look to know they're done, or that one failed.

**Decision** (Mathieu's choices):
- **Finished downloads are notified once everything has finished** by default: one notification when nothing is running or waiting any more ("All downloads finished", "3 files downloaded"; the file's name when there's only one). Settings → Notifications can make it **one per download** as each finishes, or **never**. "Everything" means no download is connecting, downloading, retrying or queued. Paused ones don't count: when the last running one is paused or cancelled, the ones that finished before it are notified then.
- **A failed download is notified** on its own, at once: "Download failed", its name and why, in the page's words (`problem_…`, less sizes and times). A download fails after its last retry, or at once when retrying can't help (a 404). A switch turns this off, on by default. Downloads that are only retrying are not notified.
- **The desktop app notifies**, from what it already follows for the tray (`app/src/status.rs`): a download is news when the events show it change to completed or failed. The first list after the app starts isn't news, but a list fetched after reconnecting or a `resync` is compared with the one before it, so what finished while the daemon couldn't be reached is still told. Its texts are in `app/messages/`, in the page's language, like the tray's.
- **The settings are this computer's,** in the app's `app.json` beside closing to the tray, read and changed by the page through app commands (`desktop`, `set_notify`, `set_notify_failed`). The section only shows in the desktop app.
- **A click on a notification opens the window.**
- **Through `tauri-plugin-notification`** (Tauri's own plugin): it shows the system's notifications on Windows, macOS and Linux, reports clicks, and works on Android, where the app will need it too (step 8). On Windows, a dev build (`target/debug`) notifies as PowerShell, since only an installed app has its own identity; the installed one notifies as Ottie.

**Alternatives:**
- The settings in the daemon, shared like the appearance: the web UI and the Android app could follow them, but only the desktop app notifies so far, and on a shared daemon each computer may want its own.
- The page notifying through the browser's `Notification`: it would work in a browser too, but only while a tab is open and signed in. The tray already follows the daemon without the page.
- `notify-rust` directly: the same notifications on desktop (the plugin uses it), but nothing for Android.
- Not notifying while the window has the focus: less noise while watching the list, but a notification one didn't see is worse than one too many. It can come later as a setting.
