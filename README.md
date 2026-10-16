# Ottie

**Ottie tucks your files away.** Sea otters keep their food and a favourite stone in a pouch of skin; Ottie keeps your downloads safe until they're done.

Ottie is a cross-platform download manager, built as a modern replacement for [uGet](https://ugetdm.com/), which is no longer maintained.

> **Status: early development.** The download engine works (pause/resume, multi-connection downloads, speed limits, categories, retries, play while downloading). `ottie serve` runs it as a daemon with an HTTP API and a web UI on 127.0.0.1:24068, and a desktop app (Tauri) opens on it. There are no releases yet.

## Try it

```bash
cargo install ottie
ottie serve      # the daemon; Ctrl+C stops it, downloads continue at the next start
ottie token      # the API token, for clients
ottie open       # the web UI in your browser, signed in
```

To keep it running without the app, at sign-in or at boot (systemd, Task Scheduler, a Windows service, launchd, Docker), see [Running Ottie as a service](docs/service.md).

To write your own client or script, see [Talking to the Ottie daemon](docs/api/README.md).

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
