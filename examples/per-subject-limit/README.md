# Per-Subject Attestation Limit

This directory contains examples demonstrating how to use TrustLink's per-subject attestation limit feature.

## Overview

The per-subject attestation limit is an optional, configurable constraint that prevents unbounded growth of attestations for a single subject. This addresses performance degradation concerns where query performance deteriorates as a subject accumulates more and more attestations.

**Key Characteristics:**
- **Optional**: Completely disabled by default for backward compatibility
- **Configurable**: Admins can set, update, or disable the limit at runtime
- **Clear Errors**: When exceeded, attempts to create attestations are rejected with explicit error messages
- **Applied Everywhere**: Limits are enforced in `create_attestation`, `import_attestation`, `bridge_attestation`, and bundle creation

## Problem Solved

Without limits, a single subject could accumulate unlimited attestations, causing:
1. **Query Performance Degradation**: Listing all attestations for a subject becomes progressively slower
2. **Unbounded Growth Vector**: Similar to unbounded pending-request and proposal-index issues, this is an attack surface
3. **Storage Bloat**: Subject indices become very large, consuming more storage and computation

With the per-subject limit:
- Deployments can cap attestation accumulation per subject
- Query performance remains predictable
- Existing deployments are unaffected (limit is `None` by default)

## Use Cases

### 1. Identity Verification System
Cap the number of identity attestations per subject to prevent replay attacks:

```
Subject: alice@example.com
Limit: 100 attestations per subject
├─ KYC_PASSED (1/100)
├─ AGE_VERIFIED (2/100)
├─ DOCUMENT_VERIFIED (3/100)
└─ ... up to 100
→ 101st attestation → REJECTED (limit exceeded)
```

### 2. Compliance Tracking
Limit how many compliance attestations accumulate for audit purposes:

```
Subject: company-xyz
Limit: 50 audit records per subject
├─ SOC2_2023 (1/50)
├─ SOC2_2024 (2/50)
├─ ISO27001_2023 (3/50)
├─ ISO27001_2024 (4/50)
└─ ... up to 50
→ 51st attestation → REJECTED
```

### 3. Healthcare Records
Prevent uncontrolled accumulation of medical records:

```
Subject: patient-id-12345
Limit: 1000 records per patient
├─ DIAGNOSIS_2023_01 (1/1000)
├─ DIAGNOSIS_2023_06 (2/1000)
├─ PRESCRIPTION_2023_03 (3/1000)
└─ ... up to 1000
→ 1001st record → REJECTED
```

## Configuration

### Setting the Limit

```typescript
// TypeScript
import { TrustLinkClient } from "@trustlink/contract";

const client = new TrustLinkClient({
  contractId: "C...",
  rpcUrl: "https://soroban-testnet.stellar.org",
});

// Set limit to 100 attestations per subject
const result = await client.invoke({
  method: "set_max_attestations_per_subject",
  args: {
    admin: adminAddress,
    limit: 100,
  },
  auth: [adminAddress],
});

console.log("Limit set to 100 attestations per subject");
```

```python
# Python
from trustlink import TrustLinkClient

client = TrustLinkClient(
    contract_id="C...",
    rpc_url="https://soroban-testnet.stellar.org",
)

# Set limit to 100 attestations per subject
client.set_max_attestations_per_subject(
    admin=admin_address,
    limit=100,
)

print("Limit set to 100 attestations per subject")
```

### Getting the Current Limit

```typescript
// TypeScript
const limit = await client.invoke({
  method: "get_max_attestations_per_subject",
});

if (limit) {
  console.log(`Per-subject limit: ${limit} attestations`);
} else {
  console.log("No per-subject limit (unlimited)");
}
```

```python
# Python
limit = client.get_max_attestations_per_subject()

if limit:
    print(f"Per-subject limit: {limit} attestations")
else:
    print("No per-subject limit (unlimited)")
```

### Disabling the Limit

```typescript
// TypeScript
// Pass `None` / `null` to remove the limit
await client.invoke({
  method: "set_max_attestations_per_subject",
  args: {
    admin: adminAddress,
    limit: null,
  },
  auth: [adminAddress],
});

console.log("Per-subject limit removed (unlimited)");
```

```python
# Python
# Pass `None` to remove the limit
client.set_max_attestations_per_subject(
    admin=admin_address,
    limit=None,
)

print("Per-subject limit removed (unlimited)")
```

## Error Handling

When a subject reaches the limit, attempting to create new attestations fails:

```typescript
// TypeScript
try {
  await client.invoke({
    method: "create_attestation",
    args: {
      issuer: issuerAddress,
      subject: subjectAddress,
      claimType: "NEW_CLAIM",
      expiration: null,
      metadata: null,
      tags: null,
    },
    auth: [issuerAddress],
  });
} catch (error) {
  if (error.message.includes("LimitExceeded")) {
    console.error(`Cannot create attestation: subject has reached maximum of 100 attestations`);
    // Consider:
    // - Informing the user
    // - Revoke old attestations if policy allows
    // - Request admin increase the limit
  }
  throw error;
}
```

```python
# Python
try:
    client.create_attestation(
        issuer=issuer_address,
        subject=subject_address,
        claim_type="NEW_CLAIM",
        expiration=None,
        metadata=None,
        tags=None,
    )
except Exception as error:
    if "LimitExceeded" in str(error):
        print(f"Cannot create attestation: subject has reached maximum attestations")
        # Consider:
        # - Informing the user
        # - Revoke old attestations if policy allows
        # - Request admin increase the limit
    raise
```

## Query Performance Considerations

The per-subject limit improves query performance by:

1. **Bounded Result Sets**: `get_subject_attestations` always returns ≤ limit results
2. **Predictable Latency**: Query time is independent of overall subject age/history
3. **Better Cache Behavior**: Smaller indices fit better in CPU caches

Example performance characteristics:

```
Without limit:
- Subject with 1,000 attestations: ~500ms query time
- Subject with 100,000 attestations: ~50s query time (degradation!)

With limit (e.g., 1,000):
- Subject with any number of total attestations: ~500ms query time (consistent!)
```

## Migration Path

### For New Deployments
Set an appropriate limit during initialization:

```typescript
// Initialize with limit
await client.invoke({
  method: "initialize",
  args: {
    admin: adminAddress,
    ttlDays: 30,
  },
});

// Immediately set the limit
await client.invoke({
  method: "set_max_attestations_per_subject",
  args: {
    admin: adminAddress,
    limit: 10000, // Reasonable default for most use cases
  },
  auth: [adminAddress],
});
```

### For Existing Deployments
Existing deployments continue to work unchanged:
- Limit is `None` (unlimited) by default
- No breaking changes
- Admins can opt-in to the limit at any time
- Can be adjusted or disabled without affecting existing attestations

## Examples

See the accompanying files for complete working examples:
- `typescript-example.ts` - Full TypeScript implementation
- `python-example.py` - Full Python implementation

Both examples demonstrate:
1. Setting the per-subject limit
2. Creating attestations up to the limit
3. Handling limit-exceeded errors
4. Querying the current limit
5. Disabling the limit
6. Creating bundles with per-subject limit enforcement
