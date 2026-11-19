# Cargo.lock Management Policy

## Overview

This document describes TrustLink's policy for handling `Cargo.lock` files across the repository structure.

## Policy

### Root Contract (`/Cargo.lock`)
**Status:** ❌ **NOT tracked in git**

**Rationale:**
- TrustLink is a **library crate** (Soroban smart contract)
- Library crates should NOT commit `Cargo.lock` to git
- This allows downstream users (integrators, other contracts) to use their own dependency versions
- Follows Rust community best practice: [Cargo docs - Should I commit Cargo.lock?](https://doc.rust-lang.org/cargo/guide/cargo-lock.html)

**How it works:**
- Root `.gitignore` excludes `Cargo.lock`
- Developers building locally will have a `Cargo.lock` generated automatically
- CI/CD can specify exact versions via `--locked` flag if reproducibility is critical
- Example: `cargo test --locked` in CI pipelines

### Example Crates (Future)
If example or test crates are added in the future:

**For executable examples:** `Cargo.lock` SHOULD be committed
- Examples are meant to be runnable references
- Committed `Cargo.lock` ensures examples work with exact tested versions

**For library examples:** `Cargo.lock` should NOT be committed
- Same rationale as root contract

## Implementation Details

### `.gitignore` Pattern
```
# Rust
Cargo.lock
```

### Verification
To verify `Cargo.lock` is excluded:
```bash
# This should show no output (no tracked Cargo.lock files)
git ls-files | grep Cargo.lock

# Show what's ignored
git check-ignore Cargo.lock
```

### CI/CD Considerations
To ensure reproducible builds in CI:

**Option 1: Use --locked flag**
```bash
cargo build --locked
cargo test --locked
```

**Option 2: Pin versions in Cargo.toml**
```toml
[dependencies]
soroban-sdk = "=21.0.0"  # Exact version
```

**Option 3: Create a lockfile in CI only**
Generate `Cargo.lock` during CI build without committing it.

## Decision Record

- **Date:** 2026
- **Status:** Established
- **Decision:** Exclude `Cargo.lock` from root and all crates
- **Rationale:** Best practice for library/contract crates; flexibility for integrators
- **Alternative Rejected:** Committing `Cargo.lock` - would limit dependency flexibility for users

## Related Files
- `.gitignore` - Main ignore rules
- `Cargo.toml` - Dependency specifications
- Continuous Integration pipeline documentation

## See Also
- [Rust Cargo Book - When to commit Cargo.lock](https://doc.rust-lang.org/cargo/guide/cargo-lock.html)
- [Soroban SDK Dependency Guidelines](https://soroban.stellar.org/docs)
