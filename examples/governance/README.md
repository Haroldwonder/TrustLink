# Governance/DAO Voting Example (TrustLink Integration)

This example demonstrates how a Soroban governance contract can gate voting on KYC verification using TrustLink.

## What It Demonstrates

- A governance contract stores a TrustLink contract address
- `vote()` checks `has_valid_claim(voter, "KYC_PASSED")` before allowing a vote
- Voting reverts if the voter is not KYC-verified
- Unit tests cover blocked and allowed voting flows

## Contract Pattern

The key voting guard is:

```rust
let trustlink_client = trustlink::Client::new(&env, &trustlink);
let kyc_claim = String::from_str(&env, "KYC_PASSED");
let has_kyc = trustlink_client.has_valid_claim(&voter, &kyc_claim);

if !has_kyc {
    return Err(String::from_str(&env, "voter must have valid KYC"));
}
```

## Files

- `src/lib.rs`: Governance contract + tests
- `Cargo.toml`: Contract dependencies

## Run Tests

```bash
cd examples/governance
cargo test
```

## Deployment

1. Build the contract:
```bash
cargo build --target wasm32-unknown-unknown --release
```

2. Deploy to testnet:
```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/governance_example.wasm \
  --source <YOUR_SECRET_KEY> \
  --network testnet
```

3. Initialize with TrustLink contract address:
```bash
soroban contract invoke \
  --id <GOVERNANCE_CONTRACT_ID> \
  --source <YOUR_SECRET_KEY> \
  --network testnet \
  -- initialize \
  --trustlink <TRUSTLINK_CONTRACT_ID>
```

4. Cast a vote:
```bash
soroban contract invoke \
  --id <GOVERNANCE_CONTRACT_ID> \
  --source <VOTER_SECRET_KEY> \
  --network testnet \
  -- vote \
  --voter <VOTER_ADDRESS> \
  --proposal_id 1 \
  --vote true
```

## Production Notes

- In production, replace error strings with typed contract errors
- Consider implementing proposal expiration and finalization logic
- Add vote weight based on token holdings or other criteria
- Implement vote delegation for proxy voting
- Consider issuer-specific policies using `has_valid_claim_from_issuer`
- Add event emission for vote tracking and indexing
