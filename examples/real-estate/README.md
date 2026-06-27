# Real-Estate Title Registry Example (TrustLink Integration)

This example demonstrates **long-lived attestations** — the opposite end of the TTL spectrum from short-lived KYC credentials. Property title and lien status change rarely; a `CLEAR_TITLE` attestation may be valid for a decade, renewed only when the property is refinanced or transferred.

## What It Demonstrates

- A **title-registry issuer** attests `CLEAR_TITLE` on a property address with a multi-year expiration.
- A **lien-holder** (bank or judgment creditor) attests `LIEN_ENCUMBRANCE` when a mortgage or lien is recorded.
- A **buyer's contract** calls `verify_purchase_eligibility`, which uses:
  - `has_valid_claim(property, "CLEAR_TITLE")` — property must have registered title.
  - `has_valid_claim(property, "LIEN_ENCUMBRANCE")` — purchase is blocked if a lien is active.
- A helper method uses `has_any_claim` to check for any encumbrance type (extensible to `TAX_LIEN`, `JUDGMENT_LIEN`, etc.).

## TTL / Expiration Patterns

| Claim type | Typical expiration | Renewal trigger |
|---|---|---|
| `CLEAR_TITLE` | 10 years | Property transfer or refinance |
| `LIEN_ENCUMBRANCE` | Mortgage term (e.g. 30 years) | Refinance, payoff, or satisfaction |

On the real TrustLink contract, set a long expiration at creation time:

```rust
// 10-year title attestation (seconds)
let ten_years = env.ledger().timestamp() + (10 * 365 * 24 * 60 * 60);
contract.create_attestation(
    &title_registry_issuer,
    &property_address,
    &String::from_str(&env, "CLEAR_TITLE"),
    &Some(ten_years),
    &None,
);
```

Renew before expiry using `renew_attestation`:

```rust
let new_expiry = env.ledger().timestamp() + (10 * 365 * 24 * 60 * 60);
contract.renew_attestation(&title_registry_issuer, &attestation_id, &new_expiry);
```

## Contract Pattern

```rust
// Buyer's eligibility check — called by escrow contract before releasing funds
pub fn verify_purchase_eligibility(env: Env, property_id: u32) -> bool {
    let tl = TrustLinkClient::new(&env, &trustlink_id);

    let has_clear = tl.has_valid_claim(&property_address, &clear_title_claim);
    let has_lien  = tl.has_valid_claim(&property_address, &lien_claim);

    has_clear && !has_lien
}
```

## Test Coverage

| Scenario | Test |
|---|---|
| Clear title, no lien → purchase allowed | `purchase_allowed_when_clear_title_and_no_lien` |
| Clear title + active lien → purchase blocked | `purchase_blocked_when_lien_present` |
| No title registered → purchase blocked | `purchase_blocked_when_no_clear_title` |
| Encumbrance check: no lien | `no_encumbrance_on_clean_title` |
| Encumbrance check: lien present | `encumbrance_detected_when_lien_present` |
| Expired title attestation → purchase blocked | `purchase_blocked_when_title_attestation_expired` |
| Sequential property ID assignment | `register_multiple_properties_returns_sequential_ids` |
| Property record retrieval | `get_property_returns_correct_record` |

## Files

- `src/lib.rs` — title registry contract and unit tests
- `Cargo.toml` — example crate dependencies

## Run Tests

```bash
cd examples/real-estate
cargo test
```

## Deployment

### Prerequisites

```bash
cargo install --locked stellar-cli --features opt
rustup target add wasm32-unknown-unknown
```

### 1. Build

```bash
cd examples/real-estate
cargo build --target wasm32-unknown-unknown --release
```

### 2. Deploy TrustLink (if you need your own instance)

```bash
export ADMIN_SECRET=SXXX...
cd ../..
make deploy NETWORK=testnet
export TRUSTLINK_ID=C...
```

### 3. Deploy the title-registry contract

```bash
export ADMIN_SECRET=SXXX...
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/real_estate_example.wasm \
  --source $ADMIN_SECRET \
  --network testnet
export TITLE_REGISTRY_ID=C...
```

### 4. Initialize

```bash
stellar contract invoke \
  --id $TITLE_REGISTRY_ID \
  --source $ADMIN_SECRET \
  --network testnet \
  -- initialize \
  --admin <ADMIN_ADDRESS> \
  --trustlink_contract $TRUSTLINK_ID
```

### 5. Register an issuer and attest clear title

```bash
# Register the title-registry office as an issuer in TrustLink
stellar contract invoke \
  --id $TRUSTLINK_ID \
  --source $ADMIN_SECRET \
  --network testnet \
  -- register_issuer \
  --admin <ADMIN_ADDRESS> \
  --issuer <TITLE_REGISTRY_ADDRESS>

# Attest clear title with a 10-year expiration
stellar contract invoke \
  --id $TRUSTLINK_ID \
  --source <TITLE_REGISTRY_SECRET> \
  --network testnet \
  -- create_attestation \
  --issuer <TITLE_REGISTRY_ADDRESS> \
  --subject <PROPERTY_ADDRESS> \
  --claim_type CLEAR_TITLE \
  --expiration <UNIX_TIMESTAMP_10_YEARS>
```

### 6. Verify purchase eligibility

```bash
stellar contract invoke \
  --id $TITLE_REGISTRY_ID \
  --network testnet \
  -- verify_purchase_eligibility \
  --property_id 1
```

## Production Notes

- Use a dedicated on-chain `Address` per property (derived deterministically from the parcel ID) rather than the title-office address.
- `LIEN_ENCUMBRANCE` attestations should be issued by registered lien holders (banks, tax authorities) — not by the title registry — to preserve the separation of duties.
- Consider using `has_valid_claim_from_issuer` when only a specific title authority's certification is acceptable.
- For multi-jurisdiction portability, use `has_any_claim` with a list of claim types from different registries.
