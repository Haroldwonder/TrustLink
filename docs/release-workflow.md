# Release Workflow Guide

This guide explains the automated release workflow for TrustLink and how to work with it effectively.

## Quick Start

### For Contributors

1. **Write code and commit with conventional format:**
   ```bash
   git commit -m "feat(storage): add dual indexing"
   ```

2. **Push to a feature branch and open a PR:**
   ```bash
   git push origin feat/your-feature
   ```

3. **PR validation:**
   - CI runs tests and checks code quality
   - Commit message validation ensures conventional format
   - At least one approval required

4. **Merge to main:**
   - Use "Squash and merge" or "Create a merge commit"
   - Do NOT use "Rebase and merge" (loses commit history)

5. **Release Please handles the rest:**
   - Automatically creates a Release PR
   - Updates version and changelog
   - You don't need to do anything else

### For Maintainers

1. **Monitor Release PRs:**
   - Release Please creates a PR after commits are merged to main
   - Review the version bump and changelog
   - Merge the Release PR

2. **GitHub Release is created automatically:**
   - WASM artifacts are built and attached
   - Release notes are generated from commits
   - No manual action needed

## Commit Message Format

### Structure

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Type (Required)

| Type | Meaning | Version Impact |
|------|---------|---|
| `feat` | New feature | Minor bump (0.1.0 → 0.2.0) |
| `fix` | Bug fix | Patch bump (0.1.0 → 0.1.1) |
| `docs` | Documentation | No version bump |
| `test` | Tests | No version bump |
| `refactor` | Code refactoring | No version bump |
| `perf` | Performance improvement | Patch bump |
| `chore` | Build, CI, dependencies | No version bump |
| `ci` | CI/CD changes | No version bump |
| `build` | Build system changes | No version bump |

### Scope (Optional)

Narrow the change to a specific area:

- `storage` — Storage layer
- `validation` — Authorization/validation
- `events` — Event emission
- `indexer` — Off-chain indexer
- `sdk` — TypeScript SDK
- `ci` — CI/CD workflows
- `docs` — Documentation

### Subject (Required)

- 50 characters max
- Start with lowercase
- Use imperative mood ("add" not "adds" or "added")
- No period at the end

### Body (Optional)

Explain *why* the change was made:

```
feat(storage): add dual indexing for subject and issuer lookups

The previous single index on subject made issuer-based queries O(n).
This adds a parallel index on issuer to enable fast lookups in both
directions. Queries now complete in O(log n) time.
```

### Footer (Optional)

Reference issues or breaking changes:

```
Closes #42
Closes #99

BREAKING CHANGE: removed the `get_all_attestations` function
```

## Examples

### Good Commits

```
feat(storage): add dual indexing for subject and issuer lookups
```

```
fix(validation): reject attestations with valid_from in the past

Previously, valid_from was only checked against the current time.
Now we also reject any valid_from that is before the current ledger
timestamp, preventing backdated attestations.

Closes #123
```

```
docs: update deployment guide with testnet contract IDs
```

```
test(events): add test for audit log append-only property
```

```
refactor: extract fee calculation into separate function
```

```
perf(storage): optimize subject index lookup with binary search

Reduces query time from O(n) to O(log n) for large attestation sets.
```

### Bad Commits

```
❌ Updated stuff
❌ Fix bug
❌ feat: Add new feature.
❌ FEAT: ADD FEATURE
❌ feat(storage): added dual indexing
❌ feat(storage): add dual indexing.
```

## Version Bumping Examples

### Scenario 1: Feature Release

**Commits merged:**
- `feat(storage): add dual indexing`
- `feat(validation): add new authorization check`
- `fix(events): emit correct event data`

**Result:** Minor version bump
- 0.1.0 → 0.2.0

**Changelog:**
```markdown
## [0.2.0] - 2024-03-26

### Features
- add dual indexing
- add new authorization check

### Bug Fixes
- emit correct event data
```

### Scenario 2: Patch Release

**Commits merged:**
- `fix(validation): reject invalid timestamps`
- `fix(storage): handle edge case in pagination`

**Result:** Patch version bump
- 0.1.0 → 0.1.1

**Changelog:**
```markdown
## [0.1.1] - 2024-03-26

### Bug Fixes
- reject invalid timestamps
- handle edge case in pagination
```

### Scenario 3: No Release

**Commits merged:**
- `docs: update README`
- `test: add edge case tests`
- `chore: update dependencies`

**Result:** No version bump, no release created

### Scenario 4: Breaking Change

**Commits merged:**
- `feat(storage): redesign storage layout`
- `BREAKING CHANGE: removed get_all_attestations function`

**Result:** Major version bump
- 0.1.0 → 1.0.0

## Workflow Files

### `.github/workflows/release-please.yml`

**Trigger:** Push to main

**What it does:**
1. Analyzes commits since last release
2. Determines next version
3. Creates or updates Release PR
4. Updates `Cargo.toml` and `CHANGELOG.md`

**Configuration:** `release-please-config.json`

### `.github/workflows/publish-release.yml`

**Trigger:** GitHub Release published

**What it does:**
1. Checks out release tag
2. Builds WASM contract
3. Optimizes with `soroban contract optimize`
4. Uploads artifacts to release

**Artifacts:**
- `trustlink.wasm` — Unoptimized
- `trustlink.optimized.wasm` — Optimized for production

### `.github/workflows/validate-commits.yml`

**Trigger:** PR opened or updated

**What it does:**
1. Validates PR title format
2. Checks commit messages
3. Blocks merge if invalid

## Configuration Files

### `release-please-config.json`

Controls Release Please behavior:

```json
{
  "packages": {
    ".": {
      "changelog-path": "CHANGELOG.md",
      "release-type": "rust",
      "prerelease": false
    }
  },
  "changelog-sections": [
    {"type": "feat", "section": "Features", "hidden": false},
    {"type": "fix", "section": "Bug Fixes", "hidden": false},
    ...
  ]
}
```

### `.commitlintrc.json`

Validates commit message format locally and in CI:

```json
{
  "extends": ["@commitlint/config-conventional"],
  "rules": {
    "type-enum": [2, "always", ["feat", "fix", "docs", ...]],
    "subject-case": [2, "never", ["start-case", "pascal-case"]],
    ...
  }
}
```

### `.pre-commit-config.yaml`

Runs validation before commits:

```yaml
repos:
  - repo: https://github.com/commitlint-rs/commitlint
    rev: v0.1.1
    hooks:
      - id: commitlint
        stages: [commit-msg]
```

## Troubleshooting

### Release PR not created

**Problem:** No Release PR appears after merging commits.

**Causes:**
1. Commits don't follow conventional format
2. All commits are `docs`, `test`, or `chore` (no version bump)
3. Release Please workflow is disabled

**Solution:**
1. Check commit messages: `git log --oneline main..origin/main`
2. Ensure at least one `feat` or `fix` commit
3. Check `.github/workflows/release-please.yml` is enabled

### Version bumped incorrectly

**Problem:** Version bumped more or less than expected.

**Cause:** Commit types don't match expectations.

**Solution:**
1. Review commits: `git log --oneline <last-tag>..main`
2. Check commit format against [Commit Message Format](#commit-message-format)
3. Verify `release-please-config.json` changelog-types

### Commit validation fails locally

**Problem:** Pre-commit hook rejects commit message.

**Cause:** Message doesn't match `.commitlintrc.json` rules.

**Solution:**
1. Review error message
2. Fix commit message format
3. Try again: `git commit --amend`

### Commit validation fails in CI

**Problem:** PR blocked by `validate-commits` workflow.

**Cause:** PR title or commits don't follow conventional format.

**Solution:**
1. Update PR title to follow format: `feat: description`
2. Or squash commits into one with proper format
3. Force-push to update PR

### WASM artifacts not attached

**Problem:** GitHub Release created but no WASM files attached.

**Cause:** `publish-release` workflow failed.

**Solution:**
1. Check workflow run: https://github.com/TrustLink/TrustLink/actions
2. Review logs for build errors
3. Manually build and attach if needed:
   ```bash
   cargo build --target wasm32-unknown-unknown --release
   soroban contract optimize \
     --wasm target/wasm32-unknown-unknown/release/trustlink.wasm \
     --wasm-out target/wasm32-unknown-unknown/release/trustlink.optimized.wasm
   gh release upload v0.2.0 \
     target/wasm32-unknown-unknown/release/trustlink.wasm \
     target/wasm32-unknown-unknown/release/trustlink.optimized.wasm
   ```

## Best Practices

### 1. Use Descriptive Commit Messages

✅ Good:
```
feat(storage): add dual indexing for subject and issuer lookups

The previous single index on subject made issuer-based queries O(n).
This adds a parallel index to enable fast lookups in both directions.
```

❌ Bad:
```
feat: update storage
```

### 2. One Logical Change Per Commit

✅ Good:
```
feat(storage): add subject index
feat(storage): add issuer index
```

❌ Bad:
```
feat(storage): add subject and issuer indexes and fix pagination bug
```

### 3. Use Squash and Merge for PRs

When merging a PR with multiple commits, use "Squash and merge" to create a single, clean commit message:

```bash
git merge --squash feature-branch
git commit -m "feat(storage): add dual indexing"
```

This ensures the changelog has clean, meaningful entries.

### 4. Reference Issues in Commits

```
fix(validation): reject invalid timestamps

Closes #42
```

This automatically closes the issue when the commit is merged.

### 5. Use Breaking Change Footer for Major Versions

```
feat(storage): redesign storage layout

BREAKING CHANGE: removed get_all_attestations function
```

This triggers a major version bump (0.1.0 → 1.0.0).

## Manual Release (Emergency Only)

If automated release fails, you can manually create a release:

1. **Create release tag:**
   ```bash
   git tag -a v0.2.0 -m "Release v0.2.0"
   git push origin v0.2.0
   ```

2. **Build artifacts:**
   ```bash
   cargo build --target wasm32-unknown-unknown --release
   soroban contract optimize \
     --wasm target/wasm32-unknown-unknown/release/trustlink.wasm \
     --wasm-out target/wasm32-unknown-unknown/release/trustlink.optimized.wasm
   ```

3. **Create GitHub Release:**
   ```bash
   gh release create v0.2.0 \
     target/wasm32-unknown-unknown/release/trustlink.wasm \
     target/wasm32-unknown-unknown/release/trustlink.optimized.wasm \
     --title "TrustLink v0.2.0" \
     --notes "See CHANGELOG.md for details"
   ```

## References

- [Conventional Commits](https://www.conventionalcommits.org/)
- [Semantic Versioning](https://semver.org/)
- [Release Please](https://github.com/googleapis/release-please)
- [Commitlint](https://commitlint.js.org/)
- [CONTRIBUTING.md](../CONTRIBUTING.md)
- [RELEASE.md](../RELEASE.md)
