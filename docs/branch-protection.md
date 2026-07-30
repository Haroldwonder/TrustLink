# Branch Protection

This document records the intended branch-protection configuration for
`main`, the repository's only long-lived branch. It exists so that CI/CD
and review-process changes can be checked against an agreed-upon policy
instead of tribal knowledge, and so anyone auditing the repository's
GitHub settings has something to diff against.

> **Maintainers:** if GitHub's settings ever drift from this document,
> either update the settings to match or update this document to match —
> don't let them silently diverge. See [Verifying the Configuration](#verifying-the-configuration).

## Protected Branch

- `main`

## Required Status Checks

Pull requests must pass the following checks before merging, based on the
workflows defined in [`.github/workflows/`](../.github/workflows):

| Check | Workflow | Why it's required |
|---|---|---|
| `Build and Test` | [`ci.yml`](../.github/workflows/ci.yml) (`ci` job) | Compiles the contract, runs the full test suite, and verifies snapshot files are up to date |
| `Security Audit` | [`ci.yml`](../.github/workflows/ci.yml) (`audit` job) | Runs `cargo audit --deny warnings`; see [`docs/dependency-security.md`](dependency-security.md) |
| `WASM Size Check` | [`ci.yml`](../.github/workflows/ci.yml) (`wasm-size` job) | Blocks merges that push the optimized WASM binary over the 100 KB threshold |
| `TypeScript Bindings` | [`ci.yml`](../.github/workflows/ci.yml) (`bindings` job) | Fails if `bindings/typescript/` is out of sync with the contract interface |
| `Check Conventional Commits` | [`validate-commits.yml`](../.github/workflows/validate-commits.yml) | Enforces the PR title format described in [CONTRIBUTING.md](../CONTRIBUTING.md#commit-message-conventions), which Release Please depends on |

Branches must be up to date with `main` before merging, so these checks run
against the code that will actually land.

## Review Requirements

- At least **one approving review** is required before merging (see
  [CONTRIBUTING.md § PR Process](../CONTRIBUTING.md#pr-process)).
- New commits pushed to a PR dismiss stale approvals — reviewers should
  re-review after force-pushes.
- Conversation resolution is required: all review comments must be marked
  resolved before merging.

## Merge Rules

- **Allowed merge strategies**: "Squash and merge" or "Create a merge
  commit". "Rebase and merge" is disallowed — it would strip the metadata
  Release Please uses to generate changelogs.
- **Force pushes to `main`** are disallowed.
- **Deletion of `main`** is disallowed.
- Administrators are expected to follow the same rules as everyone else;
  the "include administrators" option should be enabled.

## Verifying the Configuration

To confirm GitHub's actual settings match this document, a maintainer with
repository admin access should check **Settings → Branches → Branch
protection rules → `main`** and confirm:

1. The required status checks listed above are all enabled and set to
   "must be up to date before merging".
2. Required approvals is set to at least `1`.
3. "Do not allow bypassing the above settings" (include administrators) is
   enabled.
4. Force pushes and branch deletion are both disallowed.

If any setting differs, either the GitHub configuration or this document
should be updated so the two stay in sync — see the note at the top of
this file.
