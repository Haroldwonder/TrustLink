# TrustLink TypeScript Bindings (`@trustlink/contract`)

> **Not sure which TypeScript package to use?**
> See [docs/integration-guide.md — Which TypeScript package do I need?](../../docs/integration-guide.md#which-typescript-package-do-i-need) for a side-by-side comparison.
>
> **TL;DR** — most application developers should use [`@trustlink/sdk`](../../sdk/typescript/README.md) instead.

Low-level TypeScript bindings for the TrustLink Soroban smart contract.

This package exposes **`TrustLinkClient`** — a thin, typed wrapper that maps
directly to every contract entry point. It handles transaction building and
simulation but leaves signing and submission to you, making it suitable for
both browser wallets and server-side signers.

> **New to TrustLink?** If you just want to query attestations or verify claims,
> consider [`@trustlink/sdk`](../../sdk/typescript/README.md) instead — it adds
> automatic retry/circuit-breaker, pagination helpers, and a higher-level API
> on top of these bindings.
>
> See [Choosing a Package](#choosing-a-package) below for a side-by-side
> comparison.

---

## Installation

```bash
npm install @trustlink/bindings
```

Or link the local package during development:

```bash
npm install ../bindings/typescript
```

**Peer dependency:** `@stellar/stellar-sdk` ≥ 12.0.0

---

## Quick Start

```typescript
import { TrustLinkClient } from "@trustlink/bindings";
import { Keypair } from "@stellar/stellar-sdk";

const client = new TrustLinkClient({
  contractId: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCN8",
  rpcUrl: "https://soroban-testnet.stellar.org",
  // networkPassphrase defaults to Testnet
});

// ── Read: check whether a wallet has a valid KYC attestation ─────────────────
const hasKyc = await client.hasValidClaim(
  "GBRPYHIL2CI3WHZDTOOQFC6EB4CGQOFSNHERX3UNFOK2MAGNTQEFUPROTOCOL",
  "KYC_PASSED"
);
console.log("User has valid KYC:", hasKyc);

// ── Write: create an attestation (requires a signer Keypair) ─────────────────
const issuer = Keypair.fromSecret("S...");
const attestationId = await client.createAttestation(
  issuer,                         // Keypair that signs the transaction
  "GBRPYHIL...",                  // subject address
  "KYC_PASSED",                   // claim type
  undefined,                      // optional expiration (Unix timestamp as bigint)
  JSON.stringify({ level: "full" }) // optional metadata
);
console.log("Created attestation:", attestationId);
```

---

## Constructor

```typescript
new TrustLinkClient(options: TrustLinkClientOptions)
```

| Option               | Type     | Required | Description                                              |
|----------------------|----------|----------|----------------------------------------------------------|
| `contractId`         | `string` | Yes      | Deployed contract address (`C…`)                        |
| `rpcUrl`             | `string` | Yes      | Stellar Soroban RPC endpoint                            |
| `networkPassphrase`  | `string` | No       | Network passphrase. Defaults to `Networks.TESTNET`      |

---

## API Reference

All method names are **camelCase** and arguments are positional (not object
options). Return types are inferred from the contract ABI — see
[Types](#types) for the full list.

### Claim Verification (read-only)

```typescript
// Returns true if the subject holds at least one valid attestation of this
// claim type from any registered issuer.
await client.hasValidClaim(subject: string, claimType: string): Promise<boolean>

// Returns true only when the attestation was issued by the specific issuer.
await client.hasValidClaimFromIssuer(
  subject: string,
  claimType: string,
  issuer: string
): Promise<boolean>

// OR-logic: true if the subject holds ANY of the listed claim types.
await client.hasAnyClaim(subject: string, claimTypes: string[]): Promise<boolean>

// AND-logic: true only if the subject holds ALL of the listed claim types.
await client.hasAllClaims(subject: string, claimTypes: string[]): Promise<boolean>

// True if the subject holds a valid claim from an issuer at or above minTier.
await client.hasValidClaimFromTier(
  subject: string,
  claimType: string,
  minTier: IssuerTier
): Promise<boolean>
```

Example:

```typescript
const isVerified = await client.hasValidClaim(
  "GBRPYHIL2CI3WHZDTOOQFC6EB4CGQOFSNHERX3UNFOK2MAGNTQEFUPROTOCOL",
  "KYC_PASSED"
);

const passesAll = await client.hasAllClaims(
  "GBRPYHIL...",
  ["KYC_PASSED", "AML_CLEARED"]
);
```

### Attestation Queries (read-only)

```typescript
// Fetch a single attestation record by ID.
await client.getAttestation(attestationId: string): Promise<Attestation>

// Live status: "Valid" | "Expired" | "Revoked" | "Pending"
await client.getAttestationStatus(attestationId: string): Promise<AttestationStatus>

// Most recent valid attestation for subject + claim type.
await client.getAttestationByType(subject: string, claimType: string): Promise<Attestation>

// Paginated list of attestation IDs for a subject.
await client.getSubjectAttestations(subject: string, start: number, limit: number): Promise<string[]>

// Paginated list of attestation IDs created by an issuer.
await client.getIssuerAttestations(issuer: string, start: number, limit: number): Promise<string[]>

// Distinct claim types for which the subject holds a valid attestation.
await client.getValidClaims(subject: string): Promise<string[]>

// Full append-only audit trail for an attestation.
await client.getAuditLog(attestationId: string): Promise<AuditEntry[]>

// All attestations carrying a specific tag for a subject.
await client.getAttestationsByTag(subject: string, tag: string): Promise<string[]>
```

Example:

```typescript
const attestation = await client.getAttestation("att_abc123...");
console.log(attestation.claim_type, attestation.revoked, attestation.expiration);

const page = await client.getSubjectAttestations("GBRPYHIL...", 0, 10);
for (const id of page) {
  const status = await client.getAttestationStatus(id);
  console.log(id, status);
}
```

### Write Operations

Write methods require a `Keypair` signer and return the **transaction hash**
on success.

```typescript
// Create a new attestation. Returns the attestation ID.
await client.createAttestation(
  issuer: Keypair,
  subject: string,
  claimType: string,
  expiration?: bigint,   // Unix timestamp; omit or pass undefined for no expiry
  metadata?: string,     // arbitrary JSON/string; max 256 bytes
  tags?: string[]        // optional searchable tags
): Promise<string>

// Revoke an existing attestation.
await client.revokeAttestation(
  issuer: Keypair,
  attestationId: string,
  reason?: string        // optional human-readable reason; max 128 chars
): Promise<string>

// Renew (extend or clear) an expiration.
await client.renewAttestation(
  issuer: Keypair,
  attestationId: string,
  newExpiration?: bigint // pass undefined to remove the expiry
): Promise<string>

// Batch-create attestations for multiple subjects at once.
await client.createAttestationsBatch(
  issuer: Keypair,
  subjects: string[],
  claimType: string,
  expiration?: bigint
): Promise<string[]>    // returns IDs in subject order

// Batch-revoke multiple attestations atomically.
await client.revokeAttestationsBatch(
  issuer: Keypair,
  attestationIds: string[],
  reason?: string
): Promise<number>      // returns count revoked

// Import a historical attestation (admin only).
await client.importAttestation(
  admin: Keypair,
  issuer: string,
  subject: string,
  claimType: string,
  timestamp: bigint,
  expiration?: bigint
): Promise<string>

// Bridge an attestation from another chain.
await client.bridgeAttestation(
  bridge: Keypair,
  subject: string,
  claimType: string,
  sourceChain: string,
  sourceTx: string
): Promise<string>
```

Example:

```typescript
import { Keypair } from "@stellar/stellar-sdk";

const issuer = Keypair.fromSecret("SXXXXXXXX...");

// Create an attestation that expires in one year
const oneYearFromNow = BigInt(Math.floor(Date.now() / 1000) + 365 * 24 * 60 * 60);
const id = await client.createAttestation(
  issuer,
  "GBRPYHIL2CI3...",
  "KYC_PASSED",
  oneYearFromNow,
  JSON.stringify({ provider: "acme-kyc", level: "enhanced" })
);
console.log("Attestation ID:", id);

// Revoke with a reason
await client.revokeAttestation(issuer, id, "Customer account closed");
```

### Admin Operations

```typescript
await client.initialize(admin: Keypair, ttlDays?: number): Promise<string>
await client.transferAdmin(currentAdmin: Keypair, newAdmin: string): Promise<string>
await client.registerIssuer(admin: Keypair, issuer: string): Promise<string>
await client.removeIssuer(admin: Keypair, issuer: string): Promise<string>
await client.registerBridge(admin: Keypair, bridgeContract: string): Promise<string>
await client.registerClaimType(admin: Keypair, claimType: string, description: string): Promise<string>
await client.pause(admin: Keypair): Promise<string>
await client.unpause(admin: Keypair): Promise<string>
await client.setFee(admin: Keypair, fee: bigint, collector: string, feeToken?: string): Promise<string>
```

### Multi-Sig Proposals

```typescript
// Propose a multi-sig attestation; proposer auto-signs. Returns proposal ID.
await client.proposeAttestation(
  proposer: Keypair,
  subject: string,
  claimType: string,
  requiredSigners: string[],
  threshold: number
): Promise<string>

// Co-sign an open proposal.
await client.cosignAttestation(issuer: Keypair, proposalId: string): Promise<string>

// Cancel an unfinalized proposal (proposer only).
await client.cancelMultisigProposal(proposer: Keypair, proposalId: string): Promise<string>

// Inspect a proposal.
await client.getMultisigProposal(proposalId: string): Promise<MultiSigProposal>
```

### Issuer & Registry Queries

```typescript
await client.isIssuer(address: string): Promise<boolean>
await client.isBridge(address: string): Promise<boolean>
await client.getAdmin(): Promise<string>
await client.getVersion(): Promise<string>
await client.getConfig(): Promise<ContractConfig>
await client.getFeeConfig(): Promise<FeeConfig>
await client.getGlobalStats(): Promise<GlobalStats>
await client.isPaused(): Promise<boolean>
await client.getIssuerStats(issuer: string): Promise<IssuerStats>
await client.getIssuerMetadata(issuer: string): Promise<IssuerMetadata | null>
await client.getClaimTypeDescription(claimType: string): Promise<string | null>
await client.listClaimTypes(start: number, limit: number): Promise<string[]>
```

### Endorsements

```typescript
await client.endorseAttestation(endorser: Keypair, attestationId: string): Promise<string>
await client.getEndorsements(attestationId: string): Promise<Endorsement[]>
await client.getEndorsementCount(attestationId: string): Promise<number>
```

---

## Types

All contract types are exported from the package root:

```typescript
import type {
  Attestation,
  AttestationStatus,
  AuditEntry,
  ClaimTypeInfo,
  ContractConfig,
  ContractMetadata,
  Delegation,
  Endorsement,
  FeeConfig,
  GlobalStats,
  HealthStatus,
  IssuerMetadata,
  IssuerStats,
  IssuerTier,
  MultiSigProposal,
  TtlConfig,
} from "@trustlink/bindings";

import { TrustLinkClient } from "@trustlink/bindings";
```

Key type shapes:

```typescript
interface Attestation {
  id: string;
  issuer: string;
  subject: string;
  claim_type: string;
  timestamp: bigint;
  expiration: bigint | null;
  revoked: boolean;
  metadata: string | null;
  imported: boolean;
  bridged: boolean;
  source_chain: string | null;
  source_tx: string | null;
}

type AttestationStatus = "Valid" | "Expired" | "Revoked" | "Pending";

interface MultiSigProposal {
  id: string;
  subject: string;
  claim_type: string;
  proposer: string;
  signers: string[];
  threshold: number;
  finalized: boolean;
  expires_at: bigint;
}
```

---

## Error Handling

Every contract error is translated to an `Error` with a descriptive message:

```typescript
try {
  const att = await client.getAttestation("nonexistent-id");
} catch (err) {
  // err.message will be "NotFound" or "ContractError(4)" if unmapped
  console.error(err.message);
}
```

Contract error codes and their names are also exported:

```typescript
import { CONTRACT_ERRORS } from "@trustlink/bindings";
// CONTRACT_ERRORS[4] === "NotFound"
```

---

## Choosing a Package

| | `@trustlink/bindings` | `@trustlink/sdk` |
|---|---|---|
| **npm package** | `@trustlink/bindings` | `@trustlink/sdk` |
| **Class name** | `TrustLinkClient` | `TrustLinkClient` |
| **Purpose** | Thin contract wrapper; every entry point, no extra logic | Full-featured client for app developers |
| **Retry / backoff** | ✗ | ✓ (exponential backoff on reads) |
| **Circuit breaker** | ✗ | ✓ |
| **Pagination helpers** | ✗ | ✓ `iterateSubjectAttestations` / `iterateIssuerAttestations` |
| **Write operations** | ✓ (requires `Keypair`) | Simulation-only (returns raw tx for external signing) |
| **Best for** | Server-side signers, scripts, contract testing | Read-heavy frontends, dApp integrations |

**Rule of thumb:**
- Building a **frontend / dApp** that mainly reads data → use `@trustlink/sdk`.
- Building a **backend service** that creates or revokes attestations → use
  `@trustlink/bindings` (or use the SDK and call `invoke` directly for writes).
- Unsure? Start with `@trustlink/sdk`; drop down to `@trustlink/bindings` if
  you need direct control over signing or access to an entry point not yet
  surfaced by the SDK.

See [docs/integration-guide.md](../../docs/integration-guide.md) for a full
walkthrough of both packages.

---

## Regenerating Bindings

Bindings are generated from the contract ABI. After changing `src/lib.rs`, run:

```bash
make bindings
# or directly:
stellar contract bindings typescript \
  --output-dir bindings/typescript/src \
  --wasm target/wasm32-unknown-unknown/release/trustlink.wasm
```

Always commit updated bindings alongside contract changes.

---

## Development

```bash
npm install
npm run build
npm test
```

---

## License

MIT
