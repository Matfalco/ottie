# Ottie

**Ottie tucks your files away.** Sea otters keep their food and a favourite stone in a pouch of skin; Ottie keeps your downloads safe until they're done.

Ottie is a cross-platform download manager, built as a modern replacement for [uGet](https://ugetdm.com/), which is no longer maintained.

> **Status: early development.** The download engine works (pause/resume, multi-connection downloads, speed limits, categories, retries) and `ottie serve` runs it as a daemon with an HTTP API on 127.0.0.1:16889. A web UI and the desktop app come next; there's nothing to use day to day yet.

## Try it

```bash
cargo install ottie
ottie serve      # the daemon; Ctrl+C stops it, downloads continue at the next start
ottie token      # the API token, for clients
```

## Planned

- **Pause and resume**, including after a restart or crash, without ever mixing bytes from two versions of a file.
- **Multi-connection downloads.**
- **Speed limits** per download, per category and globally, adjustable while downloads run.
- **Categories, a queue and a scheduler**, as in uGet.
- **Everywhere:** a background daemon with a web UI, desktop apps for Windows and Linux (macOS later), and an Android app.
- **Browser extension** for Chromium-based browsers and Firefox that hands downloads over to Ottie.

Written in Rust, with a Svelte UI and Tauri for the desktop and Android apps.

## License

[Mozilla Public License 2.0](LICENSE).
