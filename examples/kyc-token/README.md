# KYC-Restricted Soroban Token (TrustLink Integration)

This example shows how a Soroban token contract can enforce KYC by querying TrustLink before transfers.

## What It Demonstrates

- A token contract stores a TrustLink contract address.
- `transfer` checks `has_valid_claim(subject, "KYC_PASSED")` for both sender and receiver.
- Transfer reverts if either party is not KYC-verified.
- Unit tests cover all blocked and allowed flows.

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

## Test Coverage

| Scenario | Test |
|---|---|
| Transfer blocked — sender lacks KYC | `transfer_blocked_when_sender_lacks_kyc` |
| Transfer blocked — recipient lacks KYC | `transfer_blocked_when_recipient_lacks_kyc` |
| Transfer blocked — neither party has KYC | `transfer_blocked_for_non_kyc_address` |
| Transfer allowed — both parties have KYC | `transfer_allowed_for_kyc_addresses` |

## Files

- `src/lib.rs`: Example contract + tests
- `Cargo.toml`: Example crate dependencies

## Run Tests

```bash
cd examples/kyc-token
cargo test
```

## Deployment

### Prerequisites

```bash
# Install Stellar CLI
cargo install --locked stellar-cli --features opt

# Add WASM target
rustup target add wasm32-unknown-unknown
```

### 1. Build the contract

```bash
cd examples/kyc-token
cargo build --target wasm32-unknown-unknown --release
```

The WASM artifact will be at:
`target/wasm32-unknown-unknown/release/kyc_token.wasm`

### 2. Deploy TrustLink (if not already deployed)

```bash
export ADMIN_SECRET=SXXX...
cd ../..
make deploy NETWORK=testnet
# Note the CONTRACT_ID printed by the command
export TRUSTLINK_ID=C...
```

### 3. Deploy the KYC token contract

```bash
export ADMIN_SECRET=SXXX...
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/kyc_token.wasm \
  --source $ADMIN_SECRET \
  --network testnet
# Note the KYC_TOKEN_ID printed
export KYC_TOKEN_ID=C...
```

### 4. Initialize the KYC token

```bash
stellar contract invoke \
  --id $KYC_TOKEN_ID \
  --source $ADMIN_SECRET \
  --network testnet \
  -- initialize \
  --admin <ADMIN_ADDRESS> \
  --trustlink_contract $TRUSTLINK_ID
```

### 5. Mint tokens to a KYC-verified address

```bash
# First ensure the recipient has a KYC_PASSED attestation in TrustLink
stellar contract invoke \
  --id $KYC_TOKEN_ID \
  --source $ADMIN_SECRET \
  --network testnet \
  -- mint \
  --to <RECIPIENT_ADDRESS> \
  --amount 1000
```

### 6. Transfer tokens (both parties must have KYC)

```bash
stellar contract invoke \
  --id $KYC_TOKEN_ID \
  --source <SENDER_SECRET> \
  --network testnet \
  -- transfer \
  --from <SENDER_ADDRESS> \
  --to <RECIPIENT_ADDRESS> \
  --amount 100
```

## Production Notes

- In production, replace panic strings with typed contract errors.
- Consider issuer-specific policies using TrustLink `has_valid_claim_from_issuer`.
- Decide whether to gate sender only or both sender/receiver based on regulatory needs.
- The `set_trustlink` function allows the admin to update the TrustLink contract address after deployment (e.g., for upgrades).
