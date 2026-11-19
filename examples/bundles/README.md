# TrustLink Attestation Bundles

This directory contains examples demonstrating how to use TrustLink's attestation bundling feature.

## Overview

Attestation bundles allow issuing multiple related claims atomically as a single transaction. All attestations in a bundle share a common bundle ID, enabling verifiers to confirm that a set of claims were issued together as one coherent unit.

**Key Benefits:**
- **Atomicity**: All claims in a bundle succeed or fail together (no partial states)
- **Correlation**: Verifiers can confirm claims were issued as a cohesive set
- **Efficiency**: Reduced transaction overhead for issuing related claims
- **Auditability**: Audit logs track bundle membership for each attestation

## Use Cases

### 1. KYC Bundle
A KYC (Know Your Customer) check that simultaneously establishes multiple verified claims:

```
Bundle ID: abc123...
├─ KYC_PASSED
├─ AGE_VERIFIED
├─ JURISDICTION_US
└─ IDENTITY_VERIFIED
```

Instead of 4 separate transactions with no guarantee all succeed, all 4 attestations are created atomically.

### 2. Compliance Bundle
Regulatory compliance certifications issued together:

```
Bundle ID: def456...
├─ PCI_DSS_COMPLIANT
├─ SOC_2_CERTIFIED
├─ ISO_27001_APPROVED
└─ GDPR_COMPLIANT
```

### 3. Academic Bundle
University credentials issued in one transaction:

```
Bundle ID: ghi789...
├─ DEGREE_BACHELOR
├─ MAJOR_COMPUTER_SCIENCE
├─ GPA_3_8
└─ GRADUATION_DATE_2024
```

## How Bundles Work

### Creating a Bundle

```typescript
// TypeScript
import { TrustLinkClient, createAttestationBundle } from "@trustlink/contract";

const client = new TrustLinkClient({
  contractId: "C...",
  rpcUrl: "https://soroban-testnet.stellar.org",
});

const bundleId = await createAttestationBundle(client, {
  issuer: issuerAddress,
  subject: subjectAddress,
  claimTypes: ["KYC_PASSED", "AGE_VERIFIED", "JURISDICTION_US"],
  expiration: BigInt(Math.floor(Date.now() / 1000)) + BigInt(365 * 24 * 60 * 60),
  metadata: '{"kyc_level":"enhanced","checked_at":"2024-01-15"}',
});

console.log(`Bundle created: ${bundleId}`);
```

```python
# Python
from trustlink import TrustLinkClient, BundleOptions

client = TrustLinkClient(
    contract_id="C...",
    rpc_url="https://soroban-testnet.stellar.org",
)

bundle_options = BundleOptions(
    issuer=issuer_address,
    subject=subject_address,
    claim_types=["KYC_PASSED", "AGE_VERIFIED", "JURISDICTION_US"],
    expiration=int(time.time()) + (365 * 24 * 60 * 60),
    metadata='{"kyc_level":"enhanced","checked_at":"2024-01-15"}',
)

bundle_id = client.create_attestation_bundle(
    bundle_options.issuer,
    bundle_options.subject,
    bundle_options.claim_types,
    bundle_options.expiration,
    bundle_options.metadata,
    bundle_options.tags,
)
```

### Verifying Bundle Membership

Verifiers can confirm that a set of attestations were issued together:

```typescript
// TypeScript
import {
  getBundleWithAttestations,
  verifyBundleValidity,
  verifyAttestationsInSameBundle,
} from "@trustlink/contract";

// Get all attestations in a bundle
const { bundle, attestations } = await getBundleWithAttestations(client, bundleId);

console.log(`Bundle ${bundle.id} contains ${attestations.length} attestations`);
console.log(`All valid: ${bundle.all_valid}`);

// Verify all attestations share the same bundle
const commonBundleId = verifyAttestationsInSameBundle(attestations);
if (commonBundleId === bundleId) {
  console.log("✓ All attestations are from the same bundle");
}

// Check if bundle is still valid (not revoked)
const isValid = await verifyBundleValidity(client, bundleId);
console.log(`Bundle valid: ${isValid}`);
```

```python
# Python
from trustlink import (
    verify_attestations_in_same_bundle,
    verify_bundle_claim_types,
    get_bundle_summary,
)

# Verify all attestations share the same bundle
bundle_id = verify_attestations_in_same_bundle(attestations)
if bundle_id:
    print(f"✓ All attestations are from bundle: {bundle_id}")

# Verify expected claim types
if verify_bundle_claim_types(bundle, ["KYC_PASSED", "AGE_VERIFIED"]):
    print("✓ Bundle contains expected claim types")

# Get bundle summary
summary = get_bundle_summary(bundle)
print(f"Bundle {summary['id']} from {summary['issuer']}")
print(f"  Claims: {', '.join(summary['claim_types'])}")
print(f"  Valid: {summary['all_valid']}")
```

### Querying Bundles

```typescript
// Get all bundles created by an issuer
const issuerBundles = await client.getIssuerBundles(issuerAddress);
console.log(`Issuer created ${issuerBundles.length} bundles`);

// Get all bundles issued to a subject
const subjectBundles = await client.getSubjectBundles(subjectAddress);
console.log(`Subject received ${subjectBundles.length} bundles`);

// Get bundle by ID
const bundle = await client.getBundle(bundleId);
console.log(`Bundle contains: ${bundle.claim_types.join(", ")}`);
```

```python
# Get all bundles created by an issuer
issuer_bundles = client.get_issuer_bundles(issuer_address)
print(f"Issuer created {len(issuer_bundles)} bundles")

# Get all bundles issued to a subject
subject_bundles = client.get_subject_bundles(subject_address)
print(f"Subject received {len(subject_bundles)} bundles")

# Get bundle by ID
bundle = client.get_bundle(bundle_id)
print(f"Bundle contains: {', '.join(bundle['claim_types'])}")
```

## Bundle Properties

Each attestation in a bundle has:
- **bundle_id**: Shared identifier linking all attestations in the bundle
- **timestamp**: When the bundle was created (same for all attestations)
- **claim_type**: Individual claim type for this attestation

The bundle metadata includes:
- **id**: Deterministic bundle ID (SHA256 hash of issuer + subject + claim_types + timestamp)
- **issuer**: Address that created the bundle
- **subject**: Address receiving the attestations
- **claim_types**: List of claim types (in creation order)
- **attestation_ids**: List of individual attestation IDs (matches claim_types order)
- **all_valid**: Flag indicating if all attestations are still valid (updated when any is revoked)
- **timestamp**: Creation timestamp

## Important Considerations

### Bundle ID Determinism

Bundle IDs are generated deterministically from:
- Issuer address
- Subject address
- Ordered list of claim types
- Timestamp

This means:
- Same issuer + subject + claim_types issued at the same timestamp = same bundle ID
- Changing claim order or contents changes the bundle ID
- Changing the timestamp creates a different bundle

### Atomicity Guarantee

- All attestations in a bundle are created in a single Soroban transaction
- Either all succeed or all fail
- No partial bundle states possible
- Rate limiting and storage limits are enforced at the bundle level

### Revocation

- Individual attestations in a bundle can be revoked independently
- When any attestation is revoked, the bundle's `all_valid` flag is updated to `false`
- Verifiers should check both the bundle validity and individual attestation status

### Storage Limits

- Maximum 50 attestations per bundle
- Bundle creation respects per-issuer and per-subject attestation limits
- Each attestation in bundle counts against storage quotas

## SDK Examples

- `typescript-example.ts` - Complete TypeScript examples
- `python-example.py` - Complete Python examples

## Testing Bundles

See the contract test files for comprehensive bundle testing:
- Bundle creation with various claim type combinations
- Verification of bundle membership
- Revocation scenarios
- Edge cases (max bundle size, duplicate attestations, etc.)

## Best Practices

1. **Group Related Claims**: Bundle claims that logically belong together
2. **Verify Metadata**: Always verify bundle claim_types match expectations
3. **Check Validity**: Regularly verify bundle validity, especially before relying on claims
4. **Audit Logs**: Review attestation audit logs to understand bundle membership
5. **Error Handling**: Handle potential errors in bundle creation (validation failures, rate limits)

## Related Documentation

- [EVENT_TOPICS.md](../../docs/EVENT_TOPICS.md) - Event taxonomy including bundle_created event
- [TrustLink Contract](../../src/) - Smart contract source code
- [TypeScript SDK](../../bindings/typescript/) - TypeScript bindings
- [Python SDK](../../bindings/python/) - Python bindings
