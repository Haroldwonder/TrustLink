# Insurance Policy Underwriting Example

This example shows how a Soroban insurance contract uses TrustLink to verify a policyholder before issuing coverage.

## What It Demonstrates

- A contract stores a TrustLink contract address.
- `issue_policy` checks `has_all_claims(subject, ["KYC_PASSED", "AML_CLEARED"])` on the policyholder.
- Policy issuance is rejected unless both identity claims are valid.
- Unit tests cover both allowed and blocked issuance flows.

## Contract Pattern

The key underwriting guard is:

```rust
let mut required_claims: Vec<String> = Vec::new(&env);
required_claims.push_back(String::from_str(&env, "KYC_PASSED"));
required_claims.push_back(String::from_str(&env, "AML_CLEARED"));

if !trustlink.has_all_claims(&policyholder, &required_claims) {
    panic!("policyholder must have valid KYC_PASSED and AML_CLEARED claims");
}
```

## Files

- `src/lib.rs`: Insurance contract and unit tests.
- `Cargo.toml`: Example crate configuration.

## Run Tests

```bash
cd examples/insurance
cargo test
```
