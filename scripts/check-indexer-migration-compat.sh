#!/usr/bin/env bash
# Fail if a new Prisma migration would break a currently-deployed indexer that
# has not yet been upgraded (backward-compatibility gate for expand/contract).
#
# Rules enforced on SQL under indexer/prisma/migrations/:
#   1. No DROP TABLE / DROP COLUMN / DROP TYPE without an expand-phase
#      predecessor (flagged as destructive — must be contract-phase only and
#      annotated with `-- contract-phase` after a prior expand release).
#   2. No RENAME COLUMN / RENAME TABLE (breaking for running queries).
#   3. No ALTER COLUMN ... TYPE / SET NOT NULL without DEFAULT (breaks old
#      writers that omit the column / send old type).
#   4. ADD COLUMN ... NOT NULL must include a DEFAULT (old indexer writes
#      omit the column).
#
# Usage:
#   scripts/check-indexer-migration-compat.sh
#   scripts/check-indexer-migration-compat.sh --base origin/main
#
# With --base, only migrations added since that ref are checked (PR mode).
# Without --base, every migration SQL file is checked (full audit).

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MIG_DIR="$ROOT/indexer/prisma/migrations"
BASE_REF=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --base)
      BASE_REF="${2:-}"
      shift 2
      ;;
    *)
      echo "Unknown arg: $1" >&2
      exit 2
      ;;
  esac
done

if [[ ! -d "$MIG_DIR" ]]; then
  echo "ERROR: $MIG_DIR not found" >&2
  exit 1
fi

mapfile -t FILES < <(
  if [[ -n "$BASE_REF" ]]; then
    git -C "$ROOT" diff --name-only --diff-filter=A "$BASE_REF"...HEAD -- \
      'indexer/prisma/migrations/**/migration.sql' 2>/dev/null || true
  else
    find "$MIG_DIR" -name 'migration.sql' | sort
  fi
)

# Normalize to absolute paths when coming from git diff
NORMALIZED=()
for f in "${FILES[@]+"${FILES[@]}"}"; do
  [[ -z "$f" ]] && continue
  if [[ "$f" = /* ]]; then
    NORMALIZED+=("$f")
  else
    NORMALIZED+=("$ROOT/$f")
  fi
done

if ((${#NORMALIZED[@]} == 0)); then
  echo "OK: no migration SQL files to check."
  exit 0
fi

FAILURES=0

check_file() {
  local file="$1"
  local rel="${file#"$ROOT/"}"
  local sql
  sql="$(cat "$file")"
  local annotated_contract=0
  if grep -qiE '^[[:space:]]*--[[:space:]]*contract-phase' <<<"$sql"; then
    annotated_contract=1
  fi

  # Destructive ops require -- contract-phase annotation (post-expand only).
  if grep -qiE '\bDROP[[:space:]]+(TABLE|COLUMN|TYPE|INDEX)\b' <<<"$sql"; then
    if [[ "$annotated_contract" -ne 1 ]]; then
      echo "FAIL [$rel]: destructive DROP without '-- contract-phase' annotation"
      echo "      Expand first (add new schema), deploy new indexer, then contract in a later release."
      FAILURES=$((FAILURES + 1))
    fi
  fi

  if grep -qiE '\bRENAME[[:space:]]+(TABLE|COLUMN|TO)\b|\bALTER[[:space:]]+TABLE\b.*\bRENAME\b' <<<"$sql"; then
    echo "FAIL [$rel]: RENAME is not backward-compatible with a running indexer"
    echo "      Use expand/contract: add the new name, dual-write, then drop the old name later."
    FAILURES=$((FAILURES + 1))
  fi

  # SET NOT NULL without DEFAULT breaks old writers that omit the column.
  if grep -qiE 'ALTER[[:space:]]+COLUMN[[:space:]].*SET[[:space:]]+NOT[[:space:]]+NULL' <<<"$sql"; then
    if ! grep -qiE 'SET[[:space:]]+NOT[[:space:]]+NULL[^;]*DEFAULT|DEFAULT[^;]*SET[[:space:]]+NOT[[:space:]]+NULL' <<<"$sql"; then
      # Allow if the same statement block also has DEFAULT on a nearby line — best-effort.
      if ! grep -qiE 'ALTER[[:space:]]+COLUMN' <<<"$sql" | grep -qi 'DEFAULT'; then
        echo "FAIL [$rel]: SET NOT NULL without DEFAULT breaks currently-deployed writers"
        FAILURES=$((FAILURES + 1))
      fi
    fi
  fi

  # ADD COLUMN ... NOT NULL must include DEFAULT.
  while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    if grep -qiE 'NOT[[:space:]]+NULL' <<<"$line" && ! grep -qiE 'DEFAULT' <<<"$line"; then
      echo "FAIL [$rel]: ADD COLUMN NOT NULL without DEFAULT:"
      echo "      $line"
      FAILURES=$((FAILURES + 1))
    fi
  done < <(grep -iE 'ADD[[:space:]]+COLUMN' <<<"$sql" || true)

  # ALTER COLUMN ... TYPE is a breaking change for old readers/writers.
  if grep -qiE 'ALTER[[:space:]]+COLUMN[[:space:]].*TYPE\b|ALTER[[:space:]]+COLUMN[[:space:]].*SET[[:space:]]+DATA[[:space:]]+TYPE' <<<"$sql"; then
    if [[ "$annotated_contract" -ne 1 ]]; then
      echo "FAIL [$rel]: column TYPE change without '-- contract-phase' annotation"
      FAILURES=$((FAILURES + 1))
    fi
  fi
}

for f in "${NORMALIZED[@]}"; do
  [[ -f "$f" ]] || continue
  check_file "$f"
done

if [[ "$FAILURES" -gt 0 ]]; then
  echo ""
  echo "ERROR: $FAILURES backward-compatibility issue(s) found."
  echo "See docs/indexer-migrations.md for the expand/contract runbook."
  exit 1
fi

echo "OK: checked ${#NORMALIZED[@]} migration file(s) for backward compatibility."
