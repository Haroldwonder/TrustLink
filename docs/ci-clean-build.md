# CI Clean-Build Job

## Purpose

The regular `Build and Test` job in `.github/workflows/ci.yml` restores a
`target/` cargo cache across runs. That speeds PRs up, but a corrupted or
stale incremental cache can theoretically mask a breakage that only appears
on a fresh tree (the class of failure found in the StorageKey audit).

The **Clean Build** job exists to close that gap.

## What it runs

Job name: `Clean Build` (`clean-build` in `ci.yml`)

On a fresh `actions/checkout` with **no `target/` cache restore**:

1. `cargo check --lib`
2. `cargo test`
3. `cargo build --target wasm32-unknown-unknown --release`

Registry/git caches may still be used for download speed; only build artifacts
(`target/`) are excluded so the compile is always from a clean state.

## When it runs

| Trigger | Why |
|---------|-----|
| `push` to `main` | Every merge to main gets a full clean-checkout build |
| `schedule` (nightly `0 4 * * *` UTC) | Periodic verification even without merges |
| `workflow_dispatch` | Manual re-run for verification / deliberate breakage tests |

## Verifying it catches StorageKey breakage

To confirm the job would catch the audit-class failure:

1. Open a test PR that references `StorageKey::DoesNotExist` somewhere under `src/`
   **without** adding that variant to the enum (or temporarily remove a used variant).
2. The `StorageKey variants` CI job fails immediately via
   `scripts/check-storage-keys.sh`.
3. Independently, `cargo check` / the Clean Build job also fails with unresolved
   variant errors — proving a clean checkout does not hide the break behind cache.

Do not merge such a test PR; close it after the red X is observed.

## Rust toolchain pinning

`rust-toolchain.toml` pins `channel` to an exact version (e.g. `1.xx.0`)
rather than `stable`, so the compiler does not change underneath us when a new
stable release ships. This keeps WASM builds reproducible (identical hashes
across machines and CI runs) and prevents new warnings or errors from
appearing without any code change.

### Bumping the toolchain

Dependabot is configured for the `rust-toolchain` ecosystem and opens a PR
whenever a newer Rust release is available. Review the PR, let CI (including
the Clean Build job) go green, and merge it.

To bump manually:

1. Update `channel` in `rust-toolchain.toml` to the new exact version.
2. Run the Clean Build steps locally (or trigger the `Clean Build` workflow via
   `workflow_dispatch`) to confirm `cargo check --lib`, `cargo test`, and the
   `wasm32-unknown-unknown` release build all succeed.
3. Verify WASM hash stability: build the release WASM before and after the
   bump and compare hashes. Any difference should be explainable by the
   compiler change; if not, investigate before merging.
=======
