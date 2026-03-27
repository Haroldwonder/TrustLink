# Release Process

This document describes how TrustLink automates versioning, changelog generation, and artifact publishing.

## Overview

The release process is fully automated using:

- **Release Please** — Creates release PRs based on conventional commits
- **Semantic Versioning** — Automatic version bumping (major.minor.patch)
- **Conventional Commits** — Standardized commit messages for changelog generation
- **GitHub Actions** — Builds WASM artifacts and publishes releases

## How It Works

### 1. Commit to Main

When you merge commits to `main` with conventional commit messages:

```bash
git commit -m "feat(storage): add dual indexing for subject and issuer"
git commit -m "fix(validation): reject attestations with valid_from in past"
```

### 2. Release Please Creates a PR

Release Please automatically:

1. Analyzes all commits since the last release
2. Determines the next version based on commit types:
   - `feat` → minor version bump (0.1.0 → 0.2.0)
   - `fix` → patch version bump (0.1.0 → 0.1.1)
   - `docs`, `test`, `chore` → no version bump
3. Creates a Release PR that updates:
   - `Cargo.toml` with the new version
   - `CHANGELOG.md` with formatted commit messages

### 3. Review and Merge Release PR

The Release PR is like any other PR:

- Review the version bump and changelog
- Make any final adjustments if needed
- Merge to `main`

### 4. GitHub Release is Created

When the Release PR is merged:

1. A git tag is created (e.g., `v0.2.0`)
2. A GitHub Release is published
3. The `publish-release` workflow is triggered

### 5. WASM Artifacts are Built and Attached

The `publish-release` workflow:

1. Checks out the release tag
2. Builds the WASM contract
3. Optimizes it with `soroban contract optimize`
4. Attaches both binaries to the GitHub Release:
   - `trustlink.wasm` (unoptimized)
   - `trustlink.optimized.wasm` (optimized for production)

## Commit Message Format

See [CONTRIBUTING.md — Commit Message Conventions](CONTRIBUTING.md#commit-message-conventions) for detailed guidelines.

**Quick reference:**

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat` — new feature (minor bump)
- `fix` — bug fix (patch bump)
- `docs` — documentation
- `test` — tests
- `refactor` — code refactoring
- `perf` — performance improvement (patch bump)
- `chore` — build, CI, dependencies

**Example:**

```
feat(storage): add dual indexing for subject and issuer lookups

The previous single index on subject made issuer-based queries O(n).
This adds a parallel index on issuer to enable fast lookups in both
directions. Queries now complete in O(log n) time.

Closes #42
```

## Version Bumping Rules

| Commits | Version Change | Example |
|---------|---|---|
| `feat` only | Minor | 0.1.0 → 0.2.0 |
| `fix` only | Patch | 0.1.0 → 0.1.1 |
| `feat` + `fix` | Minor | 0.1.0 → 0.2.0 |
| `docs`, `test`, `chore` only | No release | — |
| `BREAKING CHANGE` footer | Major | 0.1.0 → 1.0.0 |

## Workflows

### `release-please.yml`

**Trigger:** Push to `main`

**Actions:**
1. Analyzes commits since last release
2. Creates or updates a Release PR
3. Outputs `release_created` and `tag_name` for downstream workflows

**Configuration:** `release-please-config.json`

### `publish-release.yml`

**Trigger:** GitHub Release published

**Actions:**
1. Checks out the release tag
2. Builds WASM contract
3. Optimizes WASM with `soroban contract optimize`
4. Uploads both binaries to the release
5. Creates a summary in the GitHub Actions log

**Artifacts:**
- `trustlink.wasm` — Unoptimized WASM binary
- `trustlink.optimized.wasm` — Optimized for production deployment

### `validate-commits.yml`

**Trigger:** Pull request opened or updated

**Actions:**
1. Validates PR title follows conventional commits format
2. Ensures commit messages are properly formatted
3. Blocks merge if validation fails

## Manual Release (if needed)

If you need to manually trigger a release:

1. **Create a release PR manually:**
   ```bash
   git checkout -b release/v0.2.0
   # Update Cargo.toml version
   # Update CHANGELOG.md
   git commit -m "chore(release): v0.2.0"
   git push origin release/v0.2.0
   # Open PR and merge
   ```

2. **Create a git tag:**
   ```bash
   git tag -a v0.2.0 -m "Release v0.2.0"
   git push origin v0.2.0
   ```

3. **Create a GitHub Release:**
   - Go to https://github.com/TrustLink/TrustLink/releases
   - Click "Draft a new release"
   - Select the tag `v0.2.0`
   - Add release notes
   - Attach WASM artifacts
   - Publish

## Accessing Artifacts

### From GitHub Releases

1. Go to https://github.com/TrustLink/TrustLink/releases
2. Find the release (e.g., `v0.2.0`)
3. Download:
   - `trustlink.wasm` — Unoptimized
   - `trustlink.optimized.wasm` — Optimized (recommended for production)

### From GitHub Actions

1. Go to https://github.com/TrustLink/TrustLink/actions
2. Find the `publish-release` workflow run
3. Download artifacts from the "Artifacts" section

### From Command Line

```bash
# Download latest release
gh release download --repo TrustLink/TrustLink --pattern "*.wasm"

# Download specific release
gh release download v0.2.0 --repo TrustLink/TrustLink --pattern "*.wasm"
```

## Deployment

See [DEPLOYMENT.md](DEPLOYMENT.md) for instructions on deploying the WASM contract to testnet or mainnet.

**Quick start:**

```bash
# Use the optimized WASM for production
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/trustlink.optimized.wasm \
  --source <account> \
  --network testnet
```

## Troubleshooting

### Release PR not created

**Cause:** No commits with conventional commit format since last release.

**Solution:** Ensure commits follow the format: `feat: ...`, `fix: ...`, etc.

### Version not bumping correctly

**Cause:** Commit types don't match the expected format.

**Solution:** Check [CONTRIBUTING.md — Commit Message Conventions](CONTRIBUTING.md#commit-message-conventions).

### WASM artifacts not attached to release

**Cause:** `publish-release` workflow failed.

**Solution:**
1. Check the workflow run at https://github.com/TrustLink/TrustLink/actions
2. Review logs for build errors
3. Manually build and attach artifacts if needed

### Soroban CLI not found

**Cause:** `soroban-cli` installation failed in the workflow.

**Solution:** Check the `publish-release` workflow logs. The installation step should show any errors.

## Configuration Files

### `release-please-config.json`

Configures Release Please behavior:
- `release-type: rust` — Use Rust-specific versioning
- `changelog-path: CHANGELOG.md` — Where to write the changelog
- `version-file: Cargo.toml` — Which file to update with the version

### `.releaserc.json`

Alternative configuration for semantic-release (if used instead of Release Please).

### `.github/workflows/release-please.yml`

GitHub Actions workflow that runs Release Please.

### `.github/workflows/publish-release.yml`

GitHub Actions workflow that builds and publishes WASM artifacts.

### `.github/workflows/validate-commits.yml`

GitHub Actions workflow that validates commit message format on PRs.

## References

- [Conventional Commits](https://www.conventionalcommits.org/)
- [Semantic Versioning](https://semver.org/)
- [Release Please Documentation](https://github.com/googleapis/release-please)
- [Soroban Contract Optimization](https://soroban.stellar.org/docs/learn/storing-data)
