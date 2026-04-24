# Stellar & Soroban Concepts for New Developers

If you're coming from EVM or general backend development, a few Stellar/Soroban concepts work differently than you might expect. This document explains the ones that come up most often when working with TrustLink.

Official docs: https://developers.stellar.org/docs/build/smart-contracts/overview

---

## 1. Ledger Timestamps vs Wall Clock Time

In Soroban, time comes from the **ledger**, not a system clock.

```rust
let now: u64 = env.ledger().timestamp();
```

This returns the Unix timestamp of the most recently closed ledger. A few things to keep in mind:

- Ledgers close roughly every **5–6 seconds** on Stellar mainnet
- The timestamp is set by validator consensus — it is not the exact moment your transaction was submitted
- There is no `block.timestamp` drift concern like on Ethereum, but the granularity is still ledger-level, not millisecond-level
- You cannot get the current wall clock time from inside a contract — `env.ledger().timestamp()` is the only time source available

TrustLink uses ledger timestamps for attestation creation time, expiration checks, and `valid_from` enforcement. When writing tests, you control time by advancing the ledger:

```rust
env.ledger().set_timestamp(current + 1000);
```

Official reference: https://developers.stellar.org/docs/build/smart-contracts/getting-started/storing-data

---

## 2. Persistent vs Instance vs Temporary Storage

Soroban has three storage types, each with different lifetime and cost characteristics. Choosing the wrong one is a common mistake.

| Type | Lifetime | Use for |
|------|----------|---------|
| `Persistent` | Survives indefinitely (with TTL bumps) | Long-lived data: attestations, issuer registry |
| `Instance` | Tied to the contract instance's TTL | Contract-level config: admin address, version |
| `Temporary` | Automatically deleted after TTL expires | Short-lived data: nonces, session state |

### How TTL works

Every storage entry has a **Time To Live (TTL)** measured in ledgers. When the TTL reaches zero, the entry is archived (persistent/instance) or deleted (temporary). You extend TTL by calling `bump`:

```rust
// Extend a persistent entry's TTL
env.storage().persistent().extend_ttl(&key, min_ledgers, max_ledgers);

// Extend the contract instance TTL
env.storage().instance().extend_ttl(min_ledgers, max_ledgers);
```

TrustLink bumps TTL on every read and write to ensure attestation data stays live. If you fork TrustLink and add new storage keys, make sure to bump their TTL appropriately — otherwise entries will silently disappear after enough ledgers pass.

Official reference: https://developers.stellar.org/docs/build/smart-contracts/storage/state-archival

---

## 3. `require_auth` and How Stellar Auth Works

Soroban's authorization model is explicit and pull-based — a contract asks an address to authorize an action rather than checking `msg.sender`.

```rust
pub fn create_attestation(env: Env, issuer: Address, ...) {
    issuer.require_auth(); // issuer must have signed this invocation
    // ...
}
```

### What `require_auth` does

- It checks that the `issuer` address has authorized this specific contract invocation
- For a regular account (G... address), this means the account's keypair signed the transaction
- For a contract address, it means the calling contract explicitly authorized the sub-call

### Why this matters for TrustLink

TrustLink passes the issuer as an explicit argument and calls `require_auth()` on it. This means:

- The issuer's keypair must sign the transaction — you can't spoof the issuer by just passing their address
- In cross-contract calls, the calling contract must invoke `issuer.require_auth_for_args(...)` to propagate authorization down the call stack

### Auth in tests

In tests, you authorize an address with:

```rust
issuer.mock_auths(&[MockAuth {
    address: &issuer,
    invoke: &MockAuthInvoke {
        contract: &contract_id,
        fn_name: "create_attestation",
        args: (...).into_val(&env),
        sub_invokes: &[],
    },
}]);
```

Or use `mock_all_auths()` to skip auth checks entirely during unit testing.

Official reference: https://developers.stellar.org/docs/build/smart-contracts/authorization

---

## 4. WASM Contract Deployment Model

Soroban contracts are compiled to **WebAssembly (WASM)** and deployed in two steps.

### Step 1 — Upload the WASM blob

```bash
soroban contract upload \
  --wasm target/wasm32-unknown-unknown/release/trustlink.wasm \
  --network testnet \
  --source YOUR_SECRET_KEY
```

This stores the bytecode on-chain and returns a **WASM hash**. The same bytecode uploaded once can be reused by many contract instances.

### Step 2 — Deploy a contract instance

```bash
soroban contract deploy \
  --wasm-hash <WASM_HASH> \
  --network testnet \
  --source YOUR_SECRET_KEY
```

This creates a contract instance with its own storage and returns a **Contract ID** (a C... address). The `deploy` command can also accept `--wasm` directly and handles the upload implicitly.

### Why this matters

- The WASM hash is separate from the contract ID — upgrading a contract means uploading new WASM and calling an upgrade function, not redeploying
- Contract IDs are deterministic when using `soroban contract deploy` with a specific salt
- Cross-contract calls use the Contract ID, not the WASM hash

### Cross-contract imports in Rust

When integrating TrustLink into another contract, you import its interface at compile time:

```rust
mod trustlink {
    soroban_sdk::contractimport!(
        file = "../trustlink/target/wasm32-unknown-unknown/release/trustlink.wasm"
    );
}
```

This generates a typed `Client` struct. At runtime you pass the live Contract ID:

```rust
let client = trustlink::Client::new(&env, &trustlink_contract_id);
client.has_valid_claim(&subject, &claim_type);
```

Official reference: https://developers.stellar.org/docs/build/smart-contracts/getting-started/deploy-increment-contract

---

## 5. Events and Off-Chain Indexing

Soroban contracts emit events that are recorded in the transaction metadata but are **not stored in contract storage**. They are consumed by off-chain indexers.

```rust
env.events().publish(
    (symbol_short!("created"), subject.clone()),
    (attestation_id.clone(), issuer.clone(), claim_type.clone(), timestamp),
);
```

Key points:

- Events are write-only from the contract's perspective — you cannot read past events from inside a contract
- Events are identified by a **topics** vector and a **data** payload
- Use the Stellar Horizon API or a custom indexer to subscribe to and query events off-chain
- TrustLink emits events for every state change (attestation created, revoked, renewed, issuer registered/removed) — see the Events section in the [README](../README.md) for the full list

Official reference: https://developers.stellar.org/docs/build/smart-contracts/events

---

## Further Reading

| Topic | Link |
|-------|------|
| Soroban overview | https://developers.stellar.org/docs/build/smart-contracts/overview |
| Storage & state archival | https://developers.stellar.org/docs/build/smart-contracts/storage/state-archival |
| Authorization | https://developers.stellar.org/docs/build/smart-contracts/authorization |
| Contract deployment | https://developers.stellar.org/docs/build/smart-contracts/getting-started/deploy-increment-contract |
| Events | https://developers.stellar.org/docs/build/smart-contracts/events |
| Soroban SDK docs | https://docs.rs/soroban-sdk/latest/soroban_sdk/ |
| TrustLink Integration Guide | ./integration-guide.md |
| TrustLink Storage Layout | ./storage-layout.md |
