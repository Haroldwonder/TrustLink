# Mutation Testing Guide

## Overview

Mutation testing validates that the test suite can catch subtle bugs introduced by deliberate code mutations. Unlike traditional code coverage metrics, mutation testing proves that tests would actually fail when code is broken.

For TrustLink, mutation testing focuses on critical security-sensitive code paths in `src/validation.rs` and `src/attestation.rs`.

## Running Mutation Tests

### Prerequisites

Install `cargo-mutants`:

```bash
cargo install cargo-mutants
```

### Running Tests

Execute mutation testing on validation and attestation modules:

```bash
# Test all validation logic
cargo mutants --package trustlink --file src/validation.rs

# Test attestation creation and revocation
cargo mutants --package trustlink --file src/attestation.rs

# Test with verbose output
cargo mutants --package trustlink -v --file src/validation.rs
```

### Understanding Results

`cargo-mutants` will:
1. Introduce deliberate bugs (mutations) into the code
2. Run the test suite for each mutation
3. Report which mutations were **killed** (caught by tests) vs. **survived** (missed by tests)

Expected output:
- **Killed**: Test suite caught the bug ✅
- **Survived**: Test suite missed the bug ❌ (indicates missing test coverage)

## Surviving Mutants & Remediation

### Currently Tracked Surviving Mutants

None – all mutations in validation and attestation are caught by the test suite.

### How to Handle Survivors

If mutation testing reveals survivors, add targeted tests to `tests/mutation_testing.rs`:

1. **Identify the mutation** – e.g., "changed `!=` to `==` in admin check"
2. **Understand the risk** – e.g., "would allow unauthorized users to perform admin operations"
3. **Write a test** – e.g., verify that an unauthorized user's operation fails

Example template for a survivor fix:

```rust
#[test]
fn test_mutation_inverted_check_would_fail() {
    let env = Env::default();
    let admin = Address::random(&env);
    let unauthorized = Address::random(&env);

    admin.require_auth();
    contract.initialize(&env, &admin, &None);

    // This should fail if the authorization check is inverted
    unauthorized.require_auth();
    assert!(contract.operation(&env, &unauthorized).is_err());
}
```

## Mutation Classes Covered

### 1. Inverted Authorization Checks

**Mutation**: `!=` → `==` or vice versa

**Tests**: 
- `test_require_admin_inverted_check_would_fail`
- `test_require_issuer_must_return_error`

**Risk**: Allows unauthorized access to admin/issuer functions

### 2. Removed Authorization Checks

**Mutation**: Delete `if !Storage::is_issuer(...)` line

**Tests**:
- `test_require_issuer_must_return_error`
- `test_require_authorized_creator_accepts_either_issuer_or_bridge`

**Risk**: Anyone can create/revoke attestations

### 3. Short-Circuit Logic Changes

**Mutation**: `||` → `&&`

**Tests**:
- `test_require_authorized_creator_accepts_either_issuer_or_bridge`

**Risk**: Requires both issuer AND bridge status (too restrictive, breaks bridges)

### 4. Boundary Condition Mutations

**Mutation**: `len > 64` → `len >= 64`

**Tests**:
- `test_validate_claim_type_boundary_64_chars`
- `test_validate_metadata_boundary_256_chars`

**Risk**: Off-by-one errors in length validation

### 5. Early-Return Mutations

**Mutation**: Remove `return Err(Error::Unauthorized)` statements

**Tests**:
- `test_require_not_paused_blocks_operations`
- `test_require_admin_short_circuits_on_first_unauthorized`

**Risk**: Authorization bypass; state mutations before auth checks

## CI Integration

Mutation testing can be integrated into CI to prevent regressions:

```yaml
# .github/workflows/mutation-testing.yml
- name: Run mutation tests
  run: |
    cargo install cargo-mutants
    cargo mutants --package trustlink --file src/validation.rs --file src/attestation.rs
    # Fail if any mutations survive
    if [ $? -ne 0 ]; then exit 1; fi
```

## Best Practices

1. **Run mutations regularly** – At least before security audits
2. **Fix survivors immediately** – Each survivor represents a potential security gap
3. **Keep mutation tests updated** – As new code paths are added, add corresponding mutation tests
4. **Document surviving mutants** – If a mutation is acceptable, explain why (rare for security code)
5. **Use mutation testing alongside fuzzing** – Complements fuzzing for comprehensive coverage

## References

- [Stryker.js Mutation Testing Guide](https://stryker-mutator.io/docs/mutation-testing-elements/mutation-operators/)
- [PIT (for Java, similar concepts)](https://pitest.org/)
- [cargo-mutants GitHub](https://github.com/sourcefrog/cargo-mutants)
