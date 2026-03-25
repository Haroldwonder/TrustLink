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

echo "✓ devcontainer ready — run: cargo test"
