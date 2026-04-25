# TrustLink TypeScript Bindings

Auto-generated TypeScript bindings for the TrustLink smart contract.

## Overview

These bindings are automatically generated from the TrustLink contract ABI using the Stellar CLI. They provide full type safety and IDE autocomplete for interacting with the contract from TypeScript/JavaScript.

## Installation

```bash
npm install @trustlink/bindings
```

Or use the local bindings during development:

```bash
npm install ../bindings/typescript
```

## Quick Start

```typescript
import { Client } from "@trustlink/bindings";

const client = new Client({
  rpcUrl: "https://soroban-testnet.stellar.org",
  contractId: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCN8",
});

// Verify a claim
const hasKyc = await client.has_valid_claim({
  subject: "GBRPYHIL2CI3WHZDTOOQFC6EB4CGQOFSNHERX3UNFOK2MAGNTQEFUPROTOCOL",
  claim_type: "KYC_PASSED",
});

console.log("User has valid KYC:", hasKyc);
```

## API Reference

### Client

Main class for interacting with the TrustLink contract.

#### Constructor

```typescript
new Client(options: ClientOptions)
```

Options:
- `rpcUrl` (string): Stellar RPC endpoint
- `contractId` (string): TrustLink contract address
- `networkPassphrase` (string, optional): Network passphrase (default: testnet)

#### Methods

All contract methods are available on the client. See `types.ts` for full type definitions.

**Example: Create Attestation**

```typescript
const attestationId = await client.create_attestation({
  issuer: "GCZST3XVCDTUJ76ZAV2HA72KYQJM3O5OF7MANXVZUOTSBZUJUJVOY7XL",
  subject: "GBRPYHIL2CI3WHZDTOOQFC6EB4CGQOFSNHERX3UNFOK2MAGNTQEFUPROTOCOL",
  claim_type: "KYC_PASSED",
  expiration: Math.floor(Date.now() / 1000) + 365 * 24 * 60 * 60,
  metadata: JSON.stringify({ level: "basic" }),
});
```

**Example: Verify Claim**

```typescript
const isValid = await client.has_valid_claim({
  subject: "GBRPYHIL...",
  claim_type: "KYC_PASSED",
});

if (isValid) {
  console.log("Claim is valid");
}
```

**Example: List Attestations**

```typescript
const attestations = await client.get_subject_attestations({
  subject: "GBRPYHIL...",
  start: 0,
  limit: 10,
});

attestations.forEach(att => {
  console.log(`${att.claim_type}: ${att.status}`);
});
```

## Types

All contract types are exported from `types.ts`:

```typescript
import {
  Attestation,
  ContractConfig,
  GlobalStats,
  AttestationStatus,
  Error,
  // ... more types
} from "@trustlink/bindings";
```

### Common Types

**Attestation**
```typescript
interface Attestation {
  id: string;
  issuer: string;
  subject: string;
  claim_type: string;
  timestamp: u64;
  expiration: Option<u64>;
  revoked: boolean;
  metadata: Option<string>;
  imported: boolean;
  bridged: boolean;
  source_chain: Option<string>;
  source_tx: Option<string>;
}
```

**AttestationStatus**
```typescript
type AttestationStatus = "Valid" | "Expired" | "Revoked";
```

**ContractConfig**
```typescript
interface ContractConfig {
  admin: string;
  paused: boolean;
  attestation_fee: u64;
  fee_collector: Option<string>;
  fee_token: Option<string>;
  max_attestations_per_issuer: u64;
  max_attestations_per_subject: u64;
}
```

## Error Handling

Contract errors are returned as `Result` types:

```typescript
const result = await client.create_attestation({...});

if (result.isOk()) {
  const attestationId = result.value;
  console.log("Created:", attestationId);
} else {
  const error = result.error;
  console.error("Failed:", error);
}
```

## Regenerating Bindings

Bindings are automatically generated from the contract ABI. To regenerate after contract changes:

```bash
make bindings
```

This will:
1. Build the contract WASM
2. Generate new TypeScript bindings
3. Update `bindings/typescript/src/`

**Important**: Always commit updated bindings with contract changes.

## Development

### Building

```bash
npm install
npm run build
```

### Testing

```bash
npm test
```

### Publishing

```bash
npm publish
```

## File Structure

```
bindings/typescript/
├── src/
│   ├── client.ts      # Generated contract client
│   ├── types.ts       # Generated type definitions
│   └── index.ts       # Exports
├── package.json       # NPM package metadata
├── tsconfig.json      # TypeScript configuration
└── README.md          # This file
```

## Documentation

For more information:
- [Bindings Generation Guide](../../docs/bindings-generation.md)
- [Integration Guide](../../docs/integration-guide.md)
- [Contract Documentation](../../README.md)

## Support

For issues or questions:
1. Check the [integration guide](../../docs/integration-guide.md)
2. Review [contract documentation](../../README.md)
3. Open an issue on GitHub

## License

MIT
