# TrustLink Integration Guide

This guide walks through integrating TrustLink into your dApp — whether you're building a Rust smart contract that needs on-chain claim verification, or a JavaScript/TypeScript frontend that interacts with the contract directly.

## Testnet Contract

A deployed TrustLink instance is available on Stellar Testnet for immediate testing:

```
Contract ID: CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCN8
Network Passphrase: Test SDF Network ; September 2015
RPC URL: https://soroban-testnet.stellar.org
```

---

## 1. Adding TrustLink as a Dependency (Rust)

In your contract's `Cargo.toml`, add TrustLink as a dependency. You can reference it from a Git source or a local path during development.

```toml
[dependencies]
soroban-sdk = "21.0.0"

# From Git (recommended for production)
trustlink = { git = "https://github.com/your-org/trustlink", tag = "v0.1.0" }

# Or from a local path during development
# trustlink = { path = "../trustlink" }
```

Make sure your `lib` section produces a `cdylib`:

```toml
[lib]
crate-type = ["cdylib", "rlib"]
```

---

## 2. Rust Cross-Contract Integration

### Basic Claim Verification

The most common pattern is verifying a claim before executing a privileged operation.

```rust
#![no_std]

use soroban_sdk::{contract, contractimpl, contractclient, Address, Env, String};

// Import the TrustLink client generated from its contract interface
mod trustlink {
    soroban_sdk::contractimport!(
        file = "../trustlink/target/wasm32-unknown-unknown/release/trustlink.wasm"
    );
}

#[contract]
pub struct LendingContract;

#[contractimpl]
impl LendingContract {
    /// Borrow funds — requires a valid KYC attestation.
    pub fn borrow(
        env: Env,
        borrower: Address,
        trustlink_id: Address,
        amount: i128,
    ) -> Result<(), Error> {
        borrower.require_auth();

        let trustlink = trustlink::Client::new(&env, &trustlink_id);
        let claim = String::from_str(&env, "KYC_PASSED");

        if !trustlink.has_valid_claim(&borrower, &claim) {
            return Err(Error::KYCRequired);
        }

        // ... lending logic
        Ok(())
    }
}

#[contracterror]
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum Error {
    KYCRequired = 1,
}
```

### Checking Attestation Status

When you need more detail than a boolean — for example to distinguish expired from revoked:

```rust
use trustlink::AttestationStatus;

pub fn check_investor_status(
    env: Env,
    user: Address,
    trustlink_id: Address,
    attestation_id: String,
) -> Result<(), Error> {
    let trustlink = trustlink::Client::new(&env, &trustlink_id);

    match trustlink.get_attestation_status(&attestation_id) {
        Ok(AttestationStatus::Valid) => Ok(()),
        Ok(AttestationStatus::Expired) => Err(Error::AttestationExpired),
        Ok(AttestationStatus::Revoked) => Err(Error::AttestationRevoked),
        Err(_) => Err(Error::AttestationNotFound),
    }
}
```

### Paginated Attestation Listing

```rust
pub fn list_user_attestations(
    env: Env,
    subject: Address,
    trustlink_id: Address,
) {
    let trustlink = trustlink::Client::new(&env, &trustlink_id);

    // Fetch first page of 10
    let page = trustlink.get_subject_attestations(&subject, &0, &10);

    for id in page.iter() {
        if let Ok(attestation) = trustlink.get_attestation(&id) {
            // process attestation
            let _ = attestation.claim_type;
            let _ = attestation.expiration;
        }
    }
}
```

### Error Handling

TrustLink errors map to `u32` codes. Handle them explicitly to give users clear feedback:

```rust
use trustlink::Error as TrustLinkError;

pub fn safe_verify(
    env: Env,
    trustlink_id: Address,
    attestation_id: String,
) -> Result<(), MyError> {
    let trustlink = trustlink::Client::new(&env, &trustlink_id);

    trustlink.get_attestation(&attestation_id).map_err(|e| match e {
        TrustLinkError::NotFound         => MyError::NoAttestation,
        TrustLinkError::Unauthorized     => MyError::AccessDenied,
        TrustLinkError::AlreadyRevoked   => MyError::AttestationRevoked,
        TrustLinkError::Expired          => MyError::AttestationExpired,
        _                                => MyError::Unknown,
    })?;

    Ok(())
}
```

---

## 3. JavaScript / TypeScript Integration

### Installation

```bash
npm install @stellar/stellar-sdk
```

### Setup

```typescript
import {
  Contract,
  Networks,
  TransactionBuilder,
  SorobanRpc,
  Keypair,
  nativeToScVal,
  scValToNative,
  xdr,
} from "@stellar/stellar-sdk";

const RPC_URL = "https://soroban-testnet.stellar.org";
const CONTRACT_ID = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCN8";
const NETWORK_PASSPHRASE = Networks.TESTNET;

const server = new SorobanRpc.Server(RPC_URL);
```

### Check if a Wallet Has a Valid Claim

```typescript
async function hasValidClaim(
  subjectAddress: string,
  claimType: string
): Promise<boolean> {
  const contract = new Contract(CONTRACT_ID);

  const operation = contract.call(
    "has_valid_claim",
    nativeToScVal(subjectAddress, { type: "address" }),
    nativeToScVal(claimType, { type: "string" })
  );

  const account = await server.getAccount(subjectAddress);
  const tx = new TransactionBuilder(account, {
    fee: "100",
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(operation)
    .setTimeout(30)
    .build();

  const simResult = await server.simulateTransaction(tx);

  if (SorobanRpc.Api.isSimulationError(simResult)) {
    throw new Error(`Simulation failed: ${simResult.error}`);
  }

  const result = simResult.result?.retval;
  return result ? scValToNative(result) : false;
}

// Usage
const isKYCd = await hasValidClaim(
  "GABC...XYZ",
  "KYC_PASSED"
);
console.log("Has valid KYC:", isKYCd);
```

### Fetch an Attestation

```typescript
async function getAttestation(
  callerKeypair: Keypair,
  attestationId: string
): Promise<Record<string, unknown>> {
  const contract = new Contract(CONTRACT_ID);

  const operation = contract.call(
    "get_attestation",
    nativeToScVal(attestationId, { type: "string" })
  );

  const account = await server.getAccount(callerKeypair.publicKey());
  const tx = new TransactionBuilder(account, {
    fee: "100",
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(operation)
    .setTimeout(30)
    .build();

  const simResult = await server.simulateTransaction(tx);

  if (SorobanRpc.Api.isSimulationError(simResult)) {
    throw new Error(`Simulation failed: ${simResult.error}`);
  }

  const retval = simResult.result?.retval;
  if (!retval) throw new Error("No result returned");

  return scValToNative(retval);
}
```

### Create an Attestation (Issuer)

```typescript
async function createAttestation(
  issuerKeypair: Keypair,
  subjectAddress: string,
  claimType: string,
  expirationTimestamp?: number
): Promise<string> {
  const contract = new Contract(CONTRACT_ID);

  const expirationArg = expirationTimestamp
    ? xdr.ScVal.scvVec([nativeToScVal(expirationTimestamp, { type: "u64" })])
    : xdr.ScVal.scvVoid();

  const operation = contract.call(
    "create_attestation",
    nativeToScVal(issuerKeypair.publicKey(), { type: "address" }),
    nativeToScVal(subjectAddress, { type: "address" }),
    nativeToScVal(claimType, { type: "string" }),
    expirationArg
  );

  const account = await server.getAccount(issuerKeypair.publicKey());
  let tx = new TransactionBuilder(account, {
    fee: "100",
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(operation)
    .setTimeout(30)
    .build();

  const simResult = await server.simulateTransaction(tx);
  if (SorobanRpc.Api.isSimulationError(simResult)) {
    throw new Error(`Simulation failed: ${simResult.error}`);
  }

  tx = SorobanRpc.assembleTransaction(tx, simResult).build();
  tx.sign(issuerKeypair);

  const sendResult = await server.sendTransaction(tx);
  if (sendResult.status === "ERROR") {
    throw new Error(`Transaction failed: ${sendResult.errorResult}`);
  }

  // Poll for confirmation
  let getResult = await server.getTransaction(sendResult.hash);
  while (getResult.status === SorobanRpc.Api.GetTransactionStatus.NOT_FOUND) {
    await new Promise((r) => setTimeout(r, 1000));
    getResult = await server.getTransaction(sendResult.hash);
  }

  if (getResult.status === SorobanRpc.Api.GetTransactionStatus.FAILED) {
    throw new Error("Transaction failed on-chain");
  }

  const retval = getResult.returnValue;
  return retval ? scValToNative(retval) : "";
}
```

### Error Handling in TypeScript

TrustLink errors surface as simulation or transaction errors. Map them for clean UX:

```typescript
const TRUSTLINK_ERRORS: Record<number, string> = {
  1: "Contract already initialized",
  2: "Contract not initialized",
  3: "Unauthorized — not an admin or issuer",
  4: "Attestation not found",
  5: "Duplicate attestation",
  6: "Attestation already revoked",
  7: "Attestation has expired",
};

function parseTrustLinkError(error: unknown): string {
  const msg = String(error);
  const match = msg.match(/Error\(Contract, #(\d+)\)/);
  if (match) {
    const code = parseInt(match[1], 10);
    return TRUSTLINK_ERRORS[code] ?? `Unknown TrustLink error #${code}`;
  }
  return msg;
}

// Usage
try {
  await createAttestation(issuerKeypair, subject, "KYC_PASSED");
} catch (err) {
  console.error("TrustLink error:", parseTrustLinkError(err));
}
```

---

## 4. Testing Against Testnet

Use the Soroban CLI to interact with the testnet contract directly:

```bash
# Check if an address has a valid claim
soroban contract invoke \
  --id CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCN8 \
  --network testnet \
  -- has_valid_claim \
  --subject GABC...XYZ \
  --claim_type KYC_PASSED

# Fetch an attestation by ID
soroban contract invoke \
  --id CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCN8 \
  --network testnet \
  -- get_attestation \
  --attestation_id <ATTESTATION_ID>
```

Fund a testnet account with Friendbot if needed:

```bash
curl "https://friendbot.stellar.org?addr=YOUR_PUBLIC_KEY"
```

---

## 5. Local Development Setup

```bash
# Clone and build
git clone https://github.com/your-org/trustlink
cd trustlink
make build

# Run tests
make test

# Deploy to testnet
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/trustlink.wasm \
  --network testnet \
  --source YOUR_SECRET_KEY

# Initialize
soroban contract invoke \
  --id <YOUR_CONTRACT_ID> \
  --network testnet \
  --source YOUR_SECRET_KEY \
  -- initialize \
  --admin YOUR_PUBLIC_KEY
```

---

For the full API reference, see the [README](../README.md). For error definitions and type details, see [`src/types.rs`](../src/types.rs).
