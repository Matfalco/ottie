# 0031: Opening a media player

Accepted 2026-10-13. Builds on `docs/decisions/0029` (play links).

**Why:** Play opened the link in a browser tab, and browsers play only a few formats: MKV usually doesn't play, and Firefox never plays it. VLC and other players play nearly everything, but a web page can't start a program.

**Decision** (Mathieu's choice):
- **Desktop app: a "Play with" setting** in Settings → Playing, on this computer only:
  - **The system's player:** the app writes a playlist (`.m3u`) holding the link to its cache folder and has the system open it. Whatever opens playlists then plays the link: VLC once it's installed, else the system's own player. Playlists older than an hour are deleted.
  - **The browser:** a new tab, as before.
  - **A program** picked with the system's file picker (VLC, mpv…), started with the link as its argument.

  **The default is VLC** when it's where its installer puts it (Program Files on Windows, Applications on macOS, the `PATH` on Linux): otherwise Windows opens playlists with its own player until VLC is made the default app for `.m3u` files, a step people don't know about. Without VLC, the default is the system's player, and its hint says how to make VLC the playlist app. A setting without a program of its own gets VLC's, ready to choose.

  If the player can't start, the browser plays the link instead.
- **The setting belongs to the app**, not the daemon or the page: it's saved in the app's config folder (`player.json`). The page reads and changes it through three app commands (`player`, `set_play_with`, `choose_player`). Only the daemon's pages may call them, and only the native file picker sets the program. A page can switch between the three choices but never name what runs, which matters because the page may come from a remote daemon.
- **Web UI: "Open in player"** (in a browser, not in the app) builds the same playlist in the page and saves it through the browser's downloads. Opening that file plays the download in the system's player.

**Alternatives:**
- Starting the default program for the file's type (`.mkv`) instead of the playlist's: closer to what people expect, but it needs code for each system (Windows associations, `xdg-mime` and `.desktop` files), and Windows' own players don't all accept a link.
- A `vlc://` link: VLC doesn't register that scheme on desktop systems.
- `.m3u8` instead of `.m3u`: the same format in UTF-8, but the playlist is plain ASCII (the link is percent-encoded), `.m3u` is the extension every player and system associates, and `.m3u8` usually means an HLS stream.
- Only a hint for formats browsers rarely play: no new code, but no one-click VLC.
