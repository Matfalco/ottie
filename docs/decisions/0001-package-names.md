# 0001: Cargo package names use a `dm-` prefix

Accepted 2026-10-12. **Superseded by [0003](0003-project-name-ottie.md)** the same day, when the project was named Ottie.

**Decision:** Cargo packages are named `dm-core`, `dm-api` and `dm-daemon`. Their folders stay `crates/core`, `crates/api` and `crates/daemon`.

**Why:** a package called `core` would shadow Rust's built-in `core` crate, which every crate can refer to by that name. "dm" (download manager) is a placeholder until the project has a name.

**Consequence:** renaming later means changing package names and `use dm_core::…` paths. That's a mechanical find-and-replace.
