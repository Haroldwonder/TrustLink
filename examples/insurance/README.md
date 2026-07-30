# Insurance Policy Underwriting Example

This example shows how a Soroban insurance contract uses TrustLink to verify a policyholder before issuing coverage. It models a minimal underwriting flow: an insurer may only issue a policy to a subject who holds valid `KYC_PASSED` and `AML_CLEARED` attestations, keeping identity verification out of the insurer's own storage and delegated to TrustLink.

## What It Demonstrates

- A contract stores a TrustLink contract address at `initialize` time.
- `issue_policy` checks `has_all_claims(subject, ["KYC_PASSED", "AML_CLEARED"])` on the policyholder before underwriting.
- Policy issuance is rejected (the call panics) unless **both** identity claims are valid — `has_all_claims` uses AND-logic.
- Issued policies are numbered sequentially and stored so `get_policy_holder` can look up the holder of any policy.
- Unit tests cover both the allowed and blocked issuance flows using a mock TrustLink contract.

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

`issue_policy` requires the insurer's authorization (`insurer.require_auth()`), calls out to the configured TrustLink contract to evaluate the claims, and only then persists a new policy record keyed by an incrementing policy number.

## Scenario Walkthrough

1. **Deploy TrustLink** (or reuse an existing deployment) and register the issuers that will attest `KYC_PASSED` and `AML_CLEARED` for prospective policyholders.
2. **Deploy the insurance contract** and call `initialize(admin, trustlink_contract)` once, pointing it at the TrustLink deployment.
3. **Issue attestations** for a policyholder from the registered issuers — `KYC_PASSED` from a KYC provider, `AML_CLEARED` from an AML screening issuer.
4. **Issue the policy**: the insurer calls `issue_policy(insurer, policyholder)`. TrustLink is queried via `has_all_claims`; if the policyholder is missing either claim (or either has expired or been revoked), the call panics and no policy is created.
5. **Look up the policyholder** for any issued policy with `get_policy_holder(policy_id)`.

## Files

- `src/lib.rs`: Insurance contract and unit tests.
- `Cargo.toml`: Example crate configuration.

## Run Tests

```bash
cd examples/insurance
cargo test
```

The test suite uses a mock `TrustLink` contract (`MockTrustLink`) to exercise both branches:

| Scenario | Test |
|---|---|
| Policyholder has both `KYC_PASSED` and `AML_CLEARED` → policy issued | `issue_policy_allowed_for_policyholders_with_all_claims` |
| Policyholder is missing at least one claim → issuance rejected | `issue_policy_rejected_when_policyholder_missing_a_claim` |

## Deployment

### Prerequisites

```bash
cargo install --locked stellar-cli --features opt
rustup target add wasm32-unknown-unknown
```

### 1. Build

```bash
cd examples/insurance
cargo build --target wasm32-unknown-unknown --release
```

### 2. Deploy TrustLink (if you need your own instance)

```bash
export ADMIN_SECRET=SXXX...
cd ../..
make deploy NETWORK=testnet
export TRUSTLINK_ID=C...
```

### 3. Register the KYC and AML issuers

```bash
stellar contract invoke --id $TRUSTLINK_ID --source $ADMIN_SECRET --network testnet \
  -- register_issuer --admin <ADMIN_ADDRESS> --issuer <KYC_ISSUER_ADDRESS>

stellar contract invoke --id $TRUSTLINK_ID --source $ADMIN_SECRET --network testnet \
  -- register_issuer --admin <ADMIN_ADDRESS> --issuer <AML_ISSUER_ADDRESS>
```

### 4. Issue the required claims for a policyholder

```bash
stellar contract invoke --id $TRUSTLINK_ID --source <KYC_ISSUER_SECRET> --network testnet \
  -- create_attestation \
  --issuer <KYC_ISSUER_ADDRESS> \
  --subject <POLICYHOLDER_ADDRESS> \
  --claim_type KYC_PASSED \
  --expiration null \
  --metadata null

stellar contract invoke --id $TRUSTLINK_ID --source <AML_ISSUER_SECRET> --network testnet \
  -- create_attestation \
  --issuer <AML_ISSUER_ADDRESS> \
  --subject <POLICYHOLDER_ADDRESS> \
  --claim_type AML_CLEARED \
  --expiration null \
  --metadata null
```

### 5. Deploy and initialize the insurance contract

```bash
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/insurance_example.wasm \
  --source $ADMIN_SECRET \
  --network testnet
export INSURANCE_ID=C...

stellar contract invoke --id $INSURANCE_ID --source $ADMIN_SECRET --network testnet \
  -- initialize \
  --admin <ADMIN_ADDRESS> \
  --trustlink_contract $TRUSTLINK_ID
```

### 6. Issue a policy

```bash
stellar contract invoke --id $INSURANCE_ID --source <INSURER_SECRET> --network testnet \
  -- issue_policy \
  --insurer <INSURER_ADDRESS> \
  --policyholder <POLICYHOLDER_ADDRESS>

stellar contract invoke --id $INSURANCE_ID --network testnet \
  -- get_policy_holder --policy_id 1
```

## Production Notes

- Set a meaningful `expiration` on `KYC_PASSED` and `AML_CLEARED` attestations rather than leaving them permanent — most underwriting programs require periodic re-verification.
- Use `has_valid_claim_from_issuer` instead of `has_all_claims` if only specific accredited KYC/AML providers should be trusted for underwriting decisions.
- Subscribe to `Revoked` events from TrustLink in an off-chain indexer so previously-issued policies can be flagged for review if a policyholder's claims are later revoked.
- Keep sensitive underwriting data (health details, financial history) off-chain; TrustLink attestations here only carry a boolean claim of KYC/AML status, not the underlying documentation.
