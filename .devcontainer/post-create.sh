#!/usr/bin/env bash
set -euo pipefail

echo "→ adding wasm32-unknown-unknown target"
rustup target add wasm32-unknown-unknown

echo "→ installing soroban-cli (locked, soroban-sdk 21.x compatible)"
cargo install --locked soroban-cli --version "^21"

echo "→ installing cargo-expand (Soroban macro debugging)"
cargo install cargo-expand

echo "→ installing cargo-watch (live rebuilds)"
cargo install cargo-watch

echo "→ pre-building project (warms up the cargo cache)"
cargo build
echo "→ ensuring stellar CLI (pinned 21.x) is installed"
if command -v stellar >/dev/null 2>&1; then
	echo "stellar already installed: $(stellar --version || echo 'unknown version') — skipping install"
else
	echo "→ installing stellar-cli (locked, pinned 21.x, features: opt)"
	cargo install --locked stellar-cli --version "^21" --features opt
	echo "→ installed: $(stellar --version || echo 'stellar binary not found')"
fi

echo "✓ devcontainer ready — run: cargo test"
