#!/usr/bin/env bash
# verify_wasm_hash.sh — Reproducible build verification for TrustLink
#
# Builds the contract in the canonical reproducible environment and compares
# the SHA-256 hash of the resulting WASM to a known-good reference hash.
#
# Usage:
#   ./scripts/verify_wasm_hash.sh [KNOWN_HASH]
#
# If KNOWN_HASH is omitted the script prints the hash of the freshly-built
# WASM and exits 0 — use this to record the hash after a trusted first build.
#
# Reproducible build environment:
#   Rust toolchain : stable (pinned via rust-toolchain.toml)
#   Target         : wasm32-unknown-unknown
#   Profile        : release
#   Optimiser      : stellar contract optimize (wasm-opt -Oz)
#   OS             : Linux x86_64 (use Docker for cross-platform consistency)
#
# Example — record hash after initial trusted build:
#   ./scripts/verify_wasm_hash.sh
#
# Example — verify against a stored hash:
#   KNOWN="e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
#   ./scripts/verify_wasm_hash.sh "$KNOWN"

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WASM_PATH="$REPO_ROOT/target/wasm32-unknown-unknown/release/trustlink.wasm"
OPT_WASM_PATH="$REPO_ROOT/target/wasm32-unknown-unknown/release/trustlink.optimized.wasm"
KNOWN_HASH="${1:-}"

# ── 1. Build ─────────────────────────────────────────────────────────────────
echo "==> Building TrustLink WASM (release, wasm32-unknown-unknown)..."
cd "$REPO_ROOT"
cargo build --target wasm32-unknown-unknown --release 2>&1

if [[ ! -f "$WASM_PATH" ]]; then
  echo "ERROR: WASM artifact not found at $WASM_PATH" >&2
  exit 1
fi

# ── 2. Optimise ───────────────────────────────────────────────────────────────
echo "==> Optimising WASM..."
if command -v stellar &>/dev/null; then
  stellar contract optimize --wasm "$WASM_PATH" --wasm-out "$OPT_WASM_PATH" 2>&1
  FINAL_WASM="$OPT_WASM_PATH"
elif command -v wasm-opt &>/dev/null; then
  wasm-opt -Oz "$WASM_PATH" -o "$OPT_WASM_PATH" 2>&1
  FINAL_WASM="$OPT_WASM_PATH"
else
  echo "WARNING: neither 'stellar' nor 'wasm-opt' found; skipping optimisation." >&2
  echo "         Hash will be of the unoptimised WASM." >&2
  FINAL_WASM="$WASM_PATH"
fi

# ── 3. Hash ───────────────────────────────────────────────────────────────────
echo "==> Computing SHA-256..."
if command -v sha256sum &>/dev/null; then
  ACTUAL_HASH=$(sha256sum "$FINAL_WASM" | awk '{print $1}')
elif command -v shasum &>/dev/null; then
  ACTUAL_HASH=$(shasum -a 256 "$FINAL_WASM" | awk '{print $1}')
else
  echo "ERROR: no sha256sum or shasum tool found." >&2
  exit 1
fi

WASM_SIZE=$(wc -c < "$FINAL_WASM")
echo ""
echo "  File : $FINAL_WASM"
echo "  Size : ${WASM_SIZE} bytes"
echo "  SHA256: $ACTUAL_HASH"
echo ""

# ── 4. Compare ────────────────────────────────────────────────────────────────
if [[ -z "$KNOWN_HASH" ]]; then
  echo "No reference hash provided. Record the hash above as your known-good value."
  echo ""
  echo "  To verify on the next build:"
  echo "    ./scripts/verify_wasm_hash.sh $ACTUAL_HASH"
  exit 0
fi

if [[ "$ACTUAL_HASH" == "$KNOWN_HASH" ]]; then
  echo "✅  PASS — WASM hash matches the known-good reference."
  exit 0
else
  echo "❌  FAIL — WASM hash MISMATCH." >&2
  echo "  Expected : $KNOWN_HASH" >&2
  echo "  Actual   : $ACTUAL_HASH" >&2
  echo "" >&2
  echo "  The built WASM does not match the reference. Possible causes:" >&2
  echo "    - Different Rust toolchain version (check rust-toolchain.toml)" >&2
  echo "    - Uncommitted source changes" >&2
  echo "    - Different optimiser version" >&2
  echo "    - Non-Linux build environment (use Docker for reproducibility)" >&2
  exit 1
fi
