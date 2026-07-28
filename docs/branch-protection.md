# Branch Protection Rules

This document outlines the required status checks and branch protection rules for the TrustLink repository, ensuring a green `main` branch and preventing integration failures.

## Overview

Branch protection on `main` requires all PRs to pass automated checks before merge. This prevents broken builds from reaching production and ensures code quality standards are maintained.

## Required Status Checks

The following CI workflows are **required** to pass before merge:

### 1. **Check** (`check`)
- **Job**: Runs `cargo check --workspace --all-targets`
- **Purpose**: Fast, early compilation check to catch syntax errors and type mismatches
- **Failure**: Blocks merge immediately (fails before expensive tests)
- **Added**: Issue #927

### 2. **Lint** (`lint`)
- **Job**: Runs `cargo clippy --all-targets -- -D warnings`
- **Purpose**: Static analysis and code quality checks
- **Failure**: Blocks merge (warnings treated as errors)
- **Depends on**: `check` job (must pass first)

### 3. **Build and Test** (`ci`)
- **Job**: Runs `cargo test` and full test suite
- **Purpose**: Verifies correctness of implementation
- **Coverage**: Requires 80% line coverage (`cargo llvm-cov --fail-under-lines 80`)
- **Snapshots**: Verifies test snapshots are up-to-date
- **Failure**: Blocks merge if tests fail or coverage drops
- **Depends on**: `check` job (must pass first)

### 4. **Security Audit** (`audit`)
- **Job**: Runs `cargo audit --deny warnings`
- **Purpose**: Scans dependencies for known security vulnerabilities
- **Failure**: Blocks merge if vulnerabilities found
- **Runs independently**: Does not depend on other jobs (for early detection)

### 5. **TypeScript Bindings** (`bindings`)
- **Job**: Runs `make bindings` and verifies bindings are up-to-date
- **Purpose**: Ensures generated TypeScript bindings match Rust contract
- **Failure**: Blocks merge if bindings are stale
- **Depends on**: `ci` job (build artifacts required)

### 6. **WASM Size Check** (`wasm-size`)
- **Job**: Builds and optimizes WASM with `wasm-opt -Oz`
- **Purpose**: Enforces 100 KB WASM binary limit
- **Failure**: Blocks merge if optimized WASM exceeds limit
- **Depends on**: `ci` job (requires successful build)

## Configuration

### GitHub Branch Protection Rules

The following repository settings should be configured:

```
Settings > Branches > Branch protection rules > main

[x] Require a pull request before merging
    [x] Require approvals (minimum 1)
    [x] Require status checks to pass before merging
    [x] Require branches to be up to date before merging

    Required status checks (must pass):
    - check
    - lint
    - audit
    - ci
    - bindings
    - wasm-size

[x] Require code reviews before merging
[x] Require conversation resolution before merging
[x] Require signed commits
[x] Dismiss stale pull request approvals when new commits are pushed
[x] Require administrators to follow the same rules
[x] Restrict who can push to matching branches
```

### Workflow Execution

- All workflows are triggered on:
  - **`push` to `main`**: Runs post-merge verification
  - **Pull requests**: Blocks merge until all checks pass
  
- Jobs run concurrently where possible:
  - `audit` and `check` run independently in parallel
  - `lint` depends on `check`
  - `ci` depends on `check`
  - `bindings` depends on `ci`
  - `wasm-size` depends on `ci`
  - `pr-size-comment` depends on `wasm-size`

## Incident: Issue #926

On 2026-07-28, a merge introduced 172 compilation errors that reached `main` despite CI. Analysis revealed:

**Root Causes:**
1. No early `cargo check` job to fail fast before expensive builds
2. Branch protection may not have been configured as required
3. Possible CI bypass or check was not actually enforced

**Resolution:**
1. Added `check` job (Issue #927) for fast compilation validation
2. Documented this branch protection policy (Issue #926)
3. Verified all status checks are actually required on `main`
4. Added explicit `needs:` dependencies to prevent job skip

## CI Workflow Execution Order

```
┌─────────────────────────────────────────────────────────────┐
│ GitHub Actions triggered on push/PR                         │
└─────────────────────────────────────────────────────────────┘
                          ↓
         ┌────────────────┬────────────────┐
         ↓                ↓                ↓
     [check]         [audit]          [wasm-size]
   (cargo check) (cargo audit)      (depends on: ci)
         ↓
     [lint]
   (cargo clippy)
         ↓
     [ci]
   (cargo test + coverage)
         ↓
     [bindings]
   (make bindings)
         ↓
   [pr-size-comment]
   (posts WASM size to PR)
```

## Bypassing Branch Protection

Administrators can bypass branch protection, but:

1. **This is an explicit action** requiring confirmation
2. **A security audit trail is created** in repository logs
3. **This should be rare** — only in critical situations (e.g., security hotfix)
4. **Always create a post-mortem** documenting why the bypass was necessary

## Contributing

When submitting PRs:

1. Ensure your local `cargo check --workspace` passes before pushing
2. Wait for all CI checks to pass (typically 5-10 minutes)
3. If `check` fails, fix compilation errors immediately
4. If `lint` fails, run `cargo clippy` locally and fix warnings
5. If `ci` fails, run `cargo test` locally and fix failing tests
6. If coverage drops, add tests to maintain ≥80% line coverage

## Troubleshooting

### PR is blocked by "check" job
- **Solution**: Fix compilation errors with `cargo check --workspace`
- **Common causes**: Type mismatches, missing imports, syntax errors

### PR is blocked by "lint" job
- **Solution**: Run `cargo clippy --fix` and commit the fixes
- **Common causes**: Unused variables, lint warnings

### PR is blocked by "ci" job
- **Solution**: Run `cargo test` locally and fix failing tests
- **Common causes**: Logic errors, test assumptions, environment setup

### WASM size exceeds limit
- **Solution**: Optimize code or split functionality into separate contracts
- **Common causes**: Feature bloat, unused dependencies, unoptimized code

## Related Documentation

- [CI Workflow](../.github/workflows/ci.yml) — Source of truth for CI configuration
- [Mainnet Runbook](./mainnet-runbook.md) — Production deployment guide
- [Contributing Guide](../CONTRIBUTING.md) — How to contribute to TrustLink
