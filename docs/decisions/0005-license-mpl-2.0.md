# 0005: License is MPL-2.0

Accepted 2026-10-12.

**Decision:** all of Ottie is licensed under the Mozilla Public License 2.0 (`LICENSE`, `license = "MPL-2.0"` in `[workspace.package]`).

**Options considered**
- MIT OR Apache-2.0: the Rust ecosystem default, fully permissive.
- GPL-3.0-or-later: strong copyleft that keeps all forks open, but other projects are less likely to reuse `ottie-core`.
- **MPL-2.0 (chosen):** file-level copyleft. Changes to Ottie's own files must stay open, but Ottie can be combined with code under any license, including our MIT/Apache dependencies, closed-source apps and app stores.

**Notes**
- The root `LICENSE` is the official text from mozilla.org.
- Crates published to crates.io don't include the root `LICENSE`, so their entry file carries the MPL "Exhibit A" header.
