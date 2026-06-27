# End-to-End Attestation Request Flow Test

## Overview

The end-to-end (E2E) test suite validates the complete attestation request workflow as documented in `docs/integration-guide.md`. This guide explains the test structure, prerequisites, and how to run the tests locally.

## Test Location

```
sdk/typescript/e2e/attestation-request-flow.e2e.test.ts
```

## Workflow Steps

The test exercises the full user journey:

### 1. **Request Phase**
Subject (user) initiates an attestation request:
```typescript
// Subject asks issuer to verify KYC status
await contract.request_attestation(
  issuer_address,
  "KYC_PASSED",
  "Need KYC to trade"
);
```

### 2. **Review Phase**
Issuer retrieves and reviews pending requests:
```typescript
// Issuer checks all pending requests
const pending_requests = await contract.get_issuer_pending_requests(
  issuer_address
);

// Issuer reviews: claim type, reason, subject identity
for (const request of pending_requests) {
  console.log(`Request from ${request.subject} for ${request.claim_type}`);
}
```

### 3. **Fulfillment Phase**
Issuer fulfills the request by creating an attestation:
```typescript
// After manual verification, issuer fulfills
await contract.fulfill_request(
  issuer_address,
  request_id
);
// This internally calls create_attestation
```

### 4. **Verification Phase**
Any contract or dApp can now verify the attestation:
```typescript
// Dapp checks if user has valid KYC
const has_kyc = await contract.has_valid_claim(
  user_address,
  "KYC_PASSED"
);

if (has_kyc) {
  // Allow trading
}
```

## Prerequisites

### System Requirements
- Node.js 18+
- Docker (for Stellar Quickstart)
- Rust toolchain (for contract compilation)

### Setup Steps

1. **Start a local Stellar node:**
   ```bash
   cd /workspaces/TrustLink
   docker compose up -d
   ```

2. **Deploy and initialize the contract:**
   ```bash
   ./scripts/setup_local.sh
   ```
   
   This script:
   - Compiles the WASM contract
   - Deploys it to the local network
   - Initializes with an admin account
   - Writes the contract ID to `.local.contract-id`

3. **Verify setup:**
   ```bash
   # Check that Quickstart is running
   curl http://localhost:8000/soroban/rpc -X POST -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","id":"1","method":"ledger_closest_ledger_sequence"}'
   ```

## Running the Tests

### Run All E2E Tests
```bash
cd sdk/typescript
npm run test:e2e
```

### Run Only the Request Flow Tests
```bash
cd sdk/typescript
npm run test:e2e -- attestation-request-flow.e2e.test.ts
```

### Run a Specific Test
```bash
cd sdk/typescript
npm run test:e2e -- attestation-request-flow.e2e.test.ts -t "Subject can request an attestation"
```

### Run with Verbose Output
```bash
cd sdk/typescript
npm run test:e2e -- attestation-request-flow.e2e.test.ts --verbose
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `CONTRACT_ID` | (from `.local.contract-id`) | Deployed contract address |
| `RPC_URL` | `http://localhost:8000/soroban/rpc` | Stellar RPC endpoint |
| `NETWORK_PASSPHRASE` | `Networks.STANDALONE` | Stellar network name |

Example with custom RPC:
```bash
RPC_URL=https://rpc-futurenet.stellar.org CONTRACT_ID=C... npm run test:e2e
```

## Test Descriptions

### 1. Subject can request an attestation
Verifies that a subject can initiate a request for a specific claim type.

**Outcome**: Request is created and stored; issuer can retrieve it.

### 2. Issuer can retrieve pending requests
Verifies that an issuer can query all requests sent to them.

**Outcome**: Issuer sees pending requests with claim type and reason.

### 3. Issuer can fulfill a request
Verifies that an issuer can fulfill a request, which creates the underlying attestation.

**Outcome**: Attestation is created with matching claim type and subject.

### 4. Verifier can query and validate fulfilled attestation
Verifies that external contracts/dApps can query the fulfilled attestation.

**Outcome**: `has_valid_claim` returns true for the created attestation.

### 5. Complete workflow: request → fulfill → verify
End-to-end integration test that exercises all steps in sequence.

**Outcome**: Full workflow completes without errors; final state is consistent.

### 6. Subject can cancel pending request
Verifies that a subject can withdraw a request before it's fulfilled.

**Outcome**: Request status is marked as canceled; issuer can no longer fulfill it.

### 7. Issuer can reject request with reason
Verifies that an issuer can reject a request with an explanation.

**Outcome**: Request is marked as rejected; subject receives the rejection reason.

## Test Architecture

### Helper Functions

#### `fundAccount(keypair)`
Funds a test account using the Stellar Friendbot (testnet only).

#### `invoke(contractId, method, args, signer)`
Submits a contract invocation:
1. Builds transaction
2. Simulates for resource estimation
3. Signs with the signer keypair
4. Submits to RPC
5. Polls until confirmation

#### `invokeAndGetResult(contractId, method, args, signer)`
Like `invoke`, but returns the contract result value instead of just the hash.

### Account Setup
```typescript
const admin = Keypair.random();   // Admin (initializes contract)
const issuer = Keypair.random();  // Issuer (fulfills requests)
const subject = Keypair.random(); // Subject (makes requests)
```

Each test gets fresh keypairs to isolate state and prevent interference.

## Troubleshooting

### Contract Not Found
```
Error: CONTRACT_ID env var not set and .local.contract-id not found
```

**Solution**: Run `./scripts/setup_local.sh` to deploy and initialize.

### RPC Connection Refused
```
Error: connect ECONNREFUSED 127.0.0.1:8000
```

**Solution**: Ensure Stellar Quickstart is running:
```bash
docker compose up -d
docker ps  # Verify containers are running
```

### Transaction Timeout
```
Error: Transaction ... not confirmed after 30 seconds
```

**Causes**:
- Network is slow
- Stellar Quickstart has issues
- Contract code has bugs

**Solution**: 
- Increase timeout in test (change `30` to `60`)
- Check Quickstart logs: `docker compose logs quickstart`
- Run tests in isolation first

### Simulation Failed
```
Error: Simulation failed for fulfill_request: ...
```

**Causes**:
- Contract method signature mismatch
- Invalid parameter types
- Contract panicked

**Solution**:
- Check method name and argument types
- Review contract code for panics
- Enable contract debug logs

## Extending the Tests

### Adding a New Test
```typescript
test("New test name", async () => {
  // Setup accounts, fund them
  const issuer = Keypair.random();
  await fundAccount(issuer);

  // Invoke contract
  const txHash = await invoke(
    contractId,
    "method_name",
    [nativeToScVal(arg1), nativeToScVal(arg2)],
    issuer
  );

  // Assert results
  expect(txHash).toBeTruthy();
}, 60000); // 60s timeout
```

### Querying Contract State
After operations, query state to verify:
```typescript
// Get attestation directly from RPC
const attestation = await server.getContractData(
  contractId,
  nativeToScVal(
    StorageKey.Attestation("att_id_123")
  )
);

// Parse and assert
expect(attestation.subject).toBe(subject.publicKey());
```

### Measuring Performance
```typescript
const start = Date.now();
await invoke(contractId, "method", args, signer);
const elapsed = Date.now() - start;
console.log(`Method took ${elapsed}ms`);
```

## CI Integration

Add to `.github/workflows/ci.yml`:

```yaml
- name: E2E Tests
  run: |
    docker compose up -d
    ./scripts/setup_local.sh
    cd sdk/typescript
    npm run test:e2e
  timeout-minutes: 15
```

## References

- [Integration Guide](./integration-guide.md) – Full SDK walkthrough
- [TypeScript SDK README](../sdk/typescript/README.md) – SDK documentation
- [Stellar Soroban Docs](https://developers.stellar.org/docs/learn/smart-contracts)
- [Jest Testing Guide](https://jestjs.io/docs/getting-started)
