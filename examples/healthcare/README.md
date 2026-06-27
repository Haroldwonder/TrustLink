# Healthcare Credential Verification Example (TrustLink Integration)

This example demonstrates the **privacy-sensitive** end of the attestation spectrum. Healthcare credentials — provider licences, vaccination records — are among the most regulated data categories under GDPR Art. 9 and HIPAA. The patterns here align with the data-minimisation principles in [docs/compliance.md](../../docs/compliance.md).

## What It Demonstrates

- A **licensing-board issuer** attests `PROVIDER_LICENSED` for a registered healthcare provider.
- A **public-health issuer** attests `VACCINATION_VERIFIED` for a patient.
- A **verifier contract** exposes three access-control entry-points:
  - `verify_provider_license` — a telehealth or prescription platform checks that a provider is licensed before granting clinical access.
  - `verify_vaccination` — a care facility checks that a patient has a verified vaccination record before entry.
  - `verify_full_credentials` — an ICU or high-risk role check using `has_all_claims` (AND-logic) to require *both* claims simultaneously.

## Data-Minimisation Pattern

The `metadata` field on every `create_attestation` call is **`None`** in this example. This is deliberate:

> Integrators MUST NOT store names, identifiers, or clinical data in the on-chain `metadata` field. Use a hashed reference to an off-chain record if a linkage is needed.
> — [docs/compliance.md § Data Minimisation](../../docs/compliance.md#data-minimisation)

The only information that crosses the contract boundary is a **boolean** (`true`/`false`). No licence number, no vaccine lot, no patient date-of-birth is ever on-chain.

### Recommended expiration periods

| Claim type | Suggested expiration | Rationale |
|---|---|---|
| `PROVIDER_LICENSED` | 1 year | Matches most annual re-credentialing cycles |
| `VACCINATION_VERIFIED` | 2 years | Aligns with typical booster cadence for mandated vaccines |

Set expiration at issuance time:

```rust
let one_year = env.ledger().timestamp() + (365 * 24 * 60 * 60);
contract.create_attestation(
    &licensing_board_issuer,
    &provider_address,
    &String::from_str(&env, "PROVIDER_LICENSED"),
    &Some(one_year),
    &None,  // ← no metadata: data minimisation
);
```

## Contract Pattern

```rust
// Licensing check — called by telehealth platform before granting clinical access
pub fn verify_provider_license(env: Env, provider: Address) -> bool {
    let tl = TrustLinkClient::new(&env, &trustlink_id);
    tl.has_valid_claim(&provider, &String::from_str(&env, "PROVIDER_LICENSED"))
}

// Full credentialing — requires both claims (AND-logic)
pub fn verify_full_credentials(env: Env, provider: Address) -> bool {
    let tl = TrustLinkClient::new(&env, &trustlink_id);
    let mut required = Vec::new(&env);
    required.push_back(String::from_str(&env, "PROVIDER_LICENSED"));
    required.push_back(String::from_str(&env, "VACCINATION_VERIFIED"));
    tl.has_all_claims(&provider, &required)
}
```

## Test Coverage

| Scenario | Test |
|---|---|
| Licensed provider → access granted | `licensed_provider_is_verified` |
| Unlicensed provider → access denied | `unlicensed_provider_is_rejected` |
| Expired licence → access denied | `provider_with_expired_license_is_rejected` |
| Vaccinated patient → entry allowed | `vaccinated_patient_is_verified` |
| Unvaccinated patient → entry denied | `unvaccinated_patient_is_rejected` |
| Both credentials valid → full check passes | `full_credentials_pass_when_both_claims_valid` |
| Vaccination missing → full check fails | `full_credentials_fail_when_vaccination_missing` |
| Licence missing → full check fails | `full_credentials_fail_when_license_missing` |
| No claims → full check fails | `full_credentials_fail_when_no_claims_at_all` |
| Return type is bool only (no PII) | `provider_check_returns_only_boolean` |
| Return type is bool only (no PII) | `vaccination_check_returns_only_boolean` |

## Files

- `src/lib.rs` — verifier contract and unit tests
- `Cargo.toml` — example crate dependencies

## Run Tests

```bash
cd examples/healthcare
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
cd examples/healthcare
cargo build --target wasm32-unknown-unknown --release
```

### 2. Deploy TrustLink (if you need your own instance)

```bash
export ADMIN_SECRET=SXXX...
cd ../..
make deploy NETWORK=testnet
export TRUSTLINK_ID=C...
```

### 3. Register issuers in TrustLink

```bash
# Register the licensing board
stellar contract invoke --id $TRUSTLINK_ID --source $ADMIN_SECRET --network testnet \
  -- register_issuer --admin <ADMIN_ADDRESS> --issuer <LICENSING_BOARD_ADDRESS>

# Register the public-health authority
stellar contract invoke --id $TRUSTLINK_ID --source $ADMIN_SECRET --network testnet \
  -- register_issuer --admin <ADMIN_ADDRESS> --issuer <PUBLIC_HEALTH_ADDRESS>
```

### 4. Issue credentials (minimal metadata — no PII)

```bash
# Provider licence (1-year expiry)
stellar contract invoke --id $TRUSTLINK_ID --source <LICENSING_BOARD_SECRET> --network testnet \
  -- create_attestation \
  --issuer <LICENSING_BOARD_ADDRESS> \
  --subject <PROVIDER_ADDRESS> \
  --claim_type PROVIDER_LICENSED \
  --expiration <UNIX_TIMESTAMP_1_YEAR> \
  --metadata null

# Vaccination record (2-year expiry, no patient details)
stellar contract invoke --id $TRUSTLINK_ID --source <PUBLIC_HEALTH_SECRET> --network testnet \
  -- create_attestation \
  --issuer <PUBLIC_HEALTH_ADDRESS> \
  --subject <PATIENT_ADDRESS> \
  --claim_type VACCINATION_VERIFIED \
  --expiration <UNIX_TIMESTAMP_2_YEARS> \
  --metadata null
```

### 5. Deploy and initialise the verifier contract

```bash
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/healthcare_example.wasm \
  --source $ADMIN_SECRET \
  --network testnet
export VERIFIER_ID=C...

stellar contract invoke --id $VERIFIER_ID --source $ADMIN_SECRET --network testnet \
  -- initialize \
  --admin <ADMIN_ADDRESS> \
  --trustlink_contract $TRUSTLINK_ID
```

### 6. Verify credentials

```bash
stellar contract invoke --id $VERIFIER_ID --network testnet \
  -- verify_provider_license --provider <PROVIDER_ADDRESS>

stellar contract invoke --id $VERIFIER_ID --network testnet \
  -- verify_full_credentials --provider <PROVIDER_ADDRESS>
```

## Production Notes

- Never put names, identifiers, or clinical data in the `metadata` field. See [docs/compliance.md](../../docs/compliance.md).
- Use `has_valid_claim_from_issuer` when only a specific licensing authority's certification is acceptable (e.g. a particular state medical board).
- Subscribe to the `DeletionRequested` event in your off-chain indexer to honour patient right-to-erasure requests promptly.
- Consider a short Soroban ledger TTL (e.g. 30 days) combined with an `expiration` of 1–2 years so ledger entries are archived when not actively queried, reducing storage costs.
- For multi-jurisdiction deployments, register issuers from each jurisdiction and use `has_any_claim` to accept credentials from any recognised authority.
