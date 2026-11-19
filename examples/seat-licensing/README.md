# Multi-tenant SaaS Seat-License Verification (TrustLink Integration)

This example shows how a B2B SaaS product can use TrustLink attestations for
**per-seat licensing**: a company issues `SEAT_LICENSED` attestations to employee
wallets, a resource contract gates access with `has_valid_claim`, and the company
admin revokes the seat when an employee is offboarded.

This is a distinct commercial pattern from the KYC-token and freelance-reputation
examples — the subject is an employee seat, the issuer is the SaaS customer's
organization, and revocation is the offboarding control plane.

## What It Demonstrates

1. **Issuance** — An organization issuer creates a `SEAT_LICENSED` attestation for
   an employee's address (simulated in tests via `MockTrustLink::issue_seat`; on
   mainnet via TrustLink `create_attestation`).
2. **Access gating** — `SeatLicensedResource::access_premium_feature` requires a
   valid seat from the configured org issuer before granting access.
3. **Offboarding revocation** — Revoking the attestation immediately blocks
   further access (`has_valid_claim` / `has_valid_claim_from_issuer` return false).
4. **Multi-tenant isolation** — Seats issued by a different organization do not
   unlock this tenant's resource (`has_valid_claim_from_issuer`).

## Claim Type

| Claim type | Issuer | Meaning |
|---|---|---|
| `SEAT_LICENSED` | SaaS customer org | Employee holds an active licensed seat |

## Contract Pattern

```rust
pub fn has_active_seat(env: Env, employee: Address) -> bool {
    let tl = TrustLinkClient::new(&env, &trustlink_id);
    let org_issuer = /* stored tenant issuer */;
    tl.has_valid_claim_from_issuer(
        &employee,
        &String::from_str(&env, "SEAT_LICENSED"),
        &org_issuer,
    )
}

pub fn access_premium_feature(env: Env, employee: Address) -> u32 {
    employee.require_auth();
    if !Self::has_active_seat(env.clone(), employee.clone()) {
        panic!("active SEAT_LICENSED claim required");
    }
    // …grant feature access…
}
```

## TrustLink Lifecycle (production)

### Issue a seat (employee onboarding)

```bash
stellar contract invoke \
  --id $TRUSTLINK_ID \
  --source $ORG_ISSUER_SECRET \
  --network testnet \
  -- create_attestation \
  --issuer $ORG_ISSUER_ADDRESS \
  --subject $EMPLOYEE_ADDRESS \
  --claim_type SEAT_LICENSED \
  --expiration null \
  --metadata null \
  --tags null
```

### Gate access (resource contract)

The resource contract calls `has_valid_claim` (or preferably
`has_valid_claim_from_issuer` for multi-tenant isolation) before serving the
feature. No attestation payload is needed — only the boolean result.

### Revoke on offboarding

```bash
stellar contract invoke \
  --id $TRUSTLINK_ID \
  --source $ORG_ISSUER_SECRET \
  --network testnet \
  -- revoke_attestation \
  --issuer $ORG_ISSUER_ADDRESS \
  --attestation_id $SEAT_ATTESTATION_ID \
  --reason '"employee_offboarded"'
```

After revocation, `has_valid_claim` returns `false` and the gated resource
rejects the former employee.

## Test Coverage

| Scenario | Test |
|---|---|
| Issuance makes seat claim valid | `seat_issuance_makes_claim_valid` |
| Access granted with valid seat | `access_granted_with_valid_seat` |
| Access denied without seat | `access_denied_without_seat` |
| Foreign-tenant seat rejected | `access_denied_for_foreign_tenant_seat` |
| Offboarding revocation blocks access | `revocation_on_offboarding_blocks_access` |
| Only matching issuer can revoke | `revoke_seat_requires_matching_issuer` |
| Re-issue after offboarding restores access | `reissue_after_offboarding_restores_access` |

## Files

- `src/lib.rs` — gated resource contract, stateful mock TrustLink, unit tests
- `Cargo.toml` — example crate dependencies

## Run Tests

```bash
cd examples/seat-licensing
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
cd examples/seat-licensing
cargo build --target wasm32-unknown-unknown --release
```

WASM artifact:
`target/wasm32-unknown-unknown/release/seat_licensing_example.wasm`

### 2. Deploy TrustLink (if needed)

```bash
export ADMIN_SECRET=SXXX...
cd ../..
make deploy NETWORK=testnet
export TRUSTLINK_ID=C...
```

### 3. Deploy the seat-gated resource

```bash
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/seat_licensing_example.wasm \
  --source $ADMIN_SECRET \
  --network testnet
export SEAT_RESOURCE_ID=C...
```

### 4. Initialize with org issuer

```bash
stellar contract invoke \
  --id $SEAT_RESOURCE_ID \
  --source $ADMIN_SECRET \
  --network testnet \
  -- initialize \
  --admin <ADMIN_ADDRESS> \
  --trustlink_contract $TRUSTLINK_ID \
  --org_issuer <ORG_ISSUER_ADDRESS>
```

### 5. Issue seats and invoke gated features

Issue `SEAT_LICENSED` via TrustLink (see above), then:

```bash
stellar contract invoke \
  --id $SEAT_RESOURCE_ID \
  --source $EMPLOYEE_SECRET \
  --network testnet \
  -- access_premium_feature \
  --employee <EMPLOYEE_ADDRESS>
```

## Production Notes

- Prefer `has_valid_claim_from_issuer` over bare `has_valid_claim` so one
  customer's seats cannot unlock another customer's deployment.
- Register the org as a TrustLink issuer (`register_issuer`) before issuing seats.
- Set seat expiration to match the subscription term, or leave open-ended and
  rely on explicit revocation for offboarding.
- Replace panic strings with typed contract errors in production.
- Optionally batch-issue seats with TrustLink `create_attestations_batch` for
  large employee cohorts.
