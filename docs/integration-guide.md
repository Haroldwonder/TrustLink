# TrustLink Integration Guide

This guide walks through integrating TrustLink into your dApp — whether you're building a Rust smart contract that needs on-chain claim verification, or a JavaScript/TypeScript frontend that interacts with the contract directly.

For definitions of terms used throughout this guide (attestation, issuer, subject, bridge, claim type, etc.), see the [Glossary](./glossary.md).

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

        // Alternatively, check for an attestation from a specific issuer
        // let specific_issuer = Address::from_string(&String::from_str(&env, "GBXYZ..."));
        // if !trustlink.has_valid_claim_from_issuer(&borrower, &claim, &specific_issuer) {
        //     return Err(Error::KYCRequired);
        // }

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

### Which Package Should I Use?

The repository ships **two separate TypeScript packages** that both export a
class named `TrustLinkClient`. They serve different purposes and are
intentionally kept separate:

| | `@trustlink/sdk` | `@trustlink/bindings` |
|---|---|---|
| **npm install** | `npm install @trustlink/sdk` | `npm install @trustlink/bindings` |
| **Source path** | `sdk/typescript/` | `bindings/typescript/` |
| **Purpose** | Read-heavy frontends, dApps, dashboards | Server-side signers, scripts, raw access |
| **Retry / circuit-breaker** | ✅ Automatic on all reads | ✗ |
| **Pagination helpers** | ✅ `iterateSubjectAttestations` etc. | ✗ |
| **Write operations** | Simulation only — returns raw tx for external signing | ✅ Full sign+submit with `Keypair` |
| **Class name** | `TrustLinkClient` | `TrustLinkClient` |

**Decision guide:**

- Building a **frontend / React app / dApp** that reads claims and shows
  verification status → use **`@trustlink/sdk`**. It handles retry, circuit
  breaking, and pagination automatically.

- Building a **backend service or script** that creates, revokes, or manages
  attestations (and holds a secret key) → use **`@trustlink/bindings`**. It
  exposes every write entry point and handles the full sign-and-submit
  lifecycle.

- Need **both reads and writes** in the same codebase (e.g. an admin portal
  that displays stats and creates attestations)? Install both packages. To
  avoid naming collisions, alias one:

  ```typescript
  import { TrustLinkClient as ReadClient } from "@trustlink/sdk";
  import { TrustLinkClient as WriteClient } from "@trustlink/bindings";
  ```

> If neither package exposes the specific entry point you need, you can always
> fall back to the raw `@stellar/stellar-sdk` approach shown at the end of
> this section.

---

### Using `@trustlink/sdk` (Reads / Frontend)

```bash
npm install @trustlink/sdk
```

```typescript
import { TrustLinkClient } from "@trustlink/sdk";

const client = new TrustLinkClient({
  contractId: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCN8",
  network: "testnet",
});

// Check if a wallet has a valid KYC attestation (auto-retried on transient failures)
const hasKyc = await client.hasValidClaim(
  "GABC...XYZ",
  "KYC_PASSED"
);
console.log("Has valid KYC:", hasKyc);

// Fetch a single attestation
const att = await client.getAttestation(attestationId);
console.log(att.claim_type, att.revoked, att.expiration);

// Paginate without writing a loop yourself
for await (const attestation of client.iterateSubjectAttestations("GABC...")) {
  console.log(attestation.id, attestation.claim_type);
}

// OR-logic: any of these claim types
const canTrade = await client.hasAnyClaim("GABC...", [
  "KYC_PASSED",
  "ACCREDITED_INVESTOR",
]);

// AND-logic: all required
const canBorrowLarge = await client.hasAllClaims("GABC...", [
  "KYC_PASSED",
  "AML_CLEARED",
]);
```

---

### Using `@trustlink/bindings` (Writes / Backend)

```bash
npm install @trustlink/bindings
```

```typescript
import { TrustLinkClient } from "@trustlink/bindings";
import { Keypair } from "@stellar/stellar-sdk";

const client = new TrustLinkClient({
  contractId: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCN8",
  rpcUrl: "https://soroban-testnet.stellar.org",
});

const issuer = Keypair.fromSecret(process.env.ISSUER_SECRET!);

// Create an attestation (signs and submits)
const oneYear = BigInt(Math.floor(Date.now() / 1000) + 365 * 24 * 60 * 60);
const attestationId = await client.createAttestation(
  issuer,
  "GBRPYHIL...",   // subject
  "KYC_PASSED",
  oneYear,
  JSON.stringify({ provider: "acme-kyc" })
);
console.log("Created:", attestationId);

// Revoke
await client.revokeAttestation(issuer, attestationId, "Account closed");

// Admin operations
const admin = Keypair.fromSecret(process.env.ADMIN_SECRET!);
await client.registerIssuer(admin, issuer.publicKey());
```

---

### Raw `@stellar/stellar-sdk` Approach (Advanced)

If you need direct control over transaction building — or you're integrating
with a wallet that provides its own signing flow — you can bypass both packages
and call the contract directly:

```bash
npm install @stellar/stellar-sdk
```

```typescript
import {
  Contract,
  Networks,
  TransactionBuilder,
  SorobanRpc,
  Keypair,
  nativeToScVal,
  scValToNative,
  BASE_FEE,
} from "@stellar/stellar-sdk";

const server = new SorobanRpc.Server("https://soroban-testnet.stellar.org");
const contract = new Contract("CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCN8");

// ── Read: simulate has_valid_claim ────────────────────────────────────────────
const dummyAccount = "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN";
const readTx = new TransactionBuilder(
  new (await import("@stellar/stellar-sdk")).Account(dummyAccount, "0"),
  { fee: BASE_FEE, networkPassphrase: Networks.TESTNET }
)
  .addOperation(contract.call(
    "has_valid_claim",
    nativeToScVal("GABC...XYZ", { type: "address" }),
    nativeToScVal("KYC_PASSED", { type: "string" })
  ))
  .setTimeout(30)
  .build();

const simResult = await server.simulateTransaction(readTx);
if (SorobanRpc.Api.isSimulationError(simResult)) {
  throw new Error(`Simulation failed: ${simResult.error}`);
}
const hasKyc: boolean = scValToNative(simResult.result!.retval);

// ── Write: create_attestation ─────────────────────────────────────────────────
const issuerKeypair = Keypair.fromSecret("S...");
const account = await server.getAccount(issuerKeypair.publicKey());

let writeTx = new TransactionBuilder(account, {
  fee: BASE_FEE,
  networkPassphrase: Networks.TESTNET,
})
  .addOperation(contract.call(
    "create_attestation",
    nativeToScVal(issuerKeypair.publicKey(), { type: "address" }),
    nativeToScVal("GBRPYHIL...",              { type: "address" }),
    nativeToScVal("KYC_PASSED",               { type: "string" }),
    nativeToScVal(null),                      // no expiration
    nativeToScVal(null),                      // no metadata
    nativeToScVal(null)                       // no tags
  ))
  .setTimeout(30)
  .build();

const writeSimResult = await server.simulateTransaction(writeTx);
if (SorobanRpc.Api.isSimulationError(writeSimResult)) {
  throw new Error(`Simulation failed: ${writeSimResult.error}`);
}
writeTx = SorobanRpc.assembleTransaction(writeTx, writeSimResult).build();
writeTx.sign(issuerKeypair);

const sendResult = await server.sendTransaction(writeTx);
// Poll for confirmation…
let getResult = await server.getTransaction(sendResult.hash);
while (getResult.status === "NOT_FOUND") {
  await new Promise((r) => setTimeout(r, 1500));
  getResult = await server.getTransaction(sendResult.hash);
}
if (getResult.status !== "SUCCESS") throw new Error("Transaction failed");
const attestationId: string = scValToNative(getResult.returnValue!);
```

---

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
  await client.createAttestation(issuer, subject, "KYC_PASSED");
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

---

## Reentrancy considerations for integrators

### What reentrancy means in Soroban (for Solidity developers)

In Solidity, reentrancy happens when an external call transfers control to a malicious contract that calls back into your contract before your state update completes — the classic DAO hack pattern. Soroban prevents this for the *same* contract: the host will trap and abort the transaction if contract A is called while A already has an active frame. However, **cross-contract calls to different contracts are not protected by this rule**. If your contract reads state, calls TrustLink (or any external contract), and then writes state based on the read value, a malicious contract in that call chain could modify shared state between your read and write.

**The rule: always write state before emitting events or invoking external contracts.**

This is the check-effects-interactions (CEI) pattern applied to Soroban:

1. **Check** — validate inputs and read any state you need.
2. **Effect** — write all state changes to storage.
3. **Interact** — only then call external contracts or emit events.

> [!WARNING]
> **Caller-supplied contract addresses are dangerous.** A common integrator mistake is accepting the TrustLink contract address as a runtime parameter (e.g. `trustlink_id: Address` passed by the transaction caller). A malicious caller can substitute a fake contract that returns `true` for any claim check, bypassing your access control entirely. **Always store the TrustLink contract address in your own contract's instance storage during initialisation and read it from there — never accept it from the caller.**

### Safe usage: calling `has_valid_claim` from an integrating contract

```rust
#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env, String};

mod trustlink {
    soroban_sdk::contractimport!(
        file = "../trustlink/target/wasm32-unknown-unknown/release/trustlink.wasm"
    );
}

#[contract]
pub struct LendingContract;

#[contractimpl]
impl LendingContract {
    /// Store the trusted TrustLink address once at deploy time.
    /// Never accept it as a per-call parameter.
    pub fn initialize(env: Env, admin: Address, trustlink_id: Address) {
        admin.require_auth();
        // Safety: stored once by admin; callers cannot substitute a fake address.
        env.storage().instance().set(&"trustlink", &trustlink_id);
    }

    pub fn request_loan(
        env: Env,
        borrower: Address,
        amount: i128,
        collateral: i128,
    ) -> Result<(), Error> {
        borrower.require_auth();

        // 1. CHECK — read the hardcoded TrustLink address from our own storage.
        //    This cannot be influenced by the transaction caller.
        let trustlink_id: Address = env
            .storage()
            .instance()
            .get(&"trustlink")
            .expect("not initialized");

        let trustlink = trustlink::Client::new(&env, &trustlink_id);
        let kyc_claim = String::from_str(&env, "KYC_PASSED");

        if !trustlink.has_valid_claim(&borrower, &kyc_claim) {
            return Err(Error::KYCRequired);
        }

        // 2. EFFECT — write all state changes before any further external calls.
        //    If we needed to update a balance or record the loan, do it here,
        //    before calling any other external contract.
        env.storage().instance().set(&borrower, &amount);

        // 3. INTERACT — any additional external calls (e.g. token transfers)
        //    happen last, after state is already committed.
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

### Further reading

- [Soroban security best practices](https://developers.stellar.org/docs/learn/smart-contract-internals/security)
- [Soroban authorization model](https://developers.stellar.org/docs/learn/smart-contract-internals/authorization)
- [Soroban reentrancy internals](https://developers.stellar.org/docs/learn/smart-contract-internals/contract-interactions/reentrancy)
- Full audit findings: [`docs/reentrancy-audit.md`](./reentrancy-audit.md)

---

---

## 6. Error Handling

TrustLink errors are returned as `Error(Contract, #N)` values. This section covers every error code, whether it is retryable, and how to surface it to end users.

### Error Code Reference

| Code | Name                  | Meaning                                                        | Retryable? |
|------|-----------------------|----------------------------------------------------------------|------------|
| `#1` | `AlreadyInitialized`  | `initialize()` was called on an already-initialized contract   | No         |
| `#2` | `NotInitialized`      | Contract has not been initialized yet                          | No         |
| `#3` | `Unauthorized`        | Caller is not the admin, a registered issuer, or the subject   | No         |
| `#4` | `NotFound`            | Attestation ID does not exist                                  | No         |
| `#5` | `DuplicateAttestation`| An attestation with the same deterministic hash already exists | No         |
| `#6` | `AlreadyRevoked`      | The attestation has already been revoked                       | No         |
| `#7` | `Expired`             | The attestation's expiration timestamp has passed              | No         |
| `#8` | `InvalidThreshold`    | Multi-sig threshold is 0 or exceeds the number of signers      | No         |
| `#9` | `NotRequiredSigner`   | Cosigner address is not in the proposal's required-signers list| No         |
| `#10`| `LimitExceeded`       | Issuer or subject has reached the configured attestation limit | No†        |
| `#11`| `AlreadySigned`       | This issuer has already co-signed the proposal                 | No         |
| `#12`| `ProposalFinalized`   | The multi-sig proposal has already been activated              | No         |
| `#13`| `ProposalExpired`     | The 7-day co-signing window elapsed without reaching threshold | No         |

> † `LimitExceeded` is not retryable for the same issuer/subject until the admin raises the limit or stale attestations are revoked.

### TypeScript: Catching and Handling TrustLinkError

```typescript
/** All TrustLink contract error codes mapped to structured metadata. */
const TRUSTLINK_ERRORS: Record<
  number,
  { name: string; retryable: boolean; userMessage: string }
> = {
  1:  { name: "AlreadyInitialized",   retryable: false, userMessage: "The contract is already set up." },
  2:  { name: "NotInitialized",       retryable: false, userMessage: "The contract is not yet available. Please try again later." },
  3:  { name: "Unauthorized",         retryable: false, userMessage: "You do not have permission to perform this action." },
  4:  { name: "NotFound",             retryable: false, userMessage: "The requested attestation could not be found." },
  5:  { name: "DuplicateAttestation", retryable: false, userMessage: "This attestation already exists." },
  6:  { name: "AlreadyRevoked",       retryable: false, userMessage: "This attestation has already been revoked." },
  7:  { name: "Expired",              retryable: false, userMessage: "This attestation has expired. Please renew your verification." },
  8:  { name: "InvalidThreshold",     retryable: false, userMessage: "The approval threshold is invalid." },
  9:  { name: "NotRequiredSigner",    retryable: false, userMessage: "Your account is not authorised to co-sign this proposal." },
  10: { name: "LimitExceeded",        retryable: false, userMessage: "The attestation limit has been reached. Please contact support." },
  11: { name: "AlreadySigned",        retryable: false, userMessage: "You have already signed this proposal." },
  12: { name: "ProposalFinalized",    retryable: false, userMessage: "This proposal has already been completed." },
  13: { name: "ProposalExpired",      retryable: false, userMessage: "The signing window for this proposal has closed." },
};

interface TrustLinkErrorInfo {
  code: number;
  name: string;
  retryable: boolean;
  userMessage: string;
}

/**
 * Parse a raw error thrown by the Stellar SDK into structured TrustLink error info.
 * Returns null if the error is not a TrustLink contract error.
 */
function parseTrustLinkError(error: unknown): TrustLinkErrorInfo | null {
  const msg = String(error);
  const match = msg.match(/Error\(Contract,\s*#(\d+)\)/);
  if (!match) return null;

  const code = parseInt(match[1], 10);
  const meta = TRUSTLINK_ERRORS[code];
  return {
    code,
    name: meta?.name ?? "UnknownError",
    retryable: meta?.retryable ?? false,
    userMessage: meta?.userMessage ?? `Unexpected error (code ${code}). Please contact support.`,
  };
}

// ---------------------------------------------------------------------------
// Usage example
// ---------------------------------------------------------------------------
async function verifyAndAttest(
  issuerKeypair: Keypair,
  subjectAddress: string,
  claimType: string
): Promise<void> {
  try {
    await createAttestation(issuerKeypair, subjectAddress, claimType);
    console.log("Attestation created successfully.");
  } catch (err) {
    const tlError = parseTrustLinkError(err);

    if (tlError) {
      console.error(`TrustLink error [${tlError.name}]:`, tlError.userMessage);

      if (tlError.retryable) {
        // Safe to retry after a short delay
        console.warn("This error is transient — retrying in 2 s…");
        await new Promise((r) => setTimeout(r, 2000));
        await createAttestation(issuerKeypair, subjectAddress, claimType);
      } else {
        // Surface the user-facing message in your UI
        throw new Error(tlError.userMessage);
      }
    } else {
      // Non-contract error (network, RPC, etc.) — may be retryable
      console.error("Unexpected error:", err);
      throw err;
    }
  }
}
```

### User-Facing Message Reference

Use this table to map error codes directly to UI copy:

| Code | Recommended user-facing message                                              |
|------|------------------------------------------------------------------------------|
| `#1` | "The contract is already set up."                                            |
| `#2` | "The contract is not yet available. Please try again later."                 |
| `#3` | "You do not have permission to perform this action."                         |
| `#4` | "The requested attestation could not be found."                              |
| `#5` | "This attestation already exists."                                           |
| `#6` | "This attestation has already been revoked."                                 |
| `#7` | "This attestation has expired. Please renew your verification."              |
| `#8` | "The approval threshold is invalid."                                         |
| `#9` | "Your account is not authorised to co-sign this proposal."                   |
| `#10`| "The attestation limit has been reached. Please contact support."            |
| `#11`| "You have already signed this proposal."                                     |
| `#12`| "This proposal has already been completed."                                  |
| `#13`| "The signing window for this proposal has closed."                           |

---

## 7. Fee Estimation

TrustLink involves two distinct fees that integrators should surface to users before submitting a transaction:

| Fee type | What it is | How to obtain it |
|---|---|---|
| **Stellar network fee** | XLM paid to validators for transaction inclusion | Returned by `simulateTransaction` |
| **Attestation fee** | Token amount charged by the TrustLink contract (if configured by admin) | Returned by `get_fee_config` |

### Stellar Network Fee via `simulateTransaction`

Call `simulateTransaction` before submitting. The simulation result contains the minimum resource fee the network will accept.

```typescript
import {
  Contract,
  Networks,
  TransactionBuilder,
  SorobanRpc,
  nativeToScVal,
} from "@stellar/stellar-sdk";

const server = new SorobanRpc.Server("https://soroban-testnet.stellar.org");
const CONTRACT_ID = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCN8";

async function estimateNetworkFee(
  issuerPublicKey: string,
  subjectAddress: string,
  claimType: string
): Promise<{ networkFeeLumens: string; minResourceFee: string }> {
  const contract = new Contract(CONTRACT_ID);

  const operation = contract.call(
    "create_attestation",
    nativeToScVal(issuerPublicKey, { type: "address" }),
    nativeToScVal(subjectAddress, { type: "address" }),
    nativeToScVal(claimType, { type: "string" }),
    nativeToScVal(null, { type: "void" }), // no expiration
    nativeToScVal(null, { type: "void" })  // no metadata
  );

  const account = await server.getAccount(issuerPublicKey);
  const tx = new TransactionBuilder(account, {
    fee: "100", // base fee in stroops — will be replaced by simulation result
    networkPassphrase: Networks.TESTNET,
  })
    .addOperation(operation)
    .setTimeout(30)
    .build();

  const simResult = await server.simulateTransaction(tx);

  if (SorobanRpc.Api.isSimulationError(simResult)) {
    throw new Error(`Simulation failed: ${simResult.error}`);
  }

  // minResourceFee is in stroops (1 XLM = 10_000_000 stroops)
  const minResourceFee = simResult.minResourceFee ?? "0";
  const networkFeeLumens = (parseInt(minResourceFee, 10) / 1e7).toFixed(7);

  return { networkFeeLumens, minResourceFee };
}

// Usage
const { networkFeeLumens } = await estimateNetworkFee(
  issuerKeypair.publicKey(),
  "GABC...XYZ",
  "KYC_PASSED"
);
console.log(`Estimated network fee: ${networkFeeLumens} XLM`);
```

### Attestation Fee (Contract-Level)

If the admin has configured an attestation fee, `create_attestation` will transfer tokens from the issuer to the fee collector. Fetch the current fee config before prompting the user:

```typescript
async function getAttestationFee(callerPublicKey: string): Promise<{
  feeEnabled: boolean;
  amount: bigint;
  tokenAddress: string | null;
  collectorAddress: string;
} | null> {
  const contract = new Contract(CONTRACT_ID);

  const operation = contract.call("get_fee_config");

  const account = await server.getAccount(callerPublicKey);
  const tx = new TransactionBuilder(account, {
    fee: "100",
    networkPassphrase: Networks.TESTNET,
  })
    .addOperation(operation)
    .setTimeout(30)
    .build();

  const simResult = await server.simulateTransaction(tx);

  if (SorobanRpc.Api.isSimulationError(simResult)) {
    throw new Error(`Simulation failed: ${simResult.error}`);
  }

  const retval = simResult.result?.retval;
  if (!retval) return null;

  const config = scValToNative(retval) as {
    attestation_fee: bigint;
    fee_collector: string;
    fee_token: string | null;
  };

  return {
    feeEnabled: config.attestation_fee > 0n,
    amount: config.attestation_fee,
    tokenAddress: config.fee_token,
    collectorAddress: config.fee_collector,
  };
}

// Usage
const attestationFee = await getAttestationFee(issuerKeypair.publicKey());

if (attestationFee?.feeEnabled) {
  console.log(
    `Attestation fee: ${attestationFee.amount} token units`,
    `(token: ${attestationFee.tokenAddress})`
  );
} else {
  console.log("No attestation fee configured.");
}
```

### Displaying Both Fees to Users

Combine both estimates before asking the user to sign:

```typescript
async function estimateCreateAttestationCost(
  issuerPublicKey: string,
  subjectAddress: string,
  claimType: string
) {
  const [networkFee, attestationFee] = await Promise.all([
    estimateNetworkFee(issuerPublicKey, subjectAddress, claimType),
    getAttestationFee(issuerPublicKey),
  ]);

  console.log(`Network fee:     ~${networkFee.networkFeeLumens} XLM`);

  if (attestationFee?.feeEnabled) {
    console.log(
      `Attestation fee: ${attestationFee.amount} token units`,
      `→ paid to ${attestationFee.collectorAddress}`
    );
  } else {
    console.log("Attestation fee: none");
  }
}
```

> **Note:** `simulateTransaction` reflects the fee at simulation time. Network congestion can cause the actual fee to differ slightly. Use `SorobanRpc.assembleTransaction` to apply the simulation's recommended fee before signing.

---

For the full API reference, see the [README](../README.md). For error definitions and type details, see [`src/types.rs`](../src/types.rs).

If something is not working as expected, see the [Troubleshooting & FAQ](./troubleshooting.md) guide for common integration errors, their causes, and fixes.
