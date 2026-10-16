#!/bin/bash
# Gets a Claude Code cloud session (Ubuntu 24.04) ready to build and test Ottie.
# Run by the SessionStart hook in .claude/settings.json; on any other machine it
# does nothing. Each step is skipped when already done, so a resumed session
# starts fast. Never fails the session: a step that fails is reported and the
# session starts anyway.

if [ "$CLAUDE_CODE_REMOTE" != "true" ]; then
	exit 0
fi

cd "$CLAUDE_PROJECT_DIR" || exit 0

say() { echo "cloud-setup: $*"; }

# What the Tauri app needs to compile on Linux (the workspace includes it):
# WebKitGTK 4.1, the tray (appindicator), notification sounds (ALSA).
packages="build-essential pkg-config file libwebkit2gtk-4.1-dev \
	libayatana-appindicator3-dev librsvg2-dev libxdo-dev libssl-dev libasound2-dev"
missing=""
for p in $packages; do
	dpkg -s "$p" >/dev/null 2>&1 || missing="$missing $p"
done
if [ -n "$missing" ]; then
	sudo=""
	[ "$(id -u)" -ne 0 ] && sudo="sudo"
	say "installing$missing"
	{ $sudo apt-get update -qq && $sudo apt-get install -y -qq $missing >/dev/null; } ||
		say "apt-get failed: cargo build --workspace will fail on the app"
fi

# The workspace is edition 2024 and follows recent stable Rust.
if command -v rustup >/dev/null 2>&1; then
	rustup update stable --no-self-update >/dev/null 2>&1 && rustup default stable >/dev/null 2>&1 ||
		say "rustup update failed: using $(rustc --version 2>/dev/null)"
	rustup component add clippy rustfmt >/dev/null 2>&1
fi

# A debug build with full debug info fills the VM's 30 GB disk quickly.
if [ -n "$CLAUDE_ENV_FILE" ]; then
	echo 'export CARGO_PROFILE_DEV_DEBUG=line-tables-only' >>"$CLAUDE_ENV_FILE"
fi

# pnpm at the version package.json names, then the JS dependencies, again only
# when the lockfile or a package.json changed since the last install.
want=$(sed -n 's/.*"packageManager": *"pnpm@\([^"]*\)".*/\1/p' package.json)
if [ "$(pnpm --version 2>/dev/null)" != "$want" ]; then
	say "installing pnpm $want"
	npm install -g "pnpm@$want" >/dev/null 2>&1 || say "pnpm $want failed to install"
fi
hashes=node_modules/.cloud-setup.sha256
if ! sha256sum --check --status "$hashes" 2>/dev/null; then
	rm -f "$hashes"
	say "pnpm install"
	if pnpm install --frozen-lockfile >/dev/null 2>&1; then
		sha256sum .claude/cloud-setup.sh pnpm-lock.yaml package.json \
			ui/package.json app/package.json shared/api-types/package.json >"$hashes" 2>/dev/null ||
			rm -f "$hashes"
	else
		say "pnpm install failed: run it to see why"
	fi
fi

say "ready ($(rustc --version 2>/dev/null), node $(node --version 2>/dev/null), pnpm $(pnpm --version 2>/dev/null))"
exit 0
