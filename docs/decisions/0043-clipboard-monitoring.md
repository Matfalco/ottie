# 0043: Clipboard monitoring

Accepted 2026-10-16.

**Why:** uGet can watch the clipboard: copy a download link anywhere and it offers to download it. People coming from uGet expect it, and it's the quickest way to hand Ottie a link from an app that isn't a browser (a chat, an email, a text file) until the extension comes, and on computers without it.

**Decision** (Mathieu's choices):
- **The desktop app watches, not the daemon.** The clipboard belongs to the computer in front of the user, and the daemon may run elsewhere (a NAS) or with no one at it. A page in a browser can't read the clipboard in the background, and Android 10+ forbids it to apps not in front (there, links come from the share sheet, in the Android step). The app asks the daemon what the text holds (`POST /jobs/preview`, 0036) and adds through the API like any other client: no API change.
- **Opt-in**, saved in `app.json` (this computer's, like notifications): off until turned on in Settings → Clipboard, in the tray menu ("Watch the clipboard", Ottie's and the system's), or in the first-start setup (desktop app only, off unless chosen).
- **Which links:** those to a file type a category lists (its extensions on the Categories page; General's own list counts, its role of taking unmatched types doesn't), not on the download list yet, and not the daemon's own (a play link, the sign-in link). Page links (`.html`, no extension) are left alone, and there's no second list of types to keep in step. Several links copied at once, or a sequence (0037), are offered together; more than the cap (`maxBatch`) are ignored.
- **What a copied link does,** chosen in Settings:
  - **Ask** (the default): a small window in the corner of the screen nearest the tray icon (where notifications show; bottom right without one), drawn by the daemon's `/caught` page like the countdown window (0040). It lists up to three names, each with its automatic category and site, and counts the rest. **Download** adds them in their automatic categories; **More…** opens the main window's list for several links (0036) on them, to choose categories and a folder; **Ignore**. Links copied while it's open join it. Left alone, it goes away after 20 seconds (not while the pointer is on it); one the daemon refuses stays, saying why.
  - **Add it right away** (uGet's quiet mode): added at once in its automatic category, and a notification says so (the name, or how many), or why not.
- **The window never takes the focus** (`focusable(false)`, Windows' "no activate" style): what one is typing in keeps it, and its buttons still take clicks. So it has no keyboard use; the main window's list has.
- **What isn't caught:** anything copied while an Ottie window has the focus (a download's link, the sign-in link), what was already in the clipboard when watching starts, text over 1 MiB or without `://` (not sent to the daemon at all), and on Windows anything a password manager marks for clipboard tools to leave alone (the `ExcludeClipboardContentFromMonitorProcessing` and `Clipboard Viewer Ignore` formats: KeePass, 1Password, Bitwarden…).
- **How it watches:** a thread of its own looks twice a second. On Windows it reads the clipboard only after the system's change counter (`GetClipboardSequenceNumber`) has moved; elsewhere it reads the text and compares. Reading goes through `arboard` (1Password's clipboard library, the one Tauri's clipboard plugin wraps), text only. On Linux with Wayland it uses the data-control protocol where the compositor has it (KDE, Sway, Hyprland…); GNOME on Wayland doesn't offer it, so there only copies from X11 apps reach Ottie.

**Alternatives:**
- Opening the main window on the add step (uGet's way): no new window, but it takes the focus at every copy.
- A notification only: Windows toasts from Tauri can't carry buttons, so it would be a click to open the main window, every time.
- Ottie's own list of file types for the clipboard (uGet's way), or every link: a second list to keep in step with the categories, or a window for every web page link copied.
- `tauri-plugin-clipboard-manager`: the same `arboard` behind a plugin and its permissions, for reading from Rust, which needs neither. `clipboard-master` (change events from the system): Windows' counter already makes looking cheap, and the other systems would still need polling or a protocol of their own.
- Listening for `WM_CLIPBOARDUPDATE` on Windows: no polling at all, but a hidden window of our own and its message loop, for a counter read twice a second.
