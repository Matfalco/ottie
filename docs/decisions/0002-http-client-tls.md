# 0002: reqwest 0.13 with rustls (aws-lc-rs) and the OS certificate store

Accepted 2026-10-12.

**Decision:** `reqwest = { default-features = false, features = ["rustls", "http2", "system-proxy"] }`.

- `rustls`: TLS in pure Rust, using the aws-lc-rs crypto provider. `rustls-platform-verifier` checks certificates against the OS trust store, so corporate and user-installed CAs work on Windows, Linux and Android.
- `http2`: many CDNs prefer it.
- `system-proxy`: a download manager should respect the system proxy settings.
- Left out: `charset` (we never decode bodies as text), compression features (we request `identity` so byte ranges map to file offsets), and `default-tls`.

**Why not native-tls/OpenSSL:** it breaks the Android and Windows builds (see CLAUDE.md).

**Fallback:** if aws-lc-rs ever fails to cross-compile (e.g. for Android), switch to `rustls-no-provider` and install the `ring` provider. That changes no engine code.
