#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRIPT="$ROOT_DIR/scripts/check-sdk-coverage.mjs"

echo "=== Running check-sdk-coverage tests ==="

# Test 1: Normal execution passes
echo -n "Test 1: Default coverage check exits 0... "
output=$(node "$SCRIPT" 2>&1)
echo "PASS"

# Test 2: Help flag exits 0
echo -n "Test 2: --help exits 0... "
help_output=$(node "$SCRIPT" --help)
if [[ "$help_output" == *"SDK Contract Coverage Checker"* ]]; then
  echo "PASS"
else
  echo "FAIL: Expected help output"
  exit 1
fi

# Test 3: Empty allowlist causes exit 1
echo -n "Test 3: Empty allowlist fails with code 1... "
tmp_allowlist=$(mktemp)
echo '{"typescript":[],"python":[]}' > "$tmp_allowlist"
set +e
node "$SCRIPT" --allowlist "$tmp_allowlist" >/dev/null 2>&1
exit_code=$?
set -e
rm -f "$tmp_allowlist"
if [ "$exit_code" -eq 1 ]; then
  echo "PASS"
else
  echo "FAIL: Expected exit code 1, got $exit_code"
  exit 1
fi

echo "=== All check-sdk-coverage tests passed! ==="
