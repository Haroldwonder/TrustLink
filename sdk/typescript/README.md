# @trustlink/sdk

TypeScript SDK for the [TrustLink](https://github.com/afurious/TrustLink) on-chain attestation contract on Stellar.

[![npm version](https://badge.fury.io/js/@trustlink%2Fsdk.svg)](https://badge.fury.io/js/@trustlink%2Fsdk)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Installation

```bash
npm install @trustlink/sdk @stellar/stellar-sdk
```

**Requirements:**
- Node.js 16.0.0 or higher
- @stellar/stellar-sdk 12.0.0 or higher

## Quick Start

```typescript
import { TrustLinkClient } from "@trustlink/sdk";

const client = new TrustLinkClient({
  contractId: "CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
  network: "testnet",
});

// Check if a wallet has a valid KYC attestation
const hasKyc = await client.hasValidClaim(
  "GABC...USER_ADDRESS",
  "KYC_PASSED"
);

if (hasKyc) {
  console.log("User is KYC verified");
}
```

## Networks

| Value       | RPC URL                                    |
|-------------|--------------------------------------------|
| `"testnet"` | `https://soroban-testnet.stellar.org`      |
| `"mainnet"` | Stellar mainnet RPC                        |
| `"local"`   | `http://localhost:8000/soroban/rpc`        |

You can also pass a custom RPC URL:

```typescript
const client = new TrustLinkClient({
  contractId: "C...",
  network: "testnet",
  rpcUrl: "https://my-custom-rpc.example.com",
});
```

## API Reference

### Claim Verification

```typescript
// Check a single claim type
await client.hasValidClaim(subject, "KYC_PASSED");

// Check claim from a specific issuer
await client.hasValidClaimFromIssuer(subject, "KYC_PASSED", issuerAddress);

// OR-logic: returns true if subject holds any of the listed claims
await client.hasAnyClaim(subject, ["KYC_PASSED", "ACCREDITED_INVESTOR"]);

// AND-logic: returns true only if subject holds ALL listed claims
await client.hasAllClaims(subject, ["KYC_PASSED", "AML_CLEARED"]);

// Check claim from an issuer of at least a given tier
await client.hasValidClaimFromTier(subject, "KYC_PASSED", "Verified");
```

### Attestation Queries

```typescript
// Fetch a single attestation by ID
const attestation = await client.getAttestation(attestationId);

// Get status: "Valid" | "Expired" | "Revoked" | "Pending"
const status = await client.getAttestationStatus(attestationId);

// Most recent valid attestation for a subject + claim type
const att = await client.getAttestationByType(subject, "KYC_PASSED");

// Paginated list of attestations for a subject
const page = await client.getSubjectAttestations(subject, 0, 10);

// Paginated list of attestations issued by an issuer
const issued = await client.getIssuerAttestations(issuer, 0, 10);

// All valid claim IDs for a subject
const validClaims = await client.getValidClaims(subject);

// Attestations by tag
const tagged = await client.getAttestationsByTag(subject, "premium");

// Audit log for an attestation
const log = await client.getAuditLog(attestationId);
```

### Count Queries

```typescript
await client.getSubjectAttestationCount(subject); // all (incl. revoked/expired)
await client.getIssuerAttestationCount(issuer);
await client.getValidClaimCount(subject);         // non-revoked, non-expired only
```

### Issuer & Registry

```typescript
await client.isIssuer(address);
await client.getIssuerStats(issuer);       // { total_issued: bigint }
await client.getIssuerTier(issuer);        // "Basic" | "Verified" | "Premium" | null
await client.getIssuerMetadata(issuer);    // { name, url, description } | null
await client.isBridge(address);
```

### Claim Type Registry

```typescript
await client.getClaimTypeDescription("KYC_PASSED");
await client.listClaimTypes(0, 20);
```

### Multi-Sig Proposals

```typescript
const proposal = await client.getMultisigProposal(proposalId);
// proposal.signers, proposal.threshold, proposal.finalized, proposal.expires_at
```

### Endorsements

```typescript
const endorsements = await client.getEndorsements(attestationId);
const count = await client.getEndorsementCount(attestationId);
```

### Contract Info

```typescript
await client.getAdmin();
await client.getVersion();
await client.isPaused();
await client.healthCheck();
await client.getGlobalStats();
await client.getContractMetadata();
await client.getConfig();
await client.getFeeConfig();
```

## TypeScript Types

All contract types are exported from the package:

```typescript
import type {
  Attestation,
  AttestationStatus,
  AuditEntry,
  AuditAction,
  ClaimTypeInfo,
  ContractConfig,
  ContractMetadata,
  Endorsement,
  ExpirationHook,
  FeeConfig,
  GlobalStats,
  HealthStatus,
  IssuerMetadata,
  IssuerStats,
  IssuerTier,
  MultiSigProposal,
  TtlConfig,
} from "@trustlink/sdk";

import { TrustLinkError } from "@trustlink/sdk";
```

The `TrustLinkError` enum maps every contract error code to a named constant:

```typescript
TrustLinkError.Unauthorized    // 3
TrustLinkError.NotFound        // 4
TrustLinkError.AlreadyRevoked  // 6
// ...
```

## Error Handling

```typescript
import { TrustLinkClient, TrustLinkError } from "@trustlink/sdk";

try {
  const attestation = await client.getAttestation("invalid-id");
} catch (error) {
  if (error.code === TrustLinkError.NotFound) {
    console.log("Attestation not found");
  } else {
    console.error("Unexpected error:", error);
  }
}
```

## Examples

### Integration with React

```typescript
import { useState, useEffect } from 'react';
import { TrustLinkClient } from '@trustlink/sdk';

function UserVerificationStatus({ userAddress }: { userAddress: string }) {
  const [hasKyc, setHasKyc] = useState<boolean | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const client = new TrustLinkClient({
      contractId: process.env.REACT_APP_TRUSTLINK_CONTRACT_ID!,
      network: "testnet",
    });

    client.hasValidClaim(userAddress, "KYC_PASSED")
      .then(setHasKyc)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [userAddress]);

  if (loading) return <div>Checking verification status...</div>;
  
  return (
    <div>
      {hasKyc ? (
        <span className="verified">✅ KYC Verified</span>
      ) : (
        <span className="unverified">❌ Not Verified</span>
      )}
    </div>
  );
}
```

### DeFi Integration

```typescript
import { TrustLinkClient } from '@trustlink/sdk';

class LendingProtocol {
  private trustlink: TrustLinkClient;

  constructor(contractId: string) {
    this.trustlink = new TrustLinkClient({
      contractId,
      network: "mainnet",
    });
  }

  async canBorrow(userAddress: string, amount: bigint): Promise<boolean> {
    // Check if user has required credentials
    const hasKyc = await this.trustlink.hasValidClaim(userAddress, "KYC_PASSED");
    
    if (amount > 100_000n) {
      // Large loans require accredited investor status
      const isAccredited = await this.trustlink.hasValidClaim(
        userAddress, 
        "ACCREDITED_INVESTOR"
      );
      return hasKyc && isAccredited;
    }
    
    return hasKyc;
  }
}
```

## Building from Source

```bash
cd sdk/typescript
npm install
npm run build
```

## Publishing to npm

The SDK is automatically published to npm when a new release is created on GitHub. To publish manually:

```bash
cd sdk/typescript
npm run build
npm publish --access public
```

> **Note:** Ensure you have the `NPM_TOKEN` secret configured in your repository settings for automated publishing.

## Contributing

See the main [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.

## License

MIT - see [LICENSE](../../LICENSE) for details.
