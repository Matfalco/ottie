# 0019: The desktop window draws its own title bar

Accepted 2026-10-13.

**Decision:** the desktop app's window has no system title bar (`decorations(false)`). The page's top row takes its place, as in VS Code or Obsidian. It holds the logo, the add bar and Ottie's own minimize, maximize/restore and close buttons, drawn like Windows 11's.
- The empty parts of the row move the window when dragged and maximize it when double-clicked (Tauri's `data-tauri-drag-region`).
- In a browser the same row shows without window buttons. The page tells the two apart with `isTauri()` from `@tauri-apps/api`.
- The sign-in page has the row too, otherwise the window couldn't be moved or closed there.

**What the page may do:** 0017 gave the page no access to Tauri at all. It now gets exactly what the title bar needs, and nothing more: minimize, toggle maximize, close, "is it maximized", start dragging, the double-click maximize, and listening to window events (to swap the maximize and restore icons). Since 4d it may also open the system folder picker (`dialog:allow-open`), for "Browse…" next to a category's folder; that only hands back the path the user chose. The app grants this at runtime (`add_capability`) because the daemon's address is only known once it answers. The grant covers that origin only and the main window only. Downloads still go only through the HTTP API.

**Alternatives:**
- `tauri-plugin-decorum` keeps the real Windows buttons over our row, so hovering over maximize still shows Windows 11's snap-layout menu. Rejected: a third-party plugin, and more fragile. Dragging to the screen edges and Win+Z still snap.
- Keeping the system title bar and only matching its colour: not the look Mathieu wants.

**Other systems:** Linux gets the same drawn buttons. macOS (later) should keep its native traffic lights over our row (`titleBarStyle: Overlay`) rather than drawing Windows-style buttons.
