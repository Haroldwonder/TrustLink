#!/usr/bin/env bash
# Verify that every StorageKey::Variant referenced under src/ is defined in the
# StorageKey enum in src/storage.rs.
#
# Catches the class of bug where call sites reference StorageKey::Foo but Foo
# was never added to the enum (cargo check fails with a wall of errors; this
# script fails with a short, targeted list).
#
# Usage: scripts/check-storage-keys.sh
# Exit 0 when every usage is defined; non-zero otherwise.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STORAGE_RS="$ROOT/src/storage.rs"
SRC_DIR="$ROOT/src"

if [[ ! -f "$STORAGE_RS" ]]; then
  echo "ERROR: $STORAGE_RS not found" >&2
  exit 1
fi

# Extract variant names from `pub enum StorageKey { ... }` (first occurrence).
DEFINED="$(
  python3 - "$STORAGE_RS" <<'PY' | sort -u
import re, sys
text = open(sys.argv[1]).read()
m = re.search(r"pub enum StorageKey\s*\{", text)
if not m:
    sys.exit("failed to locate pub enum StorageKey")
i = m.end()
depth = 1
body = []
while i < len(text) and depth:
    c = text[i]
    if c == "{":
        depth += 1
    elif c == "}":
        depth -= 1
        if depth == 0:
            break
    body.append(c)
    i += 1
for line in "".join(body).splitlines():
    line = re.sub(r"//.*", "", line).strip()
    m = re.match(r"([A-Za-z_][A-Za-z0-9_]*)\b", line)
    if m:
        print(m.group(1))
PY
)"

if [[ -z "$DEFINED" ]]; then
  echo "ERROR: failed to parse any StorageKey variants from $STORAGE_RS" >&2
  exit 1
fi

USED="$(
  grep -RhoE 'StorageKey::[A-Za-z_][A-Za-z0-9_]*' "$SRC_DIR" \
    | sed 's/StorageKey:://' \
    | sort -u
)"

MISSING=()
while IFS= read -r variant; do
  [[ -z "$variant" ]] && continue
  if ! grep -qxF "$variant" <<<"$DEFINED"; then
    MISSING+=("$variant")
  fi
done <<<"$USED"

echo "StorageKey enum defines $(grep -c . <<<"$DEFINED") variant(s)"
echo "src/ references $(grep -c . <<<"$USED") distinct StorageKey:: variant(s)"

if ((${#MISSING[@]} > 0)); then
  echo ""
  echo "ERROR: StorageKey variants referenced in src/ but not defined in the enum:"
  for v in "${MISSING[@]}"; do
    echo "  - StorageKey::$v"
    grep -Rn "StorageKey::$v" "$SRC_DIR" | head -3 | sed 's/^/      /' || true
  done
  echo ""
  echo "Add the missing variant(s) to pub enum StorageKey in src/storage.rs."
  exit 1
fi

echo "OK: every StorageKey:: usage in src/ is defined in the enum."
