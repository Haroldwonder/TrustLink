#!/usr/bin/env bash
# changelog-preview.sh — Preview the next Release Please changelog entry and version.
#
# Parses conventional commits since the last release tag (or all commits if no
# tag exists) and prints the expected next version and changelog section that
# Release Please would generate, without creating a PR or modifying any files.
#
# Sections mirror release-please-config.json:
#   feat  → Features   (minor bump)
#   fix   → Bug Fixes  (patch bump)
#   perf  → Performance (patch bump)
#   docs  → Documentation
#   refactor → Refactoring
#   test, chore → hidden (not shown)
#
# Usage: bash scripts/changelog-preview.sh
#        make changelog-preview

set -euo pipefail

# ── Helpers ───────────────────────────────────────────────────────────────────

# Read current version from Cargo.toml
current_version() {
    grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/'
}

# Bump semver: bump_version <major> <minor> <patch> <bump_type>
bump_version() {
    local major=$1 minor=$2 patch=$3 bump=$4
    case "$bump" in
        major) echo "$((major + 1)).0.0" ;;
        minor) echo "${major}.$((minor + 1)).0" ;;
        patch) echo "${major}.${minor}.$((patch + 1))" ;;
        *)     echo "${major}.${minor}.${patch}" ;;
    esac
}

# ── Determine commit range ────────────────────────────────────────────────────

LAST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "")
if [ -n "$LAST_TAG" ]; then
    RANGE="${LAST_TAG}..HEAD"
    SINCE_MSG="since ${LAST_TAG}"
else
    RANGE="HEAD"
    SINCE_MSG="(no previous release tag found — scanning all commits)"
fi

# ── Collect conventional commits ─────────────────────────────────────────────

# Format: <hash> <subject>
mapfile -t COMMITS < <(git log "$RANGE" --format="%H %s" --no-merges 2>/dev/null || true)

declare -a FEAT_LINES=() FIX_LINES=() PERF_LINES=() DOCS_LINES=() REFACTOR_LINES=()
BUMP_TYPE="none"

for entry in "${COMMITS[@]}"; do
    hash="${entry%% *}"
    subject="${entry#* }"
    short="${hash:0:7}"

    # Parse: type(scope): description  OR  type!: description (breaking)
    if [[ "$subject" =~ ^([a-z]+)(\([^\)]*\))?(!)?:\ (.+)$ ]]; then
        type="${BASH_REMATCH[1]}"
        scope="${BASH_REMATCH[2]}"
        breaking="${BASH_REMATCH[3]}"
        desc="${BASH_REMATCH[4]}"

        # Check for BREAKING CHANGE in commit body
        body=$(git log -1 --format="%b" "$hash" 2>/dev/null || true)
        if [[ -n "$breaking" ]] || echo "$body" | grep -q "^BREAKING CHANGE:"; then
            BUMP_TYPE="major"
        fi

        # Format scope for display
        scope_display=""
        if [ -n "$scope" ]; then
            # scope is like "(storage)" — strip parens for display
            scope_inner="${scope:1:${#scope}-2}"
            scope_display="**${scope_inner}:** "
        fi
        line="* ${scope_display}${desc} ([${short}](../../commit/${hash}))"

        case "$type" in
            feat)
                FEAT_LINES+=("$line")
                [ "$BUMP_TYPE" != "major" ] && BUMP_TYPE="minor"
                ;;
            fix)
                FIX_LINES+=("$line")
                [ "$BUMP_TYPE" = "none" ] && BUMP_TYPE="patch"
                ;;
            perf)
                PERF_LINES+=("$line")
                [ "$BUMP_TYPE" = "none" ] && BUMP_TYPE="patch"
                ;;
            docs)
                DOCS_LINES+=("$line")
                ;;
            refactor)
                REFACTOR_LINES+=("$line")
                ;;
            # test, chore, ci, build → hidden per release-please-config.json
        esac
    fi
done

# ── Compute next version ──────────────────────────────────────────────────────

VERSION=$(current_version)
IFS='.' read -r MAJOR MINOR PATCH <<< "$VERSION"

if [ "$BUMP_TYPE" = "none" ]; then
    NEXT_VERSION="$VERSION"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  Changelog Preview  ${SINCE_MSG}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo "  No releasable commits found since ${LAST_TAG:-the beginning}."
    echo "  Release Please would not create a release PR."
    echo ""
    echo "  Current version: ${VERSION}"
    echo ""
    exit 0
fi

NEXT_VERSION=$(bump_version "$MAJOR" "$MINOR" "$PATCH" "$BUMP_TYPE")
TODAY=$(date +%Y-%m-%d)

# ── Print preview ─────────────────────────────────────────────────────────────

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Changelog Preview  ${SINCE_MSG}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "  Current version : ${VERSION}"
echo "  Next version    : ${NEXT_VERSION}  (${BUMP_TYPE} bump)"
echo "  Version bump    : ${BUMP_TYPE}"
echo ""
echo "────────────────────────────────────────────────────────────"
echo "  CHANGELOG.md entry that Release Please would generate:"
echo "────────────────────────────────────────────────────────────"
echo ""
echo "## [${NEXT_VERSION}](../../compare/v${VERSION}...v${NEXT_VERSION}) (${TODAY})"
echo ""

if [ ${#FEAT_LINES[@]} -gt 0 ]; then
    echo "### Features"
    echo ""
    for line in "${FEAT_LINES[@]}"; do echo "$line"; done
    echo ""
fi

if [ ${#FIX_LINES[@]} -gt 0 ]; then
    echo "### Bug Fixes"
    echo ""
    for line in "${FIX_LINES[@]}"; do echo "$line"; done
    echo ""
fi

if [ ${#PERF_LINES[@]} -gt 0 ]; then
    echo "### Performance"
    echo ""
    for line in "${PERF_LINES[@]}"; do echo "$line"; done
    echo ""
fi

if [ ${#DOCS_LINES[@]} -gt 0 ]; then
    echo "### Documentation"
    echo ""
    for line in "${DOCS_LINES[@]}"; do echo "$line"; done
    echo ""
fi

if [ ${#REFACTOR_LINES[@]} -gt 0 ]; then
    echo "### Refactoring"
    echo ""
    for line in "${REFACTOR_LINES[@]}"; do echo "$line"; done
    echo ""
fi

echo "────────────────────────────────────────────────────────────"
echo "  NOTE: This is a local preview only. No files were changed."
echo "  Release Please may produce slightly different output when"
echo "  it runs against the GitHub repository."
echo "────────────────────────────────────────────────────────────"
