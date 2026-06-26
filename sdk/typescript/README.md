# @trustlink/sdk

TypeScript client for the [TrustLink](../../README.md) Soroban attestation contract on Stellar.

## Installation

```bash
npm install @trustlink/sdk
```

## Quick start

```ts
import { TrustLinkClient } from "@trustlink/sdk";

const client = new TrustLinkClient({
  rpcUrl: "https://soroban-testnet.stellar.org",
  contractId: "C...",
});
```

## API reference

### Attestation range queries

#### `getAttestationsInRange(start, end)`

Returns all attestations whose `timestamp` falls within the closed interval
`[start, end]` (Unix seconds).

```ts
const attestations = await client.getAttestationsInRange(
  1_700_000_000,
  1_710_000_000
);
```

Maps to the contract's `get_attestations_in_range(start, end)` entry point.

#### `getAttestationsInRangeAfter(cursor, limit)`

Cursor-based pagination — returns up to `limit` attestations created **after**
the attestation identified by `cursor`. Use the last returned ID as the next
`cursor` to page through large result sets without re-fetching.

```ts
let cursor = "";
const pageSize = 20;

while (true) {
  const page = await client.getAttestationsInRangeAfter(cursor, pageSize);
  if (page.length === 0) break;
  // process page…
  cursor = page[page.length - 1].id;
}
```

Maps to the contract's `get_attestations_in_range_after(cursor, limit)` entry
point.

### Admin / council

#### `getCouncil()`

Returns the current admin-council configuration (members list, threshold,
creation timestamp).

```ts
const council = await client.getCouncil();
console.log(council.members, council.threshold);
```

#### `getCouncilProposal(proposalId)`

Returns a single council proposal including approvals and execution status.

```ts
const proposal = await client.getCouncilProposal("abc123");
console.log(proposal.action, proposal.approvals);
```

### Storage limits

#### `getLimits()`

Returns the `StorageLimits` configuration enforced by the contract.

```ts
const limits = await client.getLimits();
console.log(limits.max_attestations_per_subject);
```

```ts
interface StorageLimits {
  max_attestations_per_subject: number;
  max_attestations_per_issuer: number;
  max_tags_per_attestation: number;
  max_tag_length: number;
  max_metadata_length: number;
}
```

### Other methods

| Method | Description |
|--------|-------------|
| `getAttestation(id)` | Fetch a single attestation |
| `getAttestationStatus(id)` | Fetch live `AttestationStatus` |
| `getSubjectAttestations(subject)` | All attestation IDs for a subject |
| `getIssuerAttestations(issuer)` | All attestation IDs from an issuer |
| `getMultisigProposal(id)` | Fetch a multi-sig proposal |
| `getAdmin()` | Contract admin address |
| `getFeeConfig()` | Fee configuration |
| `getIssuerMetadata(issuer)` | Issuer name / URL / description |
| `getContractMetadata()` | Contract name / version / description |
| `getVersion()` | Contract version string |

## Types

All TypeScript types are exported from the package root:

```ts
import type {
  Attestation,
  AttestationStatus,
  Council,
  CouncilProposal,
  StorageLimits,
  MultiSigProposal,
  FeeConfig,
} from "@trustlink/sdk";
```
