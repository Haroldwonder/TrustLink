# KYC-Restricted Soroban Token (TrustLink Integration)

This example shows how a Soroban token contract can enforce KYC by querying TrustLink before transfers.

## What It Demonstrates

- A token contract stores a TrustLink contract address.
- `transfer` checks `has_valid_claim(subject, "KYC_PASSED")` for both sender and receiver.
- Transfer reverts if either party is not KYC-verified.
- Unit tests cover blocked and allowed flows.

## Contract Pattern

The key transfer guard is:

```rust
let claim = String::from_str(&env, "KYC_PASSED");
let from_kyc = trustlink.has_valid_claim(&from, &claim);
let to_kyc = trustlink.has_valid_claim(&to, &claim);

if !from_kyc || !to_kyc {
    panic!("kyc required for sender and receiver");
}
```

## Files

- `src/lib.rs`: Example contract + tests
- `Cargo.toml`: Example crate dependencies

## Run Tests

```bash
cd examples/kyc-token
cargo test
```

## Production Notes

- In production, replace panic strings with typed contract errors.
- Consider issuer-specific policies using TrustLink `has_valid_claim_from_issuer`.
- Decide whether to gate sender only or both sender/receiver based on regulatory needs.
