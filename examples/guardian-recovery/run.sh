#!/usr/bin/env bash
#
# End-to-end validation script for the DID-style guardian social-recovery
# example. Builds and runs the example against a fresh in-memory Soroban
# environment. The example `assert!`s guardian registration, the M-of-N quorum,
# and re-linkage of attestations to the new address, so a zero exit code means
# the full recovery flow succeeded.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

cd "${REPO_ROOT}"
echo "Running guardian social-recovery example..."
cargo run --quiet --example guardian-recovery "$@"
