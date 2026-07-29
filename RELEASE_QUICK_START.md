# Release Automation Quick Start

Get up and running with TrustLink's automated release process in 5 minutes.

## For Contributors

### 1. Install Pre-commit Hooks (One Time)

```bash
pip install pre-commit
pre-commit install
```

This validates your commits before they're created.

### 2. Write Code and Commit

Use conventional commit format:

```bash
git commit -m "feat(storage): add dual indexing"
git commit -m "fix(validation): reject invalid timestamps"
git commit -m "docs: update README"
```

**Commit types:**
- `feat` — new feature (triggers minor version bump)
- `fix` — bug fix (triggers patch version bump)
- `docs`, `test`, `chore` — no version bump

### 3. Push and Open PR

```bash
git push origin your-branch
# Open PR on GitHub
```

### 4. Merge to Main

Use "Squash and merge" or "Create a merge commit" (not "Rebase and merge").

**That's it!** Release Please handles the rest automatically.

---

## For Maintainers

### 1. Monitor Release PRs

After commits are merged to main, Release Please creates a Release PR within minutes.

### 2. Review and Merge

- Check version bump is correct
- Review changelog entries
- Merge the Release PR

### 3. GitHub Release is Created Automatically

- Git tag is created
- WASM artifacts are built and attached
- Release notes are published

**No manual action needed!**

---

## Commit Message Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Quick Examples

✅ Good:
```
feat(storage): add dual indexing for subject and issuer lookups
```

✅ Good:
```
fix(validation): reject attestations with valid_from in the past

Previously, valid_from was only checked against the current time.
Now we also reject any valid_from that is before the current ledger
timestamp, preventing backdated attestations.

Closes #123
```

❌ Bad:
```
Updated stuff
feat: Add new feature.
feat(storage): added dual indexing
```

---

## Version Bumping

| Commits | Version Change |
|---------|---|
| `feat` only | Minor (0.1.0 → 0.2.0) |
| `fix` only | Patch (0.1.0 → 0.1.1) |
| `feat` + `fix` | Minor (0.1.0 → 0.2.0) |
| `docs`, `test`, `chore` only | No release |
| `BREAKING CHANGE` footer | Major (0.1.0 → 1.0.0) |

---

## Commit Types

| Type | Purpose |
|------|---------|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation |
| `test` | Tests |
| `refactor` | Code refactoring |
| `perf` | Performance improvement |
| `chore` | Build, CI, dependencies |

---

## Scopes (Optional)

Narrow the change to a specific area:

- `storage` — Storage layer
- `validation` — Authorization/validation
- `events` — Event emission
- `indexer` — Off-chain indexer
- `sdk` — TypeScript SDK
- `ci` — CI/CD workflows
- `docs` — Documentation

---

## Troubleshooting

### Pre-commit hook fails

**Problem:** Commit is rejected by pre-commit.

**Solution:**
1. Fix the reported issue
2. Try committing again

### Release PR not created

**Problem:** No Release PR appears after merging commits.

**Solution:**
1. Check commits follow conventional format: `git log --oneline main..origin/main`
2. Ensure at least one `feat` or `fix` commit
3. Wait a few minutes (Release Please runs on schedule)

### WASM artifacts not attached

**Problem:** GitHub Release created but no WASM files.

**Solution:**
1. Check workflow logs: https://github.com/TrustLink/TrustLink/actions
2. Review `publish-release` workflow for errors
3. Manually build and attach if needed

---

## Documentation

- **[CONTRIBUTING.md](CONTRIBUTING.md)** — Full contribution guide with commit conventions
- **[RELEASE.md](RELEASE.md)** — Detailed release process documentation
- **[docs/release-workflow.md](docs/release-workflow.md)** — Comprehensive workflow guide
- **[RELEASE_AUTOMATION_IMPLEMENTATION.md](RELEASE_AUTOMATION_IMPLEMENTATION.md)** — Implementation details

---

## Key Files

| File | Purpose |
|------|---------|
| `.github/workflows/release-please.yml` | Creates Release PRs |
| `.github/workflows/publish-release.yml` | Builds and publishes WASM |
| `.github/workflows/validate-commits.yml` | Validates commit format |
| `release-please-config.json` | Release Please configuration |
| `.commitlintrc.json` | Commit validation rules |
| `.pre-commit-config.yaml` | Local pre-commit hooks |

---

## Next Steps

1. **Install pre-commit hooks:**
   ```bash
   pre-commit install
   ```

2. **Read [CONTRIBUTING.md](CONTRIBUTING.md) for full guidelines**

3. **Start committing with conventional format**

4. **Watch Release PRs appear automatically!**

---

**Questions?** See [RELEASE.md](RELEASE.md) or [docs/release-workflow.md](docs/release-workflow.md) for detailed documentation.
