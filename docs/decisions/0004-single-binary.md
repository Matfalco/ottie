# 0004: One `ottie` binary; `ottie serve` runs the daemon

Accepted 2026-10-12.

**Decision:** a single binary, `ottie` (package `crates/ottie`). `ottie serve` runs the daemon (core + api). Other subcommands (`ottie add <url>`, `ottie list`, …) are thin clients that talk to a running daemon over the HTTP API, like every other client.

**Alternative rejected:** a separate `ottied` daemon plus a thin `ottie` CLI, in the style of `sshd`/`ssh`. That would give a smaller CLI, and a NAS could install the daemon alone, but it costs one more crate and two binaries to version and ship in lockstep.

**Why:** one thing to install and one Tauri sidecar. `cargo install ottie` gets everything, which matches the crates.io name. The systemd unit is `ottie.service` with `ExecStart=ottie serve`.

**Consequence:** the CLI always contains the whole engine (a few MB). Splitting it out later is possible, because the CLI only uses the HTTP API.
