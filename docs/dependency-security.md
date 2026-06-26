# Dependency Security Policy

## Overview

TrustLink uses two complementary tools to keep its dependency tree secure and compliant:

- **cargo-audit** — scans `Cargo.lock` against the [RustSec Advisory Database](https://rustsec.org/) for known CVEs, unsound code, and yanked crates.
- **cargo-deny** — enforces policy rules across the full dependency graph: license compliance, duplicate crate detection, banned crate enforcement, and source registry restrictions.

Both tools run in CI on every push and pull request. Developers are expected to run them locally before submitting changes.

---

## Approved License Policy

The following SPDX identifiers are permitted for all dependencies:

| Identifier | Notes |
|---|---|
| `MIT` | Primary permissive license |
| `Apache-2.0` | Primary permissive license |
| `BSD-2-Clause` | Permissive |
| `BSD-3-Clause` | Permissive |
| `ISC` | Permissive (functionally equivalent to MIT) |
| `0BSD` | Zero-clause BSD; currently used by `adler2` |
| `BSD-1-Clause` | Used by `fiat-crypto` (OR expression with MIT/Apache-2.0) |
| `Unlicense` | Public domain dedication; used by `memchr` (OR expression with MIT) |
| `Zlib` | Permissive; used by `miniz_oxide`, `tinyvec`, `tinyvec_macros` |
| `Unicode-3.0` | Unicode data license; used by `unicode-ident` (AND expression with MIT/Apache-2.0) |
| `Apache-2.0 WITH LLVM-exception` | Apache-2.0 with LLVM linking exception; used by `wasi`, `wasmparser`, `wit-bindgen` |

Any dependency whose license expression cannot be satisfied by the above identifiers requires an explicit exception documented in `deny.toml` and reviewed in the PR that introduces it.

**Prohibited licenses** include (but are not limited to): GPL-2.0, GPL-3.0, AGPL-3.0, SSPL, and any other copyleft license that would impose obligations on TrustLink's users or downstream contracts.

---

## Running Security Checks Locally

Before submitting a PR, run:

```bash
# Check for known CVEs and yanked crates
cargo audit

# Check licenses, advisories, bans, and sources
make deny
# or equivalently:
cargo deny check
```

To run individual cargo-deny checks:

```bash
cargo deny check licenses    # license compliance only
cargo deny check advisories  # CVE/advisory check only
cargo deny check bans        # duplicate and banned crate check only
cargo deny check sources     # registry source check only
```

---

## How Dependency Advisories Are Handled

When `cargo audit` or `cargo deny check advisories` reports a vulnerability:

1. **Assess impact** — determine whether the vulnerable code path is reachable from TrustLink's contract logic. Many advisories affect features (e.g. async, networking) that are not used in a `no_std` WASM contract.
2. **Update the dependency** — run `cargo update -p <crate>` to pull in a patched version if one exists.
3. **If no patch is available** — add the advisory ID to the `ignore` list in `deny.toml` with a documented reason and a tracking issue. Example:
   ```toml
   [advisories]
   ignore = [
       { id = "RUSTSEC-2024-XXXX", reason = "Affects async I/O path not present in WASM build; tracking #NNN" },
   ]
   ```
4. **Never ignore a vulnerability silently** — every `ignore` entry must have a `reason` field.

---

## Duplicate Dependency Review

`cargo deny check bans` warns when multiple versions of the same crate appear in the dependency graph. Duplicates increase binary size and can introduce subtle incompatibilities.

When a new duplicate warning appears:

1. Check whether `cargo update` can resolve it without breaking other constraints.
2. If the duplicate is unavoidable (e.g. two major versions required by different upstream crates), add a `skip` entry to `deny.toml` with a documented reason:
   ```toml
   [bans]
   skip = [
       { name = "some-crate", reason = "upstream-a requires 1.x, upstream-b requires 2.x" },
   ]
   ```
3. Review existing `skip` entries whenever `soroban-sdk` is upgraded — some duplicates may be resolved by a new SDK release and the corresponding `skip` entries should be removed.

---

## Banned Crates

The `[bans] deny` list in `deny.toml` is currently empty. Add entries here for crates that must never appear in the dependency graph, for example:

```toml
[bans]
deny = [
    { name = "openssl", reason = "use rustls instead" },
]
```

---

## Source Registry Policy

All dependencies must come from the official crates.io registry. Git dependencies and private registries are denied by `cargo deny check sources`. If a git dependency is temporarily required (e.g. an unreleased patch), it must be:

1. Approved in a PR with a documented reason.
2. Added to `[sources] allow-git` in `deny.toml`.
3. Replaced with a published crate version as soon as one is available.

---

## Adding New Dependencies

When adding a new dependency:

1. Prefer crates with `MIT` or `Apache-2.0` licenses.
2. Run `cargo deny check` before committing to confirm the new crate does not introduce a license violation, advisory, or banned crate.
3. If the crate introduces a new license identifier, add it to the `allow` list in `deny.toml` with a comment explaining which crate requires it.
4. Avoid adding dependencies with known security advisories. If unavoidable, document the reason in `deny.toml`.

---

## Dependency Updates

- Run `cargo update` periodically (at minimum before each release) to pull in patch-level fixes.
- After updating, re-run `make deny` and `cargo audit` to confirm no new advisories were introduced.
- Major version upgrades require a PR with a changelog review.
- `soroban-sdk` upgrades should be treated as high-priority since they affect the entire dependency tree.

---

## CI Integration

The CI pipeline (`ci.yml`) runs `cargo audit --deny warnings` in a dedicated `audit` job on every push and pull request. `cargo deny` is not yet in CI — add it by inserting a step after the audit job:

```yaml
- name: Run cargo-deny
  run: cargo deny check
```

Until then, run `make deny` locally before every PR.

---

## Exception Management

All exceptions to the default policy (ignored advisories, permitted duplicate versions, non-standard licenses) must be:

1. Documented in `deny.toml` with a `reason` field.
2. Reviewed in the PR that introduces them.
3. Re-evaluated on each `soroban-sdk` upgrade or quarterly, whichever comes first.
4. Removed as soon as the underlying issue is resolved (patch released, duplicate resolved, etc.).

---

## Quick Reference

| Task | Command |
|---|---|
| Check for CVEs | `cargo audit` |
| Full policy check | `make deny` |
| Licenses only | `cargo deny check licenses` |
| Advisories only | `cargo deny check advisories` |
| Bans/duplicates only | `cargo deny check bans` |
| Sources only | `cargo deny check sources` |
| Update all deps | `cargo update` |
| Install cargo-deny | `cargo install --locked cargo-deny` |
| Install cargo-audit | `cargo install --locked cargo-audit` |
