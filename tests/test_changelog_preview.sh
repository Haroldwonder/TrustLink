#!/usr/bin/env bash
# tests/test_changelog_preview.sh — Tests for scripts/changelog-preview.sh
#
# Runs in a temporary git repository to isolate from the real repo state.
# Each test creates a minimal git history and verifies the script output.
#
# Usage: bash tests/test_changelog_preview.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PREVIEW_SCRIPT="${SCRIPT_DIR}/../scripts/changelog-preview.sh"

PASS=0
FAIL=0

# ── Test harness ──────────────────────────────────────────────────────────────

run_test() {
    local name="$1"
    local fn="$2"
    if "$fn"; then
        echo "  PASS  $name"
        PASS=$((PASS + 1))
    else
        echo "  FAIL  $name"
        FAIL=$((FAIL + 1))
    fi
}

# Create a temp git repo with a Cargo.toml at the given version.
setup_repo() {
    local version="${1:-0.1.0}"
    TMPDIR_REPO=$(mktemp -d)
    cd "$TMPDIR_REPO"
    git init -q
    git config user.email "test@test.com"
    git config user.name "Test"
    printf '[package]\nname = "test"\nversion = "%s"\nedition = "2021"\n' "$version" > Cargo.toml
    git add Cargo.toml
    git commit -q -m "chore: initial commit"
}

cleanup_repo() {
    cd /
    rm -rf "$TMPDIR_REPO"
}

add_commit() {
    local msg="$1"
    echo "$msg" >> changes.txt
    git add changes.txt
    git commit -q -m "$msg"
}

add_tag() {
    git tag "$1"
}

# ── Tests ─────────────────────────────────────────────────────────────────────

test_no_releasable_commits() {
    setup_repo "0.1.0"
    add_commit "chore: update ci"
    add_commit "test: add unit tests"
    local out
    out=$(bash "$PREVIEW_SCRIPT")
    cleanup_repo
    echo "$out" | grep -q "No releasable commits"
}

test_feat_produces_minor_bump() {
    setup_repo "0.1.0"
    add_commit "feat(storage): add dual indexing"
    local out
    out=$(bash "$PREVIEW_SCRIPT")
    cleanup_repo
    echo "$out" | grep -q "0.2.0"
    echo "$out" | grep -q "minor"
}

test_fix_produces_patch_bump() {
    setup_repo "0.1.0"
    add_commit "fix(validation): reject past expiration"
    local out
    out=$(bash "$PREVIEW_SCRIPT")
    cleanup_repo
    echo "$out" | grep -q "0.1.1"
    echo "$out" | grep -q "patch"
}

test_feat_and_fix_produces_minor_bump() {
    setup_repo "0.1.0"
    add_commit "fix(auth): fix auth check"
    add_commit "feat(api): add new endpoint"
    local out
    out=$(bash "$PREVIEW_SCRIPT")
    cleanup_repo
    echo "$out" | grep -q "0.2.0"
    echo "$out" | grep -q "minor"
}

test_feat_appears_in_features_section() {
    setup_repo "0.1.0"
    add_commit "feat(storage): add dual indexing"
    local out
    out=$(bash "$PREVIEW_SCRIPT")
    cleanup_repo
    echo "$out" | grep -q "### Features"
    echo "$out" | grep -q "add dual indexing"
}

test_fix_appears_in_bug_fixes_section() {
    setup_repo "0.1.0"
    add_commit "fix(validation): reject past expiration"
    local out
    out=$(bash "$PREVIEW_SCRIPT")
    cleanup_repo
    echo "$out" | grep -q "### Bug Fixes"
    echo "$out" | grep -q "reject past expiration"
}

test_docs_appears_in_documentation_section() {
    setup_repo "0.1.0"
    add_commit "docs(readme): update usage examples"
    local out
    out=$(bash "$PREVIEW_SCRIPT")
    cleanup_repo
    # docs alone don't trigger a release bump, but should appear in output
    # (no version bump, so "No releasable commits" is expected)
    echo "$out" | grep -q "No releasable commits"
}

test_chore_is_hidden() {
    setup_repo "0.1.0"
    add_commit "chore(deps): bump soroban-sdk"
    local out
    out=$(bash "$PREVIEW_SCRIPT")
    cleanup_repo
    echo "$out" | grep -q "No releasable commits"
}

test_since_last_tag_only() {
    setup_repo "0.1.0"
    add_commit "feat(old): old feature before tag"
    add_tag "v0.1.0"
    add_commit "fix(new): new fix after tag"
    local out
    out=$(bash "$PREVIEW_SCRIPT")
    cleanup_repo
    # Should show 0.1.1 (patch), not 0.2.0 (minor from old feat)
    echo "$out" | grep -q "0.1.1"
    # Old feat should NOT appear
    ! echo "$out" | grep -q "old feature before tag"
}

test_no_files_modified() {
    setup_repo "0.1.0"
    add_commit "feat(api): add endpoint"
    local before
    before=$(find . -newer Cargo.toml -not -path './.git/*' 2>/dev/null | sort)
    bash "$PREVIEW_SCRIPT" > /dev/null
    local after
    after=$(find . -newer Cargo.toml -not -path './.git/*' 2>/dev/null | sort)
    cleanup_repo
    [ "$before" = "$after" ]
}

test_exit_code_zero_on_success() {
    setup_repo "0.1.0"
    add_commit "feat(api): add endpoint"
    bash "$PREVIEW_SCRIPT" > /dev/null
    local code=$?
    cleanup_repo
    [ "$code" -eq 0 ]
}

test_exit_code_zero_no_releasable() {
    setup_repo "0.1.0"
    add_commit "chore: update ci"
    bash "$PREVIEW_SCRIPT" > /dev/null
    local code=$?
    cleanup_repo
    [ "$code" -eq 0 ]
}

test_current_version_shown() {
    setup_repo "1.2.3"
    add_commit "fix(auth): fix token check"
    local out
    out=$(bash "$PREVIEW_SCRIPT")
    cleanup_repo
    echo "$out" | grep -q "1.2.3"
    echo "$out" | grep -q "1.2.4"
}

test_scope_formatted_in_output() {
    setup_repo "0.1.0"
    add_commit "feat(storage): add indexing"
    local out
    out=$(bash "$PREVIEW_SCRIPT")
    cleanup_repo
    echo "$out" | grep -q "storage"
}

test_perf_produces_patch_bump() {
    setup_repo "0.1.0"
    add_commit "perf(query): optimize attestation lookup"
    local out
    out=$(bash "$PREVIEW_SCRIPT")
    cleanup_repo
    echo "$out" | grep -q "0.1.1"
    echo "$out" | grep -q "patch"
}

# ── Run all tests ─────────────────────────────────────────────────────────────

echo ""
echo "Running changelog-preview tests..."
echo ""

run_test "no releasable commits → no release message" test_no_releasable_commits
run_test "feat commit → minor bump (0.1.0 → 0.2.0)" test_feat_produces_minor_bump
run_test "fix commit → patch bump (0.1.0 → 0.1.1)" test_fix_produces_patch_bump
run_test "feat + fix → minor bump wins" test_feat_and_fix_produces_minor_bump
run_test "feat appears in Features section" test_feat_appears_in_features_section
run_test "fix appears in Bug Fixes section" test_fix_appears_in_bug_fixes_section
run_test "docs alone → no release" test_docs_appears_in_documentation_section
run_test "chore is hidden (no release)" test_chore_is_hidden
run_test "only commits since last tag are included" test_since_last_tag_only
run_test "no files are modified by the script" test_no_files_modified
run_test "exit code 0 on success" test_exit_code_zero_on_success
run_test "exit code 0 when no releasable commits" test_exit_code_zero_no_releasable
run_test "current and next version shown correctly" test_current_version_shown
run_test "scope is formatted in output" test_scope_formatted_in_output
run_test "perf commit → patch bump" test_perf_produces_patch_bump

echo ""
echo "Results: ${PASS} passed, ${FAIL} failed"
echo ""

[ "$FAIL" -eq 0 ]

