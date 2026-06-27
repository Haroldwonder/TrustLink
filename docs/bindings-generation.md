# TypeScript Bindings Auto-Generation

This document explains how TrustLink automatically generates and maintains TypeScript bindings from the contract ABI.

## Overview

TypeScript bindings are automatically generated from the compiled WASM contract using the Stellar CLI. This ensures bindings are always in sync with the contract interface and prevents manual maintenance errors.

## Architecture

### Build Process

```
src/lib.rs (Rust contract)
    ↓
cargo build --target wasm32-unknown-unknown --release
    ↓
target/wasm32-unknown-unknown/release/trustlink.wasm
    ↓
stellar contract bindings typescript
    ↓
bindings/typescript/src/
    ├── client.ts      (Generated contract client)
    ├── types.ts       (Generated type definitions)
    └── index.ts       (Exports)
```

### Makefile Targets

#### `make bindings`

Generates TypeScript bindings from the compiled WASM:

```bash
make bindings
```

This target:
1. Builds the contract in release mode
2. Runs `stellar contract bindings typescript` to generate bindings
3. Outputs to `bindings/typescript/src/`

#### `make check-bindings`

Verifies that committed bindings are up-to-date with the current contract:

```bash
make check-bindings
```

This target:
1. Regenerates bindings
2. Compares with committed versions
3. Fails if any differences are found

Used in CI to prevent stale bindings from being merged.

## CI Integration

### GitHub Actions Workflow

The `.github/workflows/ci.yml` includes a `bindings` job that:

1. Checks out the repository
2. Installs Rust and Stellar CLI
3. Builds the contract WASM
4. Generates TypeScript bindings
5. Fails if bindings are out of date

```yaml
bindings:
  name: TypeScript Bindings
  runs-on: ubuntu-latest
  needs: ci
  steps:
    - name: Checkout repository
      uses: actions/checkout@v4
    
    - name: Install Rust toolchain
      run: rustup show
    
    - name: Install Stellar CLI
      run: cargo install --locked stellar-cli --features opt
    
    - name: Build WASM
      run: cargo build --target wasm32-unknown-unknown --release
    
    - name: Generate TypeScript bindings
      run: make bindings
    
    - name: Fail if bindings are out of date
      run: |
        git diff --exit-code bindings/typescript/ || (
          echo "TypeScript bindings are out of date."
          echo "Run 'make bindings' locally and commit the updated bindings/typescript/ directory."
          exit 1
        )
```

## Workflow

### For Developers

When you modify the contract interface:

1. Make changes to `src/lib.rs`
2. Build and test locally:
   ```bash
   cargo test
   ```
3. Generate updated bindings:
   ```bash
   make bindings
   ```
4. Commit both contract changes and updated bindings:
   ```bash
   git add src/lib.rs bindings/typescript/
   git commit -m "feat(contract): add new function and update bindings"
   ```

### For CI/CD

When a PR is submitted:

1. CI builds the contract
2. CI generates bindings
3. CI compares generated bindings with committed versions
4. If they differ, CI fails with a message directing the developer to run `make bindings`

This prevents:
- Stale bindings from being merged
- Manual binding maintenance errors
- Inconsistencies between contract and bindings

## Generated Files

### `bindings/typescript/src/client.ts`

Contains the `Client` class with methods for all contract functions:

```typescript
export class Client {
  constructor(options: ClientOptions);
  
  // Contract methods
  initialize(options: MethodOptions): Promise<Result<void>>;
  register_issuer(options: MethodOptions): Promise<Result<void>>;
  create_attestation(options: MethodOptions): Promise<Result<string>>;
  has_valid_claim(options: MethodOptions): Promise<Result<boolean>>;
  // ... more methods
}
```

### `bindings/typescript/src/types.ts`

Contains TypeScript type definitions for all contract types:

```typescript
export interface Attestation {
  id: string;
  issuer: string;
  subject: string;
  claim_type: string;
  timestamp: u64;
  expiration: Option<u64>;
  revoked: boolean;
  metadata: Option<string>;
  // ... more fields
}

export interface ContractConfig {
  admin: string;
  paused: boolean;
  // ... more fields
}
```

### `bindings/typescript/src/index.ts`

Exports all types and the client:

```typescript
export * from "./client";
export * from "./types";
```

## Usage

### In TypeScript Projects

```typescript
import { Client, Attestation } from "@trustlink/bindings";

const client = new Client({
  rpcUrl: "https://soroban-testnet.stellar.org",
  contractId: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCN8",
});

// Call contract functions with full type safety
const result = await client.has_valid_claim({
  subject: "GBRPYHIL...",
  claim_type: "KYC_PASSED",
});

if (result.isOk()) {
  console.log("User has valid KYC:", result.value);
}
```

## Troubleshooting

### Bindings are out of date

If CI fails with "TypeScript bindings are out of date":

1. Pull the latest changes
2. Run `make bindings` locally
3. Commit the updated `bindings/typescript/` directory
4. Push the changes

### Bindings don't match contract

If you see type mismatches between bindings and contract:

1. Ensure you're using the latest bindings:
   ```bash
   make bindings
   ```
2. Rebuild your TypeScript project:
   ```bash
   npm install
   npm run build
   ```
3. Check for any recent contract changes you might have missed

### Stellar CLI not found

If `make bindings` fails with "stellar: command not found":

```bash
cargo install --locked stellar-cli --features opt
```

## Best Practices

1. **Always regenerate bindings after contract changes**
   ```bash
   make bindings
   ```

2. **Commit bindings with contract changes**
   ```bash
   git add src/lib.rs bindings/typescript/
   git commit -m "feat(contract): ..."
   ```

3. **Keep bindings in sync with main**
   - Don't manually edit generated files
   - Always use `make bindings` to update

4. **Review binding changes in PRs**
   - Check that generated types match your contract changes
   - Ensure no unexpected changes were introduced

## Future Improvements

Potential enhancements to the bindings system:

- [ ] Publish bindings to npm registry
- [ ] Generate bindings for other languages (Python, Go, Rust)
- [ ] Add binding version compatibility checks
- [ ] Automate binding updates in CI
- [ ] Generate API documentation from bindings
