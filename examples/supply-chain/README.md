# Supply Chain Provenance Verification Example

This example demonstrates how to use TrustLink attestations to track goods through multiple stages of a supply chain, with verification from independent issuers at each stage.

## Scenario

A coffee importer wants to prove that a shipment of beans has been:
1. **Certified Organic** by an independent certifier
2. **Customs Cleared** by a customs authority
3. **Retailer Verified** by the final retailer before sale

Each issuer attests to a different claim type, all about the same shipment ID. A buyer can then verify the complete chain-of-custody by checking that all three attestations exist and are current.

## Workflow

```
Certifier          Customs           Retailer          Buyer
    │                  │                  │              │
    ├─ Issue ──────────┼─ Issue ──────────┼─ Issue ────┤
    │ CERTIFIED_       │ CUSTOMS_         │ RETAILER_   │
    │ ORGANIC          │ CLEARED          │ VERIFIED    │
    │                  │                  │              │
    └──────────────────────────────────────────────┤ Check
                                                   │ has_all_claims()
                                                   └─> PASS / FAIL
```

## Key Concepts

### 1. Multiple Issuers, Single Subject

Unlike KYC (where one issuer verifies one person's identity), supply chain uses:

```
Subject: SHIPMENT_BATCH_001 (the shipment ID, not a person)
Issuers: 
  - Certifier (e.g., https://certifier-org.com)
  - Customs (e.g., government customs authority)
  - Retailer (e.g., trader company)
```

Each issuer must be registered with TrustLink. The subject is the shipment's identifier (could be a hash, batch number, or even a Stellar address if tracking on-chain).

### 2. Different Claim Types for Each Stage

```rust
const CERTIFIED_ORGANIC: &str = "CERTIFIED_ORGANIC";
const CUSTOMS_CLEARED: &str = "CUSTOMS_CLEARED";
const RETAILER_VERIFIED: &str = "RETAILER_VERIFIED";
```

Each claim type captures a specific verification stage. The claim type is arbitrary — you define what makes sense for your supply chain.

### 3. Verifying the Complete Chain with `has_all_claims`

```rust
let shipment_id = /* batch ID */;
let required_claims = vec![
    "CERTIFIED_ORGANIC",
    "CUSTOMS_CLEARED",
    "RETAILER_VERIFIED",
];

// Returns true only if the shipment has valid attestations for ALL three claims
let verified = trustlink.has_all_claims(&shipment_id, &required_claims);
```

This is AND-logic: if any claim is missing, revoked, or expired, the entire check fails.

## Implementation

### Setup

1. **Register issuers** with TrustLink (admin-only):

```bash
# Register the certifier
soroban contract invoke \
  --id <CONTRACT_ID> \
  -- register_issuer \
  --admin <ADMIN_ADDR> \
  --issuer <CERTIFIER_ADDR>

# Register customs authority
soroban contract invoke \
  --id <CONTRACT_ID> \
  -- register_issuer \
  --admin <ADMIN_ADDR> \
  --issuer <CUSTOMS_ADDR>

# Register retailer
soroban contract invoke \
  --id <CONTRACT_ID> \
  -- register_issuer \
  --admin <ADMIN_ADDR> \
  --issuer <RETAILER_ADDR>
```

2. **Register claim types** (admin-only):

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  -- register_claim_type \
  --admin <ADMIN_ADDR> \
  --claim_type "CERTIFIED_ORGANIC" \
  --description "Product certified as organic"

soroban contract invoke \
  --id <CONTRACT_ID> \
  -- register_claim_type \
  --admin <ADMIN_ADDR> \
  --claim_type "CUSTOMS_CLEARED" \
  --description "Cleared by customs authority"

soroban contract invoke \
  --id <CONTRACT_ID> \
  -- register_claim_type \
  --admin <ADMIN_ADDR> \
  --claim_type "RETAILER_VERIFIED" \
  --description "Final retailer verification complete"
```

3. **Create attestations** at each stage:

```bash
# Certifier issues CERTIFIED_ORGANIC
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source <CERTIFIER_SECRET> \
  -- create_attestation \
  --issuer <CERTIFIER_ADDR> \
  --subject <SHIPMENT_ID> \
  --claim_type "CERTIFIED_ORGANIC" \
  --expiration null

# Customs issues CUSTOMS_CLEARED (expires in 1 year)
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source <CUSTOMS_SECRET> \
  -- create_attestation \
  --issuer <CUSTOMS_ADDR> \
  --subject <SHIPMENT_ID> \
  --claim_type "CUSTOMS_CLEARED" \
  --expiration <FUTURE_TIMESTAMP>

# Retailer issues RETAILER_VERIFIED
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source <RETAILER_SECRET> \
  -- create_attestation \
  --issuer <RETAILER_ADDR> \
  --subject <SHIPMENT_ID> \
  --claim_type "RETAILER_VERIFIED" \
  --expiration <FUTURE_TIMESTAMP>
```

### Verify the Chain

```bash
# Call the example contract to verify all claims
soroban contract invoke \
  --id <SUPPLY_CHAIN_EXAMPLE_CONTRACT> \
  -- verify_supply_chain \
  --shipment_id <SHIPMENT_ID> \
  --trustlink <TRUSTLINK_CONTRACT_ID>
```

Returns `true` if all three attestations are valid; `false` otherwise.

### Get Shipment Status

Optionally check which stages have been completed:

```bash
# Get status bitmask (0x01 = certified, 0x02 = customs, 0x04 = retailer)
soroban contract invoke \
  --id <SUPPLY_CHAIN_EXAMPLE_CONTRACT> \
  -- get_shipment_status \
  --shipment_id <SHIPMENT_ID> \
  --trustlink <TRUSTLINK_CONTRACT_ID>
```

Returns an integer where each bit indicates completion:
- `0x01` — CERTIFIED_ORGANIC
- `0x02` — CUSTOMS_CLEARED
- `0x04` — RETAILER_VERIFIED

## Comparison with Other Examples

| Example | Subject | Issuers | Claims | Logic |
|---------|---------|---------|--------|-------|
| **KYC Token** | Individual | One KYC provider | KYC_PASSED | Single issuer, single claim |
| **Governance** | Voter | Admin | VOTER_ELIGIBLE | Single issuer, single claim |
| **Insurance** | Policyholder | Insurance co. | ACTIVE_POLICY | Single issuer, single claim |
| **Supply Chain** | Shipment | Certifier, Customs, Retailer | Multiple | Multiple issuers, `has_all_claims` |

## Real-World Variants

### Variant 1: Optional vs. Mandatory Stages

Use `has_any_claim` instead for scenarios where a shipment passes verification if it reaches *any one* of multiple possible checkpoints:

```rust
// Accept goods that passed either Certification OR Retailer check (but not necessarily both)
let mut acceptable_paths = soroban_sdk::Vec::new(&env);
acceptable_paths.push_back(String::from_str(&env, "CERTIFIED_ORGANIC"));
acceptable_paths.push_back(String::from_str(&env, "DIRECT_IMPORT_VERIFIED"));

let verified = trustlink.has_any_claim(&shipment_id, &acceptable_paths);
```

### Variant 2: Expiration Tracking

Claims can have expiration dates. Goods that were certified 5 years ago might no longer be considered "current." The attestations automatically fail `has_all_claims` if expired:

```rust
// Customs clearance expires after 2 years (attestation includes expiration)
let expiration = current_timestamp + (2 * 365 * 24 * 60 * 60);
trustlink.create_attestation(
    &customs_issuer,
    &shipment_id,
    "CUSTOMS_CLEARED",
    Some(expiration),
    None
);
```

### Variant 3: Audit Trail with Revocation

If a certification is later found to be invalid, the issuer can revoke it:

```rust
// Revoke a fraudulent certification
trustlink.revoke_attestation(&certifier, &attestation_id);
```

The shipment immediately fails verification (revoked attestations are excluded from `has_all_claims`).

## Testing

The example includes a skeleton test structure. To run full integration tests:

1. Deploy TrustLink contract to testnet
2. Deploy the supply-chain example contract
3. Register test issuers and claim types
4. Run the Rust tests or JavaScript integration tests

## Extension Ideas

- **Timestamp checking**: Record when each stage was completed for traceability
- **Metadata**: Attach certifier notes or batch details as metadata in attestations
- **Multi-sig approvals**: Require multiple issuers to co-sign before a claim is finalized (see ADR-006)
- **Historical imports**: Import pre-existing certifications from external systems as attestations
- **Webhook notifications**: Alert stakeholders when a shipment reaches each checkpoint

## Related Documentation

- [TrustLink README — `has_all_claims` vs `has_any_claim`](../../README.md#verify-all-of-multiple-claims)
- [ADR-006: Multi-Issuer OR Logic](../../docs/adr/ADR-006-multi-issuer-or-logic.md)
- [Storage Layout & Querying](../../docs/storage-layout.md)
