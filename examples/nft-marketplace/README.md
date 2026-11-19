# NFT Marketplace — KYC + Jurisdiction Gating (TrustLink Integration)

This example shows a Soroban NFT marketplace that gates **listing** and **bidding** behind two TrustLink checks:

1. A valid `KYC_PASSED` attestation (`has_valid_claim`)
2. At least one attestation in the marketplace's allowed jurisdiction (`get_attestations_by_jurisdiction`)

This is a realistic compliance pattern for regulated NFT marketplaces.

## What It Demonstrates

- Marketplace stores a TrustLink contract address and an allowed ISO jurisdiction code (e.g. `US`).
- `list_item` requires the seller to pass both KYC and jurisdiction eligibility.
- `place_bid` requires the bidder to pass both KYC and jurisdiction eligibility.
- Unit tests cover allowed and rejected flows for listing and bidding.

## Contract Pattern

```rust
let kyc_claim = String::from_str(&env, "KYC_PASSED");
if !trustlink.has_valid_claim(&wallet, &kyc_claim) {
    panic!("KYC_PASSED attestation required");
}

let allowed = /* configured marketplace jurisdiction, e.g. "US" */;
let hits = trustlink.get_attestations_by_jurisdiction(&wallet, &allowed, &0, &1);
if hits.is_empty() {
    panic!("jurisdiction eligibility required");
}
```

Both `list_item` and `place_bid` call this guard before mutating marketplace state.

## Test Coverage

| Scenario | Test |
|---|---|
| List allowed — KYC + jurisdiction OK | `list_allowed_when_kyc_and_jurisdiction_ok` |
| List rejected — missing KYC | `list_rejected_without_kyc` / `list_rejected_without_kyc_seedable` |
| List rejected — wrong jurisdiction | `list_rejected_without_jurisdiction_eligibility` / `list_rejected_without_jurisdiction_seedable` |
| Bid allowed — KYC + jurisdiction OK | `bid_allowed_when_kyc_and_jurisdiction_ok` / `bid_allowed_with_seedable_eligible_parties` |
| Bid rejected — missing KYC | `bid_rejected_without_kyc_seedable` |
| Bid rejected — wrong jurisdiction | `bid_rejected_without_jurisdiction_eligibility` |

## Files

- `src/lib.rs` — marketplace contract + unit tests
- `Cargo.toml` — example crate dependencies

## Run Tests

```bash
cd examples/nft-marketplace
cargo test
```

## Deployment

### Prerequisites

```bash
cargo install --locked stellar-cli --features opt
rustup target add wasm32-unknown-unknown
```

### 1. Build the contract

```bash
cd examples/nft-marketplace
cargo build --target wasm32-unknown-unknown --release
```

WASM artifact:
`target/wasm32-unknown-unknown/release/nft_marketplace_example.wasm`

### 2. Deploy TrustLink (if needed)

```bash
export ADMIN_SECRET=SXXX...
cd ../..
make deploy NETWORK=testnet
export TRUSTLINK_ID=C...
```

### 3. Deploy the marketplace

```bash
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/nft_marketplace_example.wasm \
  --source $ADMIN_SECRET \
  --network testnet
export MARKETPLACE_ID=C...
```

### 4. Initialize

```bash
stellar contract invoke \
  --id $MARKETPLACE_ID \
  --source $ADMIN_SECRET \
  --network testnet \
  -- initialize \
  --admin <ADMIN_ADDRESS> \
  --trustlink_contract $TRUSTLINK_ID \
  --allowed_jurisdiction US
```

### 5. Issue eligibility attestations

Sellers and bidders need a `KYC_PASSED` attestation that includes the marketplace jurisdiction (via `create_attestation_jurisdiction` on TrustLink), for example jurisdiction `US`.

### 6. List and bid

```bash
# List
stellar contract invoke \
  --id $MARKETPLACE_ID \
  --source <SELLER_SECRET> \
  --network testnet \
  -- list_item \
  --seller <SELLER_ADDRESS> \
  --token_id nft-001 \
  --price 1000

# Bid
stellar contract invoke \
  --id $MARKETPLACE_ID \
  --source <BIDDER_SECRET> \
  --network testnet \
  -- place_bid \
  --bidder <BIDDER_ADDRESS> \
  --listing_id 1 \
  --amount 1250
```

## Production Notes

- Replace panic strings with typed contract errors.
- Consider issuer-specific KYC via `has_valid_claim_from_issuer`.
- Jurisdiction codes must be uppercase ISO 3166-1 alpha-2 (TrustLink validates this at issuance).
- Decide whether settlements, transfers, and withdrawals should reuse the same eligibility guard.
