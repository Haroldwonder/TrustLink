# ADR-013: Postmortem — Main-Branch Build-Breakage Incident

- **Status**: Accepted
- **Date**: 2026-09-01

## Context

In late August 2026, `main` was found to be failing `cargo check --lib` with
172 compilation errors. The breakage was discovered during a routine audit of
the repository's CI status and confirmed by the external security researcher
who filed issue #1238 (they were unable to build the contract locally before
reporting their findings).

Root-cause analysis identified the following contributing factors:

1. **Incomplete merge resolution.** The `main` branch contained conflicting or
   redundant definitions — duplicate `StorageKey` variants, duplicate type
   definitions, and duplicate method implementations — consistent with a merge
   conflict that was committed without being fully resolved. Feature code that
   referenced types and error variants introduced on one side of the merge was
   retained, but the corresponding definitions from the other side were not,
   leaving dangling references that `rustc` could not resolve.

2. **No compile gate on `main` pushes at the time.** The regular `Build and
   Test` CI job ran on pull requests and cached the `target/` directory. A
   corrupted or warmed cache from a prior green build can mask new breakage
   that only surfaces on a clean tree. There was no job that ran
   `cargo check --lib` from a clean checkout on every push to `main`.

3. **Missing enum-completeness guards.** There were no automated checks to
   verify that every `StorageKey::Variant` referenced in `src/` was actually
   defined in the enum, or that every `Error::Variant` reference resolved to a
   definition in `src/errors.rs`. These gaps meant broken references could
   silently accumulate across merges.

### Impact

- `cargo check --lib`, `cargo test`, and the WASM release build all failed on
  a clean checkout of `main` for an indeterminate period.
- Any contributor cloning the repository fresh received 172 compiler errors
  with no straightforward path to a working build.
- The external security researcher who filed issue #1238 was unable to verify
  their findings locally using the published source, reducing confidence in the
  responsible-disclosure process.
- No on-chain contract was affected; the project had not yet been deployed to
  mainnet, so there was zero user-facing impact.

### Timeline (approximate)

| Date | Event |
|------|-------|
| Before 2026-08-26 | Incomplete merge committed to `main`; 172 compiler errors introduced |
| 2026-08-26 | Security researcher files issue #1238, incidentally noting build failure |
| 2026-09-01 | Internal audit confirms `cargo check --lib` fails on clean checkout |
| 2026-09-01 | This ADR and accompanying safeguards drafted (issues #1034, #1032, #1033) |

## Decision

To document the incident and record the process/tooling changes made in
response, so future contributors understand both what happened and why the new
safeguards exist.

### Remediation applied

The following changes were made or formalised as a direct result of this
incident:

**1. Clean-Build CI job** (`clean-build` in `.github/workflows/ci.yml`)

A dedicated job was added that runs on every push to `main`, on a nightly
schedule, and on manual dispatch. It explicitly deletes `target/` before
compiling, so no cached artifacts can hide a fresh-checkout failure. It runs
`cargo check --lib`, `cargo test`, `cargo clippy`, and the WASM release build
in sequence. See [`docs/ci-clean-build.md`](../ci-clean-build.md) for details.

**2. StorageKey completeness check** (`scripts/check-storage-keys.sh`)

A Bash script that statically parses `pub enum StorageKey` in `src/storage.rs`
and cross-references every `StorageKey::Variant` usage in `src/`. It fails
with a targeted error listing any variant referenced but not defined. Runs as
the `StorageKey variants` CI job on every pull request.

**3. Error-variant completeness check** (`scripts/check_error_variants.py`)

A Python script that extracts defined variants from `src/errors.rs` and
cross-references every `Error::Variant` reference in `src/`. Fails with a
targeted error on any undefined reference. Runs as the `Enum Completeness` CI
job on every pull request.

**4. Branch-protection policy documented** (`docs/branch-protection.md`)

The intended branch-protection rules for `main` are now documented in source
control, making it auditable whether GitHub's actual settings match the agreed
policy. Required status checks include compilation, test, and the completeness
checks above.

**5. Build breakage fix**

The duplicate definitions and dangling references introduced by the incomplete
merge were identified and resolved, returning `main` to a green state on a
clean checkout.

## Consequences

**Positive**
- Every merge to `main` now triggers a compile + test run from a clean tree,
  making it impossible for cache-masked breakage to accumulate silently.
- The `StorageKey` and `Error` completeness scripts provide targeted, early
  feedback — the CI failure message names the missing variant and the files
  that reference it, rather than dumping a wall of `rustc` errors.
- The incident and its response are documented here, giving future maintainers
  context for why these guards exist and what they protect against.
- The branch-protection document creates an auditable policy that keeps GitHub
  settings from drifting silently.

**Negative**
- The `clean-build` job adds ~10–15 minutes to every main-branch push and to
  the nightly run. This is an acceptable cost for the safety it provides.
- Static text-parsing scripts (`check-storage-keys.sh`,
  `check_error_variants.py`) are inherently fragile to large refactors (e.g.,
  renaming the enum or splitting `errors.rs`). They should be updated whenever
  the files they parse are significantly restructured.

**Neutral**
- The `clean-build` job does not replace the cached `Build and Test` job; both
  run in parallel. PRs use the cached job for speed; main-branch merges also
  get the clean-checkout verification.
- The enum-completeness scripts catch the class of error that triggered this
  incident; they do not provide general compile-time safety. `cargo check` in
  the `clean-build` job remains the authoritative compilation gate.

## Related

- Issue [#1034](https://github.com/Haroldwonder/TrustLink/issues/1034) — request to document this incident
- Issue [#1238](https://github.com/Haroldwonder/TrustLink/issues/1238) — security researcher unable to build from `main`
- [`docs/ci-clean-build.md`](../ci-clean-build.md) — Clean Build job documentation
- [`docs/branch-protection.md`](../branch-protection.md) — branch protection policy
- [`scripts/check-storage-keys.sh`](../../scripts/check-storage-keys.sh) — StorageKey completeness check
- [`scripts/check_error_variants.py`](../../scripts/check_error_variants.py) — Error variant completeness check
