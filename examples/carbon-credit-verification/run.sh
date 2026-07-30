#!/usr/bin/env bash
#
# End-to-end validation script for the carbon-credit verifier-of-verifiers
# example. Builds and runs the example against a fresh in-memory Soroban
# environment. The example `assert!`s every step of the trust chain, so a
# zero exit code means the two-tier chain (project -> auditor -> registry)
# validated successfully.
set -euo pipefail

# Resolve the repo root from this script's location so it works from anywhere.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

cd "${REPO_ROOT}"
echo "Running carbon-credit verifier-of-verifiers example..."
cargo run --quiet --example carbon-credit-verification "$@"
