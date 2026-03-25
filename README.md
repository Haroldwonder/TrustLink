# TrustLink - On-Chain Attestation & Verification System

[![CI](https://github.com/afurious/TrustLink/actions/workflows/ci.yml/badge.svg)](https://github.com/afurious/TrustLink/actions/workflows/ci.yml)

TrustLink is a Soroban smart contract that provides a reusable trust layer for the Stellar blockchain. It enables trusted issuers, bridge contracts, and administrators to create, import, manage, and revoke attestations about wallet addresses, allowing other contracts and applications to verify claims before executing financial operations.

## Overview

TrustLink solves the problem of decentralized identity verification and trust establishment on-chain. Instead of each application building its own KYC/verification system, TrustLink provides a shared attestation infrastructure that can be queried by any smart contract or dApp.

### Key Features

- **Authorized Issuers**: Admin-controlled registry of trusted attestation issuers
- **Claim Type Registry**: Admin-managed registry of standard claim types with descriptions
- **Flexible Claims**: Support for any claim type (KYC_PASSED, ACCREDITED_INVESTOR, MERCHANT_VERIFIED, etc.)
- **Expiration Support**: Optional time-based expiration for attestations
- **Historical Import**: Admin can import externally verified attestations with original timestamps
- **Cross-Chain Bridge Support**: Trusted bridge contracts can bring attestations from other chains on-chain
- **Configurable Fees**: Admin can require a token-denominated fee for native attestation creation
- **Revocation**: Issuers can revoke attestations at any time
- **Deterministic IDs**: Attestations have unique, reproducible identifiers
- **Event Emission**: All state changes emit events for off-chain indexing
- **Query Interface**: Easy verification of claims for other contracts
- **Pagination**: Efficient listing of attestations per subject or issuer

## Architecture

### Core Components

```
src/
├── lib.rs          # Main contract implementation
├── types.rs        # Data structures and error definitions
├── storage.rs      # Storage patterns and key management
├── validation.rs   # Authorization and access control
├── events.rs       # Event emission for indexers
└── test.rs         # Comprehensive unit tests
```

### Data Model

**Attestation Structure:**
```rust
{
    id: String,               // Deterministic hash-based ID
    issuer: Address,          // Who issued the attestation
    subject: Address,         // Who the attestation is about
    claim_type: String,       // Type of claim (e.g., "KYC_PASSED")
    timestamp: u64,           // When it was created
    expiration: Option<u64>,  // Optional expiration time
    revoked: bool,            // Revocation status
    metadata: Option<String>, // Optional issuer-supplied metadata
    imported: bool,           // True when migrated from an external source
    bridged: bool,            // True when created by a trusted bridge contract
    source_chain: Option<String>, // Chain where the original attestation exists
    source_tx: Option<String> // Source transaction or reference
}
```

**Storage Keys:**
- `Admin`: Contract administrator address
- `FeeConfig`: Global attestation fee settings
- `Issuer(Address)`: Authorized issuer registry
- `Bridge(Address)`: Authorized bridge contract registry
- `Attestation(String)`: Individual attestation data
- `SubjectAttestations(Address)`: Index of attestations per subject
- `IssuerAttestations(Address)`: Index of attestations per issuer
- `ClaimType(String)`: Registered claim type info keyed by identifier
- `ClaimTypeList`: Ordered list of all registered claim type identifiers

## Usage

### Initialization

```rust
// Deploy and initialize with admin and optional custom TTL (days)
// ttl_days: None uses default 30 days, or Some(7) for custom TTL
contract.initialize(&admin_address, &None);
```

### Configure Attestation Fees

Fees are disabled by default. When enabled, `create_attestation` transfers the
configured amount from the issuer to the configured collector before the
attestation is stored.

The contract stores an explicit `fee_token` because Soroban fee collection must
transfer a concrete token contract rather than an abstract currency amount.

```rust
let fee_token = token_contract_address;

contract.set_fee(
    &admin,
    &25,
    &collector_address,
    &Some(fee_token),
);

let fee_config = contract.get_fee_config();
assert_eq!(fee_config.attestation_fee, 25);
assert_eq!(fee_config.fee_collector, collector_address);
```

### Register Issuers

```rust
// Admin registers a trusted issuer
contract.register_issuer(&admin, &issuer_address);

// Check if address is authorized
let is_authorized = contract.is_issuer(&issuer_address);
```

### Register Bridge Contracts

Bridge contracts use a separate trust registry from regular issuers.

```rust
contract.register_bridge(&admin, &bridge_contract_address);

let is_bridge = contract.is_bridge(&bridge_contract_address);
```

### Claim Type Registry

The contract ships with a set of standard claim types that the admin can pre-register on deployment.

| Claim Type | Description |
|---|---|
| `KYC_PASSED` | Subject has passed KYC identity verification |
| `ACCREDITED_INVESTOR` | Subject qualifies as an accredited investor |
| `MERCHANT_VERIFIED` | Subject is a verified merchant |
| `AML_CLEARED` | Subject has passed AML screening |
| `SANCTIONS_CHECKED` | Subject has been checked against sanctions lists |

```rust
// Admin registers a claim type
contract.register_claim_type(
    &admin,
    &String::from_str(&env, "KYC_PASSED"),
    &String::from_str(&env, "Subject has passed KYC identity verification"),
);

// Look up a description
let desc = contract.get_claim_type_description(&String::from_str(&env, "KYC_PASSED"));

// List registered types (paginated)
let page1 = contract.list_claim_types(&0, &10);
```

### Create Attestations

Issuers cannot create an attestation where they are also the subject (`issuer ==
subject`); that would allow trivial self-certification. The contract returns
`Unauthorized` in that case.

If fees are enabled, the issuer must hold enough of the configured token for
the transfer to succeed.

```rust
// Issuer creates a KYC attestation
let attestation_id = contract.create_attestation(
    &issuer,
    &user_address,
    &String::from_str(&env, "KYC_PASSED"),
    &None,  // No expiration
    &None   // No metadata
);

// Create attestation with expiration
let expiration_time = current_timestamp + 365 * 24 * 60 * 60; // 1 year
let attestation_id = contract.create_attestation(
    &issuer,
    &user_address,
    &String::from_str(&env, "ACCREDITED_INVESTOR"),
    &Some(expiration_time),
    &None
);
```

### Import Historical Attestations

Use this when migrating records from another verified system. The admin performs
the import, but the imported record is still attached to a registered issuer.

```rust
let historical_timestamp = 1_700_000_000;
let expiration = Some(1_731_536_000);

let imported_id = contract.import_attestation(
    &admin,
    &issuer,
    &user_address,
    &String::from_str(&env, "KYC_PASSED"),
    &historical_timestamp,
    &expiration,
);

let attestation = contract.get_attestation(&imported_id);
assert!(attestation.imported);
assert_eq!(attestation.timestamp, historical_timestamp);
```

### Bridge Cross-Chain Attestations

Use this when a trusted bridge contract is mirroring an attestation that was
verified on another chain. The bridge contract becomes the on-chain attestation
creator, while the original source is preserved on the record.

```rust
let bridged_id = contract.bridge_attestation(
    &bridge_contract_address,
    &user_address,
    &String::from_str(&env, "KYC_PASSED"),
    &String::from_str(&env, "ethereum"),
    &String::from_str(&env, "0xabc123"),
);

let attestation = contract.get_attestation(&bridged_id);
assert!(attestation.bridged);
assert_eq!(attestation.source_chain, Some(String::from_str(&env, "ethereum")));
assert_eq!(attestation.source_tx, Some(String::from_str(&env, "0xabc123")));
```

### Verify Claims

```rust
// Check if user has valid KYC
let has_kyc = contract.has_valid_claim(
    &user_address,
    &String::from_str(&env, "KYC_PASSED")
);

if has_kyc {
    // Proceed with financial operation
}

// Check if user has valid KYC from a specific issuer
let has_specific_kyc = contract.has_valid_claim_from_issuer(
    &user_address,
    &String::from_str(&env, "KYC_PASSED"),
    &specific_issuer_address
);
```

### Verify Any of Multiple Claims

`has_any_claim(env: Env, subject: Address, claim_types: Vec<String>) -> bool`

| Parameter     | Type          | Description                                      |
|---------------|---------------|--------------------------------------------------|
| `env`         | `Env`         | Soroban environment (ledger time, storage)       |
| `subject`     | `Address`     | The address whose attestations are queried       |
| `claim_types` | `Vec<String>` | One or more claim type identifiers to check      |

Returns `true` if the subject holds at least one valid attestation matching any of the listed claim types; `false` otherwise.

**Behavior:**
- Uses OR-logic — returns `true` on the first valid match found (short-circuit evaluation)
- An empty `claim_types` list always returns `false`
- Revoked, expired, and pending attestations are excluded from matching

```rust
// Check if user has either KYC or an accredited investor credential
let claim_types = vec![
    &env,
    String::from_str(&env, "KYC_PASSED"),
    String::from_str(&env, "ACCREDITED_INVESTOR"),
    String::from_str(&env, "MERCHANT_VERIFIED"),
];
let has_any = contract.has_any_claim(&user_address, &claim_types);

if has_any {
    // Proceed — user satisfies at least one required credential
}
```

**Relationship to `has_valid_claim`:** Calling `has_any_claim` with a single-element list is equivalent to calling `has_valid_claim` with that same claim type. Use `has_valid_claim` when checking a single claim type, and `has_any_claim` when OR-logic across multiple claim types is needed.

### Verify All of Multiple Claims

`has_all_claims(env: Env, subject: Address, claim_types: Vec<String>) -> bool`

| Parameter     | Type          | Description                                      |
|---------------|---------------|--------------------------------------------------|
| `env`         | `Env`         | Soroban environment (ledger time, storage)       |
| `subject`     | `Address`     | The address whose attestations are queried       |
| `claim_types` | `Vec<String>` | All claim type identifiers that must be valid    |

Returns `true` only if the subject holds a valid attestation for **every** claim type in the list; `false` as soon as any one is missing, revoked, expired, or pending.

**Behavior:**
- Uses AND-logic — short-circuits and returns `false` on the first unsatisfied claim type
- An empty `claim_types` list always returns `true` (vacuous truth)
- Revoked, expired, and pending attestations are excluded from matching

```rust
// Require the user to hold ALL three credentials before proceeding
let mut required = soroban_sdk::Vec::new(&env);
required.push_back(String::from_str(&env, "KYC_PASSED"));
required.push_back(String::from_str(&env, "ACCREDITED_INVESTOR"));
required.push_back(String::from_str(&env, "AML_CLEARED"));

let fully_verified = trustlink.has_all_claims(&user_address, &required);

if fully_verified {
    // All credentials present and valid — proceed with restricted operation
} else {
    // At least one credential is missing, revoked, or expired
    return Err(Error::InsufficientCredentials);
}
```

**Relationship to `has_any_claim`:** `has_any_claim` uses OR-logic (at least one match), while `has_all_claims` uses AND-logic (every claim must match). Use `has_all_claims` when a workflow requires a complete set of credentials, such as high-value lending that demands both KYC and AML clearance.

### Revoke Attestations

```rust
// Issuer revokes an attestation
contract.revoke_attestation(&issuer, &attestation_id);
```

### Expiration Hooks

Subjects can register a callback contract to be notified when one of their attestations is approaching expiry. This lets wallets, dApps, or automation contracts react before a credential lapses.

**Flow:**
1. Subject calls `register_expiration_hook` with their callback contract address and how many days before expiry they want to be notified.
2. Whenever `has_valid_claim` is called and a matching attestation is inside the notification window, TrustLink emits an `exp_hook` event and calls `notify_expiring` on the callback contract.
3. If the callback call fails for any reason, the failure is silently swallowed — the main `has_valid_claim` result is unaffected.
4. Subject can overwrite or remove their hook at any time.

**Callback interface** — your contract must implement:
```rust
fn notify_expiring(env: Env, subject: Address, attestation_id: String, expiration: u64);
```

**Usage:**
```rust
// Register: notify me 7 days before any attestation expires
contract.register_expiration_hook(
    &subject,
    &my_callback_contract,
    &7,
);

// Retrieve the current hook
let hook = contract.get_expiration_hook(&subject);

// Remove the hook
contract.remove_expiration_hook(&subject);
```

**Event emitted when hook fires:**
```
topics: ["exp_hook", subject_address]
data:   (attestation_id, expiration_timestamp)
```

**Notes:**
- Only the subject can register or remove their own hook (requires auth).
- Attestations without an expiration never trigger the hook.
- A subject can only have one hook at a time; re-registering overwrites the previous one.
- Failed callback calls do not revert or affect the caller.

### Multi-Sig Attestations

High-value claims (e.g. `ACCREDITED_INVESTOR`) can require M-of-N registered issuers to co-sign before the attestation becomes active. This prevents a single compromised issuer from unilaterally issuing sensitive credentials.

**Flow:**
1. A registered issuer calls `propose_attestation` — they automatically count as the first signer.
2. Other required issuers call `cosign_attestation` with the returned `proposal_id`.
3. Once the number of signatures reaches `threshold`, the attestation is finalized and stored as a normal active attestation.
4. Proposals expire after 7 days if the threshold is not reached.

```rust
// Build the required-signers list (all must be registered issuers)
let mut required_signers = soroban_sdk::Vec::new(&env);
required_signers.push_back(issuer_a.clone());
required_signers.push_back(issuer_b.clone());
required_signers.push_back(issuer_c.clone());

// Propose a 2-of-3 multi-sig attestation
let proposal_id = contract.propose_attestation(
    &issuer_a,                                          // proposer (auto-signs)
    &user_address,                                      // subject
    &String::from_str(&env, "ACCREDITED_INVESTOR"),     // claim type
    &required_signers,                                  // all required signers
    &2,                                                 // threshold
);

// issuer_b co-signs — threshold reached, attestation activated
contract.cosign_attestation(&issuer_b, &proposal_id);

assert!(contract.has_valid_claim(&user_address, &String::from_str(&env, "ACCREDITED_INVESTOR")));
```

**Inspect a proposal:**
```rust
let proposal = contract.get_multisig_proposal(&proposal_id);
// proposal.signers     — addresses that have signed so far
// proposal.threshold   — required number of signatures
// proposal.finalized   — true once the attestation is active
// proposal.expires_at  — unix timestamp after which cosigning is rejected
```

**Error cases:**
- `InvalidThreshold` — threshold is 0 or exceeds the number of required signers
- `Unauthorized` — proposer or a required signer is not a registered issuer
- `NotRequiredSigner` — cosigner is not in the proposal's required-signers list
- `AlreadySigned` — the issuer has already co-signed this proposal
- `ProposalFinalized` — the proposal has already been activated
- `ProposalExpired` — the 7-day window has passed without reaching threshold

**Events emitted:**
```
topics: ["ms_prop", subject_address]   data: (proposal_id, proposer, threshold)
topics: ["ms_sign", signer_address]    data: (proposal_id, signatures_so_far, threshold)
topics: ["ms_actv"]                    data: (proposal_id, attestation_id)
```

### Query Attestations

```rust
// Get specific attestation
let attestation = contract.get_attestation(&attestation_id);

// Check status
let status = contract.get_attestation_status(&attestation_id);
// Returns: Valid, Expired, or Revoked

// Find the most recent valid attestation by subject + claim type
let attestation = contract.get_attestation_by_type(&user_address, &String::from_str(&env, "KYC_PASSED"));

// Count queries — returns total count, no pagination needed
let total = contract.get_subject_attestation_count(&user_address); // all attestations (incl. revoked/expired)
let issued = contract.get_issuer_attestation_count(&issuer_address); // all issued by this issuer
let valid  = contract.get_valid_claim_count(&user_address);          // only non-revoked, non-expired

// List user's attestations (paginated)
let attestations = contract.get_subject_attestations(&user_address, &0, &10);

// List issuer's attestations
let issued = contract.get_issuer_attestations(&issuer_address, &0, &10);
```

### Issuer Activity Stats

Every registered issuer has an `IssuerStats` record that is updated atomically alongside each attestation operation. Consumers can use these metrics to assess issuer trustworthiness before relying on their attestations.

```rust
let stats = contract.get_issuer_stats(&issuer_address);
// stats.total_issued   — total attestations ever created by this issuer
// stats.total_revoked  — total attestations revoked by this issuer
// stats.registered_at  — ledger timestamp when the issuer was first registered
```

The revocation rate (`total_revoked as f64 / total_issued as f64`) is a useful signal:
- A high rate may indicate an issuer that frequently issues incorrect or fraudulent attestations.
- A low rate on a high-volume issuer is a positive trustworthiness signal.
- `registered_at` lets consumers weight newer issuers differently from long-standing ones.

Stats for an address that has never been registered return zeroed defaults (`total_issued: 0`, `total_revoked: 0`, `registered_at: 0`).

## Integration Example

Here's how another contract would verify attestations:

```rust
use soroban_sdk::{contract, contractimpl, Address, Env, String};

#[contract]
pub struct LendingContract;

#[contractimpl]
impl LendingContract {
    pub fn borrow(
        env: Env,
        borrower: Address,
        trustlink_contract: Address,
        amount: i128
    ) -> Result<(), Error> {
        borrower.require_auth();
        
        // Create client for TrustLink contract
        let trustlink = trustlink::Client::new(&env, &trustlink_contract);
        
        // Verify borrower has valid KYC
        let kyc_claim = String::from_str(&env, "KYC_PASSED");
        let has_kyc = trustlink.has_valid_claim(&borrower, &kyc_claim);
        
        if !has_kyc {
            return Err(Error::KYCRequired);
        }
        
        // Proceed with lending logic
        // ...
        
        Ok(())
    }
}
```

## Error Handling

TrustLink defines clear error types:

- `AlreadyInitialized`: Contract already initialized
- `NotInitialized`: Contract not yet initialized
- `Unauthorized`: Caller lacks required permissions
- `NotFound`: Attestation doesn't exist
- `DuplicateAttestation`: Attestation with same hash already exists
- `AlreadyRevoked`: Attestation already revoked
- `Expired`: Attestation has expired
- `InvalidThreshold`: Multi-sig threshold is 0 or exceeds signer count
- `NotRequiredSigner`: Cosigner is not in the proposal's required-signers list
- `AlreadySigned`: Issuer has already co-signed the proposal
- `ProposalFinalized`: Proposal has already been activated into an attestation
- `ProposalExpired`: Proposal window (7 days) elapsed without reaching threshold

## Events

TrustLink emits events for off-chain indexing:

**AttestationCreated:**
```rust
topics: ["created", subject_address]
data: (attestation_id, issuer, claim_type, timestamp)
```

**AttestationRevoked:**
```rust
topics: ["revoked", issuer_address]
data: attestation_id
```

**AttestationRenewed:**
```rust
topics: ["renewed", issuer_address]
data: (attestation_id, new_expiration)
```

**IssuerRegistered:**
```rust
topics: ["iss_reg", issuer_address]
data: (admin_address, timestamp)
```

**IssuerRemoved:**
```rust
topics: ["iss_rem", issuer_address]
data: (admin_address, timestamp)
```

**ClaimTypeRegistered:**
```rust
topics: ["clmtype"]
data: (claim_type, description)
```

## Building and Testing

### Prerequisites

- Rust 1.70+
- Soroban CLI
- wasm32-unknown-unknown target

### Commands

```bash
# Run tests
make test

# Build contract
make build

# Build optimized version
make optimize

# Clean artifacts
make clean

# Format code
make fmt

# Run linter
make clippy
```

### Running Tests

```bash
cargo test
```

### Build Verification

To verify the WASM build target compiles correctly for Stellar deployment:

```bash
# Build for wasm32-unknown-unknown target
cargo build --target wasm32-unknown-unknown --release

# Verify the WASM artifact exists
ls -la target/wasm32-unknown-unknown/release/trustlink.wasm

# Validate the WASM binary (requires wasm-tools)
cargo install wasm-tools --locked
wasm-tools validate target/wasm32-unknown-unknown/release/trustlink.wasm
```

Or use the Makefile target:

```bash
make build
```

**Build Verification Criteria:**
- ✅ Build exits with code 0
- ✅ `trustlink.wasm` artifact exists in `target/wasm32-unknown-unknown/release/`
- ✅ WASM file size is reasonable (< 100KB after optimization)
- ✅ No std dependency errors (`#![no_std]` is respected)
- ✅ WASM binary is valid and can be inspected with wasm-objdump

Tests cover:
- Initialization and admin management
- Issuer registration and removal
- Attestation creation with validation
- Duplicate prevention
- Revocation logic
- Expiration handling
- Authorization enforcement
- Pagination
- Cross-contract verification

## Security Considerations

1. **Authorization**: Only admin can manage issuers; only issuers can create attestations
2. **Deterministic IDs**: Prevents replay attacks and ensures uniqueness
3. **Immutable History**: Attestations are never deleted, only marked as revoked
4. **Time-based Expiration**: Automatic invalidation of expired claims
5. **Event Transparency**: All changes are logged for auditability

## Use Cases

- **DeFi Protocols**: Verify KYC before lending/borrowing
- **Token Sales**: Ensure accredited investor status
- **Payment Systems**: Verify merchant credentials
- **Governance**: Validate voter eligibility
- **Marketplaces**: Confirm seller reputation
- **Insurance**: Verify policyholder identity

## Deployment

```bash
# Build optimized contract
make optimize

# Deploy to network
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/trustlink.wasm \
  --network testnet

# Initialize
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- initialize \
  --admin <ADMIN_ADDRESS>
```

## Video Tutorial

New to TrustLink or Soroban? Watch the [TrustLink Video Tutorial](https://www.youtube.com/watch?v=TODO_REPLACE_WITH_VIDEO_ID) for a 10–15 minute walkthrough covering what TrustLink is, how to deploy it, and how to integrate it into your contracts and frontend.

A companion written guide with all commands and code snippets is available at [docs/video-tutorial-guide.md](docs/video-tutorial-guide.md).

## Integration Guide

For a step-by-step walkthrough covering Rust cross-contract patterns, JavaScript/TypeScript usage, error handling, and testnet testing, see [docs/integration-guide.md](docs/integration-guide.md).

## Storage Layout

For a full reference of every on-chain storage key, the data each holds, TTL policy, serialization format, and a practical RPC read example for indexer developers, see [docs/storage-layout.md](docs/storage-layout.md).

## FAQ

**How do I verify an attestation in my contract?**

Use `has_valid_claim` for a simple boolean check, or `get_attestation` if you need the full record. The client is generated from the TrustLink contract ABI — see the [Integration Example](#integration-example) above for a complete pattern.

```rust
let trustlink = TrustLinkContractClient::new(&env, &trustlink_contract_id);

// Boolean — cheapest option for a gate check
if !trustlink.has_valid_claim(&user, &String::from_str(&env, "KYC_PASSED")) {
    return Err(Error::KYCRequired);
}

// Full record — use when you need issuer, timestamp, metadata, etc.
let attestation = trustlink.get_attestation(&attestation_id);
```

For issuer-specific trust, use `has_valid_claim_from_issuer`. For multi-claim gates, use `has_any_claim` (OR) or `has_all_claims` (AND).

---

**What happens when an issuer is removed?**

Removing an issuer with `remove_issuer` deregisters them from the issuer registry — they can no longer call `create_attestation` or `revoke_attestation`. However, all attestations they previously issued remain on-chain and continue to be queryable and valid (subject to their own expiration/revocation status). Existing attestations are never deleted when an issuer is removed.

If you need to invalidate all attestations from a removed issuer, you must revoke them individually before removing the issuer, while they still have the authority to do so.

---

**Can a subject dispute an attestation?**

Not directly — TrustLink is an issuer-controlled system. Only the issuer who created an attestation can revoke it. Subjects have no on-chain mechanism to dispute or remove an attestation about themselves.

If your use case requires subject consent or dispute resolution, the recommended pattern is to build a wrapper contract that enforces subject approval before calling `create_attestation`, or to use the multi-sig flow (`propose_attestation` / `cosign_attestation`) where the subject's address can be included as a required co-signer.

---

**How are attestation IDs generated?**

IDs are a 32-byte SHA-256 hash, hex-encoded to a 64-character string. The input is the XDR serialization of four fields concatenated together:

```
SHA-256( issuer_xdr || subject_xdr || claim_type_xdr || timestamp_xdr )
```

This makes IDs deterministic and collision-resistant. Two attestations with the same issuer, subject, claim type, and ledger timestamp will produce the same ID — which is why `create_attestation` rejects duplicates with `DuplicateAttestation`. Bridge attestations use a different scheme that also folds in `source_chain` and `source_tx`. See [`src/types.rs`](src/types.rs) — `Attestation::generate_id` and `Attestation::generate_bridge_id`.

---

**What is the cost of creating an attestation?**

By default fees are disabled (`attestation_fee: 0`). The admin can enable them with `set_fee`, specifying a token contract address, a fee amount, and a collector address. When enabled, `create_attestation` transfers exactly `attestation_fee` tokens from the issuer to the collector before storing the attestation — if the issuer's balance is insufficient the transaction fails.

```rust
// Check current fee config before issuing
let config = trustlink.get_fee_config();
// config.attestation_fee — amount in token's smallest unit (0 = free)
// config.fee_token       — None when fees are disabled
// config.fee_collector   — address that receives the fee
```

In addition to the application-level fee, every Soroban transaction pays the standard Stellar network fee (base fee + resource fee). The resource fee depends on the ledger entries read/written; a typical `create_attestation` call touches 3–4 persistent entries.

---

**How do I handle expired attestations?**

`has_valid_claim` and `get_attestation_status` both treat expired attestations as invalid automatically — you don't need to check expiration manually. When an expired attestation is encountered during a `has_valid_claim` call, a `["expired", subject]` event is emitted so off-chain indexers can react.

```rust
// Status check — returns Valid, Expired, Revoked, or Pending
let status = trustlink.get_attestation_status(&attestation_id);

match status {
    AttestationStatus::Expired => {
        // Ask the issuer to renew or re-issue
    }
    AttestationStatus::Valid => { /* proceed */ }
    _ => { /* handle revoked / pending */ }
}
```

To renew an expired attestation, the original issuer calls `renew_attestation` with a new expiration timestamp. Alternatively, the issuer can create a fresh attestation — the old one stays on-chain as a historical record.

## License

MIT

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for a history of notable changes.

## Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for setup instructions, code style requirements, and the PR process.

## Support

For issues or questions, please open a GitHub issue.
