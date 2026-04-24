# TrustLink Video Tutorial — Companion Guide

This guide accompanies the [TrustLink video tutorial](https://www.youtube.com/watch?v=TODO_REPLACE_WITH_VIDEO_ID).  
It covers the same material in written form so you can follow along at your own pace, copy-paste commands, and refer back without scrubbing through video.

---

## What You'll Learn

- What TrustLink is and why it exists
- How to deploy TrustLink to the Stellar testnet
- How to issue and verify attestations via the CLI
- How to integrate TrustLink into a Rust smart contract
- How to query TrustLink from a JavaScript / TypeScript frontend

---

## Prerequisites

| Tool | Install |
|------|---------|
| Rust + Cargo | https://rustup.rs |
| `wasm32-unknown-unknown` target | `rustup target add wasm32-unknown-unknown` |
| Soroban CLI | `cargo install --locked soroban-cli` |
| Node.js ≥ 18 (for JS section) | https://nodejs.org |

Fund a testnet account via Friendbot:

```bash
curl "https://friendbot.stellar.org?addr=YOUR_PUBLIC_KEY"
```

---

## 1. What is TrustLink?

TrustLink is a Soroban smart contract that provides a shared attestation layer on the Stellar blockchain. It solves a common problem: every dApp that needs identity verification (KYC, AML, accredited investor status) ends up building its own system from scratch.

With TrustLink:

- An **admin** controls which addresses are trusted **issuers**
- **Issuers** create **attestations** that link a claim type (e.g. `KYC_PASSED`) to a subject wallet address
- Any contract or frontend can call `has_valid_claim` to check whether a wallet holds a valid attestation
- Attestations support optional expiration and can be revoked at any time

This means one deployed TrustLink instance can serve as the trust backbone for an entire ecosystem of contracts.

---

## 2. Clone and Build

```bash
git clone https://github.com/unixfundz/TrustLink.git
cd TrustLink

# Confirm tests pass
make test

# Build optimized wasm
make optimize
```

The compiled contract lands at:
```
target/wasm32-unknown-unknown/release/trustlink.wasm
```

---

## 3. Deploy to Testnet

```bash
# Deploy the wasm
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/trustlink.wasm \
  --network testnet \
  --source YOUR_SECRET_KEY
```

Save the contract ID printed to stdout — you'll use it in every subsequent command.

```bash
# Initialize with your admin address
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  --source YOUR_SECRET_KEY \
  -- initialize \
  --admin YOUR_PUBLIC_KEY
```

---

## 4. Register an Issuer and Create Attestations

Only the admin can register issuers. Only registered issuers can create attestations.

```bash
# Admin registers a trusted issuer
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  --source YOUR_SECRET_KEY \
  -- register_issuer \
  --issuer ISSUER_PUBLIC_KEY
```

```bash
# Issuer creates a KYC attestation (no expiration)
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  --source ISSUER_SECRET_KEY \
  -- create_attestation \
  --issuer ISSUER_PUBLIC_KEY \
  --subject SUBJECT_PUBLIC_KEY \
  --claim_type KYC_PASSED \
  --expiration null
```

```bash
# Verify the claim (read-only, no signing needed)
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- has_valid_claim \
  --subject SUBJECT_PUBLIC_KEY \
  --claim_type KYC_PASSED
```

Expected output: `true`

### Available Claim Types

| Claim Type | Meaning |
|---|---|
| `KYC_PASSED` | Subject passed KYC identity verification |
| `ACCREDITED_INVESTOR` | Subject qualifies as an accredited investor |
| `MERCHANT_VERIFIED` | Subject is a verified merchant |
| `AML_CLEARED` | Subject passed AML screening |
| `SANCTIONS_CHECKED` | Subject checked against sanctions lists |

You can also register custom claim types:

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  --source YOUR_SECRET_KEY \
  -- register_claim_type \
  --claim_type MY_CUSTOM_CLAIM \
  --description "Description of what this claim means"
```

---

## 5. Cross-Contract Integration (Rust)

Add TrustLink as a dependency in your contract's `Cargo.toml`:

```toml
[dependencies]
soroban-sdk = "21.0.0"
trustlink = { git = "https://github.com/unixfundz/TrustLink.git", tag = "v0.1.0" }
```

Import the generated client and gate your function:

```rust
mod trustlink {
    soroban_sdk::contractimport!(
        file = "../trustlink/target/wasm32-unknown-unknown/release/trustlink.wasm"
    );
}

#[contractimpl]
impl LendingContract {
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

        // lending logic
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

The `contractimport!` macro generates a fully typed client at compile time — no manual ABI wrangling.

### Checking Multiple Claims

```rust
// Require KYC AND AML clearance
let mut required = soroban_sdk::Vec::new(&env);
required.push_back(String::from_str(&env, "KYC_PASSED"));
required.push_back(String::from_str(&env, "AML_CLEARED"));

if !trustlink.has_all_claims(&borrower, &required) {
    return Err(Error::InsufficientCredentials);
}
```

---

## 6. JavaScript / TypeScript Integration

```bash
npm install @stellar/stellar-sdk
```

### Read-only claim check (no signing)

```typescript
import {
  Contract, Networks, TransactionBuilder,
  SorobanRpc, nativeToScVal, scValToNative,
} from "@stellar/stellar-sdk";

const server = new SorobanRpc.Server("https://soroban-testnet.stellar.org");
const CONTRACT_ID = "<YOUR_CONTRACT_ID>";

async function hasValidClaim(subject: string, claimType: string): Promise<boolean> {
  const contract = new Contract(CONTRACT_ID);
  const op = contract.call(
    "has_valid_claim",
    nativeToScVal(subject, { type: "address" }),
    nativeToScVal(claimType, { type: "string" })
  );

  const account = await server.getAccount(subject);
  const tx = new TransactionBuilder(account, {
    fee: "100",
    networkPassphrase: Networks.TESTNET,
  }).addOperation(op).setTimeout(30).build();

  const sim = await server.simulateTransaction(tx);
  if (SorobanRpc.Api.isSimulationError(sim)) throw new Error(sim.error);

  return scValToNative(sim.result?.retval);
}

// Example
const isKYCd = await hasValidClaim("GABC...XYZ", "KYC_PASSED");
console.log("KYC valid:", isKYCd);
```

### Error handling

```typescript
const ERRORS: Record<number, string> = {
  1: "Already initialized",
  2: "Not initialized",
  3: "Unauthorized",
  4: "Not found",
  5: "Duplicate attestation",
  6: "Already revoked",
  7: "Expired",
};

function parseTrustLinkError(err: unknown): string {
  const match = String(err).match(/Error\(Contract, #(\d+)\)/);
  if (match) return ERRORS[parseInt(match[1])] ?? `Unknown error #${match[1]}`;
  return String(err);
}
```

---

## 7. Further Reading

- [README](../README.md) — full API reference
- [Integration Guide](./integration-guide.md) — deeper patterns, pagination, testnet CLI examples
- [Storage Layout](./storage-layout.md) — on-chain key reference for indexer developers
- [GitHub Issues](https://github.com/unixfundz/TrustLink/issues) — bug reports and feature requests
