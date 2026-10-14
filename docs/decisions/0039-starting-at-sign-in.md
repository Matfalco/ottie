# 0039: Starting at sign-in

Accepted 2026-10-14.

**Why:** a download manager is expected to be there after a restart: unfinished downloads should carry on without anyone opening Ottie.

**Decision** (Mathieu's choices):
- **Three choices**, in Settings → Desktop app (desktop app only), under "When you sign in to this computer":
  - **Start Ottie in the tray**: the app starts with its window hidden, so only the tray icon shows. It starts its daemon as usual (`docs/decisions/0017`), so downloads carry on, and the tray and notifications work.
  - **Start only the downloads**: the daemon starts on its own (`ottie serve`, not `--managed`), and the app quits at once. Nothing shows: no tray icon, no notifications, until Ottie is opened. The app then finds that daemon running, as it finds one started from a terminal, so quitting the app leaves it running.
  - **Don't start Ottie.**
- **The first-start setup offers it**, in the desktop app, with "Start Ottie in the tray" chosen. It's applied by either of the setup's buttons ("Skip" skips the categories). After installing, Ottie doesn't start at sign-in until the setup is done. A daemon whose setup was done elsewhere (in a browser) never shows it; Settings has the same choice.
- **One entry in the system, with one argument:** the app registers itself with `--autostart` (Windows: the `Run` key of the current user; Linux: a `.desktop` file in `~/.config/autostart`; macOS: a LaunchAgent). `app.json` says what it starts (`startup`: `tray` or `daemon`). Whether it starts at all is the system's entry, read each time, so turning Ottie off in Task Manager's Startup tab shows as "Don't start Ottie".
  - The entry is named "Ottie": Tauri's NSIS uninstaller already deletes the `Run` value named after the product (but not on an update). A development build uses "Ottie (development)", so trying it never takes over the installed app's entry.
  - At each start, an entry that's there is written again, so it follows the app when it moves (an AppImage, another install folder).
- **A daemon started on its own logs to `daemon.log`** in the app's log folder (`%LOCALAPPDATA%\io.github.matfalco.ottie\logs` on Windows), replaced at each sign-in. The app waits until it answers (or fails, with the dialog that a failed start shows, and its last error line), then quits.
- **Through `auto-launch`** (0.6), the library `tauri-plugin-autostart` wraps, used directly: the plugin writes the app's path unquoted (`C:\Users\Ann Lee\…\ottie-app.exe --autostart`), and gives no way to quote it. Windows asks the shell to guess where an unquoted path with spaces ends, and the Linux `.desktop` file breaks on it. The app quotes the path itself (Windows, Linux) and registers for the current user only. The plugin's JavaScript side isn't needed: the page uses the app's own commands (`desktop`, `set_startup`), like the other desktop settings.

**Alternatives:**
- Only starting the app: simpler, but Mathieu wanted the choice of the daemon alone, with nothing on screen.
- The daemon as a real service (Task Scheduler, a systemd user service, a LaunchAgent for `ottie serve`): it could restart on a crash and run before sign-in, but needs different code on each system, and the console `ottie.exe` would need a window of its own hidden on Windows. Starting it through the app gives the same result at sign-in with one entry. A service can come later for headless use (`ottie service install`).
- Two entries (one for the app, one for the daemon): no `app.json` setting, but two things to keep in step, and Task Manager would show two Otties.
- On by default after installing: the setup offering it, already chosen, was preferred to registering without asking.
- `tauri-plugin-autostart`: Tauri's own, but with the unquoted path above.
