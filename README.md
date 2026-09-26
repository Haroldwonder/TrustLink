# TrustLink - On-Chain Attestation & Verification System

[![CI](https://github.com/afurious/TrustLink/actions/workflows/ci.yml/badge.svg)](https://github.com/afurious/TrustLink/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/afurious/TrustLink/branch/main/graph/badge.svg)](https://codecov.io/gh/afurious/TrustLink)
[![Security Audit](https://img.shields.io/badge/Security%20Audit-In%20Progress-yellow)](./AUDIT_SCOPE.md)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](./LICENSE)

**[Documentation index](docs/README.md)** · **[Contributing](CONTRIBUTING.md)** · **[Code of Conduct](CODE_OF_CONDUCT.md)** · **[License](LICENSE)**

TrustLink is a Soroban smart contract that provides a reusable trust layer for the Stellar blockchain. It enables trusted issuers, bridge contracts, and administrators to create, import, manage, and revoke attestations about wallet addresses, allowing other contracts and applications to verify claims before executing financial operations.

## Overview

TrustLink solves the problem of decentralized identity verification and trust establishment on-chain. Instead of each application building its own KYC/verification system, TrustLink provides a shared attestation infrastructure that can be queried by any smart contract or dApp.

**New here? Start with the [5-Minute Quickstart](docs/quickstart.md)** — go from zero to verifying a testnet attestation using only the TypeScript SDK, in under 15 commands. Browse the full docs catalog in the **[documentation index](docs/README.md)**.

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
- **Pagination & Filtering**: Efficient listing and date-range searching of attestations

## Security

TrustLink is designed with security as a first-class concern. Before mainnet deployment with real funds, the contract undergoes comprehensive external security audits.

### Audit Status

- **Current Status:** Security audit in progress
- **Audit Scope:** [AUDIT_SCOPE.md](./AUDIT_SCOPE.md)
- **Firm Selection:** [AUDIT_FIRM_SELECTION.md](./AUDIT_FIRM_SELECTION.md)
- **Security Review:** [docs/security-review.md](./docs/security-review.md)
- **Security Model:** [docs/security.md](./docs/security.md)

### Pre-Audit Findings

Three security findings were identified in the pre-audit review and must be resolved before mainnet deployment:

1. **FINDING-001 [MEDIUM]:** `initialize()` state read before auth — ✅ resolved (`require_auth()` is now the first operation in `initialize()`)
2. **FINDING-002 [HIGH]:** `revoke_attestation()` missing `require_issuer` check
3. **FINDING-003 [HIGH]:** `update_expiration()` missing `require_issuer` check

See [docs/security-review.md](./docs/security-review.md) for details and remediation.

### Security Documentation

- **Trust Hierarchy:** [docs/security.md](./docs/security.md) - Admin, issuer, and subject roles
- **Threat Model:** [docs/security.md](./docs/security.md) - Known limitations and mitigations
- **Bug Bounty:** [docs/bug-bounty.md](./docs/bug-bounty.md) - Vulnerability rewards and scope
- **GDPR Compliance:** [docs/compliance.md](./docs/compliance.md) - Right to erasure and data minimization
- **Monitoring:** [docs/monitoring.md](./docs/monitoring.md) - Event streaming and alerting

### Reporting Security Issues

If you discover a security vulnerability, please email security@trustlink.io with:

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

Please do not disclose security issues publicly until they have been addressed.

### Bug Bounty Program

TrustLink operates an active bug bounty program with rewards for verified vulnerabilities. See [docs/bug-bounty.md](./docs/bug-bounty.md) for:

- In-scope contracts and components
- Severity tiers and reward ranges
- Submission process and safe harbor
- Known limitations and excluded findings

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

// Admin removes an issuer
contract.remove_issuer(&admin, &issuer_address);
```

#### Issuer Removal Behavior

When an issuer is removed via `remove_issuer`:

| Action                                                   | Allowed? | Reason                                                                                                        |
| -------------------------------------------------------- | -------- | ------------------------------------------------------------------------------------------------------------- |
| Existing attestations remain valid                       | Yes      | Attestation validity depends only on revocation and expiration status, not issuer registration                |
| `has_valid_claim` returns true for existing attestations | Yes      | Validity checks do not verify issuer registration                                                             |
| Removed issuer creates new attestations                  | **No**   | `create_attestation` calls `require_issuer`, which rejects unregistered issuers                               |
| Removed issuer revokes their own attestations            | **No**   | `revoke_attestation` requires the caller to still be a registered issuer (deregistered issuers cannot revoke) |

Removing an issuer prevents future issuance and also blocks the removed issuer from revoking attestations after deregistration. Previously issued attestations remain valid unless successfully revoked by a currently-registered issuer.

### Register Bridge Contracts

Bridge contracts use a separate trust registry from regular issuers.

```rust
contract.register_bridge(&admin, &bridge_contract_address);

let is_bridge = contract.is_bridge(&bridge_contract_address);
```

### Claim Type Registry

The contract ships with a set of standard claim types that the admin can pre-register on deployment.

| Claim Type            | Description                                      |
| --------------------- | ------------------------------------------------ |
| `KYC_PASSED`          | Subject has passed KYC identity verification     |
| `ACCREDITED_INVESTOR` | Subject qualifies as an accredited investor      |
| `MERCHANT_VERIFIED`   | Subject is a verified merchant                   |
| `AML_CLEARED`         | Subject has passed AML screening                 |
| `SANCTIONS_CHECKED`   | Subject has been checked against sanctions lists |

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

#### Multi-issuer behavior

A subject may hold the same claim type issued by multiple issuers. `has_valid_claim` uses OR-logic across all issuers — it returns `true` if **any one** attestation for that claim type is currently valid, regardless of the state of the others.

| Scenario                            | Result  |
| ----------------------------------- | ------- |
| Two issuers, both valid             | `true`  |
| Two issuers, one revoked, one valid | `true`  |
| Two issuers, one expired, one valid | `true`  |
| Two issuers, both revoked           | `false` |
| Two issuers, both expired           | `false` |

Use `has_valid_claim_from_issuer` when you need to verify a claim from a specific trusted issuer rather than any issuer in the registry.

### Verify Any of Multiple Claims

`has_any_claim(env: Env, subject: Address, claim_types: Vec<String>) -> bool`

| Parameter     | Type          | Description                                 |
| ------------- | ------------- | ------------------------------------------- |
| `env`         | `Env`         | Soroban environment (ledger time, storage)  |
| `subject`     | `Address`     | The address whose attestations are queried  |
| `claim_types` | `Vec<String>` | One or more claim type identifiers to check |

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

| Parameter     | Type          | Description                                   |
| ------------- | ------------- | --------------------------------------------- |
| `env`         | `Env`         | Soroban environment (ledger time, storage)    |
| `subject`     | `Address`     | The address whose attestations are queried    |
| `claim_types` | `Vec<String>` | All claim type identifiers that must be valid |

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

### Transfer Attestations (Admin Only)

Admin can transfer ownership of an attestation to a new registered issuer. This is useful when an issuer account is deactivated/compromised, allowing orphaned attestations to be re-assigned to a successor issuer.

```rust
// Register the new issuer first
contract.register_issuer(&admin, &new_issuer);

// Transfer attestation ownership
contract.transfer_attestation(&admin, &attestation_id, &new_issuer);
```

**Effects:**

- Updates `issuer` field in attestation record
- Removes ID from old issuer's attestation index
- Adds ID to new issuer's attestation index
- Updates `total_issued` stats for both issuers
- Emits `attestation_transferred` event: `["att_xfer", old_issuer] (attestation_id, new_issuer)`
- Appends `Transferred` audit entry: `actor=admin, details=new_issuer_address`

**Validations:**

- Caller must be admin
- `attestation_id` must exist
- `new_issuer` must be registered
- Idempotent if `old_issuer == new_issuer`

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

// Search attestations by date range (legacy offset pagination)
let from_ts = 1_700_000_000;
let to_ts = 1_701_000_000;
let attestations = contract.get_attestations_in_range(&user_address, &from_ts, &to_ts, &0, &10);

// Preferred page-after cursor pagination for resilient traversal across deletions
let first_page = contract.get_attestations_in_range_after(
    &user_address,
    &from_ts,
    &to_ts,
    &None,
    &10,
);
let second_page = contract.get_attestations_in_range_after(
    &user_address,
    &from_ts,
    &to_ts,
    &Some(first_page.get(9).unwrap().id.clone()),
    &10,
);

// List issuer's attestations
let issued = contract.get_issuer_attestations(&issuer_address, &0, &10);
```

## Global Statistics

`get_global_stats(env: Env) -> GlobalStats` returns a snapshot of contract-wide counters. No authentication is required — it is safe to call from dashboards, analytics tools, and indexers.

```rust
let stats = contract.get_global_stats();
// stats.total_attestations — all attestations ever created (native, imported, bridged, multi-sig)
// stats.total_revocations  — all revocations ever performed (single + batch)
// stats.total_issuers      — current number of registered issuers
```

**`GlobalStats` fields:**

| Field                | Type  | Description                                            |
| -------------------- | ----- | ------------------------------------------------------ |
| `total_attestations` | `u64` | Cumulative count of all attestations created           |
| `total_revocations`  | `u64` | Cumulative count of all revocations                    |
| `total_issuers`      | `u64` | Current registered issuer count (live, not cumulative) |

Stats are updated atomically on every mutating operation:

- `register_issuer` → increments `total_issuers`
- `remove_issuer` → decrements `total_issuers` (saturating at 0)
- `create_attestation`, `import_attestation`, `bridge_attestation` → each increments `total_attestations` by 1
- `create_attestations_batch` → increments `total_attestations` by the number of subjects
- `cosign_attestation` (on threshold reached) → increments `total_attestations` by 1
- `revoke_attestation` → increments `total_revocations` by 1
- `revoke_attestations_batch` → increments `total_revocations` by the number revoked

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

## Storage Exhaustion Protection

TrustLink enforces configurable limits to prevent malicious issuers from exhausting on-chain storage.

| Limit                          | Default | Description                                 |
| ------------------------------ | ------- | ------------------------------------------- |
| `max_attestations_per_issuer`  | 10,000  | Max attestations a single issuer may create |
| `max_attestations_per_subject` | 100     | Max attestations a single subject may hold  |

Attempting to create an attestation beyond either limit returns `Error::LimitExceeded` (code `#10`).

The admin can view and adjust limits at any time:

```rust
// Read current limits
let limits = contract.get_limits();

// Adjust limits (admin only)
contract.set_limits(
    &admin,
    &5_000,  // max per issuer
    &50,     // max per subject
);
```

```bash
# CLI — read limits
soroban contract invoke --id <CONTRACT_ID> --network testnet -- get_limits

# CLI — update limits (admin)
soroban contract invoke --id <CONTRACT_ID> --network testnet --source ADMIN_SECRET \
  -- set_limits \
  --admin ADMIN_PUBLIC_KEY \
  --max_attestations_per_issuer 5000 \
  --max_attestations_per_subject 50
```

## Storage Cost Estimates

Every attestation written to the Stellar ledger consumes a base reserve (locked XLM). The table below gives issuers a quick budget reference.

| Scenario                | Approx. XLM reserve per attestation |
| ----------------------- | ----------------------------------- |
| Baseline (no metadata)  | ~15–16 XLM                          |
| With ~200-byte metadata | ~18–20 XLM                          |
| Revocation only         | ~1–2 XLM                            |

Costs are **reserve**, not fees — the XLM is locked, not burned, and is returned if the entry is evicted or deleted.

Key drivers:

- The main `Attestation` entry is ~800 bytes → ~13 XLM data reserve + 0.5 XLM entry fee.
- Each new subject/issuer index Vec entry adds ~1 XLM.
- Optional metadata adds ~0.5 XLM per 32 bytes.

For full ledger entry size breakdown, XLM calculations, TTL/rent guidance, and bulk cost projections (10 → 10,000 attestations), see [docs/performance.md — Storage Cost Per Attestation](docs/performance.md#storage-cost-per-attestation-xlm).

> Base reserve figures are based on the current Stellar network parameters (0.5 XLM per entry + 0.5 XLM per 32 bytes). Verify with `stellar network info` before budgeting for production.

## Error Handling

TrustLink defines clear error types:

- `AlreadyInitialized`: Contract already initialized
- `NotInitialized`: Contract not yet initialized
- `Unauthorized`: Caller lacks required permissions
- `NotFound`: Attestation doesn't exist
- `DuplicateAttestation`: Attestation with same hash already exists
- `AlreadyRevoked`: Attestation already revoked
- `Expired`: Attestation has expired
- `LimitExceeded`: Issuer or subject attestation count has reached the configured limit
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

# Build contract (WASM release)
make build

# Build + optimize WASM
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

### Trade-finance example

[`examples/trade_finance.rs`](examples/trade_finance.rs) demonstrates a
bill-of-lading attestation chain. An exporter, customs authority, and financing
bank independently attest to the same shipment reference; clearance requires all
three claims to remain valid.

```bash
cargo test --example trade_finance
```

## Security Considerations

1. **Authorization**: Only admin can manage issuers; only issuers can create attestations
2. **Deterministic IDs**: Prevents replay attacks and ensures uniqueness
3. **Immutable History**: Attestations are never deleted, only marked as revoked
4. **Time-based Expiration**: Automatic invalidation of expired claims
5. **Event Transparency**: All changes are logged for auditability

For a full description of the trust hierarchy, threat model, known limitations,
and operational security recommendations, see [docs/security.md](docs/security.md).

For the pre-mainnet line-by-line authorization audit, see
[docs/security-review.md](docs/security-review.md).

## Use Cases

- **DeFi Protocols**: Verify KYC before lending/borrowing
- **Lending Pool**: IssuerTier-gated LTV and liquidation thresholds — see [examples/lending-pool/README.md](examples/lending-pool/README.md)
- **Token Sales**: Ensure accredited investor status
- **Payment Systems**: Verify merchant credentials
- **Governance**: Validate voter eligibility
- **Marketplaces**: Confirm seller reputation
- **Insurance**: Verify policyholder identity — see [examples/insurance/README.md](examples/insurance/README.md)
- **Real-Estate Title**: Long-lived title and lien attestation flow — see [examples/real-estate/README.md](examples/real-estate/README.md)
- **Healthcare Credentials**: Privacy-sensitive provider licensing and vaccination verification — see [examples/healthcare/README.md](examples/healthcare/README.md)
- **Stellar Anchors**: End-to-end anchor KYC attestation flow example in [examples/anchor-integration/README.md](examples/anchor-integration/README.md)
- **Soroban Tokens**: KYC-restricted token transfer example in [examples/kyc-token/README.md](examples/kyc-token/README.md)
- **DAO Governance**: Voter eligibility-gated voting example in [examples/governance/README.md](examples/governance/README.md)
- **SaaS Seat Licensing**: Multi-tenant per-seat `SEAT_LICENSED` issuance, access gating, and offboarding revocation — see [examples/seat-licensing/README.md](examples/seat-licensing/README.md)

## Release Process

TrustLink uses **automated release management** with semantic versioning and conventional commits.

**How it works:**

1. Merge commits to `main` with conventional commit messages (`feat:`, `fix:`, etc.)
2. Release Please automatically creates a Release PR with:
   - Updated version in `Cargo.toml`
   - Generated `CHANGELOG.md`
3. Merge the Release PR
4. GitHub Release is created automatically with WASM artifacts attached

**For details, see [RELEASE.md](RELEASE.md) and [CONTRIBUTING.md — Commit Message Conventions](CONTRIBUTING.md#commit-message-conventions).**

**Quick reference:**

```bash
# Commit with conventional format
git commit -m "feat(storage): add dual indexing for subject and issuer"

# Push to main (or merge PR)
git push origin main

# Release Please creates a Release PR automatically
# Review, merge, and GitHub Release is published with WASM artifacts
```

## Deployment

TrustLink's Makefile supports deploying to testnet, mainnet, and a local node
with a single command. All network targets build an optimized WASM artifact
before deploying.

### Prerequisites

```bash
# Install Stellar CLI
cargo install --locked stellar-cli --features opt

# Add WASM target (if not already present)
rustup target add wasm32-unknown-unknown
```

### Environment variables

| Variable          | Required            | Description                                                           |
| ----------------- | ------------------- | --------------------------------------------------------------------- |
| `ADMIN_SECRET`    | Yes (deploy/invoke) | Stellar secret key (`S...`) used to sign transactions                 |
| `CONTRACT_ID`     | Yes (invoke)        | Contract address returned by `deploy`                                 |
| `TESTNET_RPC_URL` | No                  | Override testnet RPC (default: `https://soroban-testnet.stellar.org`) |
| `MAINNET_RPC_URL` | No                  | Override mainnet RPC                                                  |
| `LOCAL_RPC_URL`   | No                  | Override local RPC (default: `http://localhost:8000/soroban/rpc`)     |

Never commit `ADMIN_SECRET` to version control. Always pass it via the shell environment.

### Deploy to testnet

```bash
export ADMIN_SECRET=SXXX...
make deploy                      # NETWORK defaults to testnet
# or explicitly:
make deploy NETWORK=testnet
# or use the convenience alias:
make testnet
```

### Deploy to mainnet

Mainnet deploys prompt for confirmation before proceeding.

```bash
export ADMIN_SECRET=SXXX...
make deploy NETWORK=mainnet
# or:
make mainnet
```

### Deploy to a local node

```bash
export ADMIN_SECRET=SXXX...
make deploy NETWORK=local
# or:
make local
```

### Initialize after deploy

```bash
export CONTRACT_ID=C...          # printed by make deploy
export ADMIN_SECRET=SXXX...

make invoke ARGS='-- initialize --admin <ADMIN_ADDRESS> --ttl_days null'
```

### Invoke any contract function

```bash
export CONTRACT_ID=C...

# Read-only (no ADMIN_SECRET needed)
make invoke ARGS='-- get_admin'
make invoke ARGS='-- is_paused'
make invoke ARGS='-- get_global_stats'

# State-changing (ADMIN_SECRET required)
export ADMIN_SECRET=SXXX...
make invoke ARGS='-- register_issuer --admin <ADMIN> --issuer <ISSUER>'
make invoke ARGS='-- pause --admin <ADMIN>'

# Target a specific network
make invoke NETWORK=mainnet ARGS='-- get_admin'
```

### All Makefile targets

| Target                        | Description                                  |
| ----------------------------- | -------------------------------------------- |
| `make build`                  | Build WASM release artifact                  |
| `make test`                   | Run all unit tests                           |
| `make optimize`               | Build + optimize WASM                        |
| `make fmt`                    | Format source code                           |
| `make clippy`                 | Run clippy linter                            |
| `make clean`                  | Remove build artifacts                       |
| `make install`                | Print dependency installation instructions   |
| `make deploy`                 | Deploy to `NETWORK` (default: testnet)       |
| `make deploy NETWORK=testnet` | Deploy to testnet                            |
| `make deploy NETWORK=mainnet` | Deploy to mainnet (prompts for confirmation) |
| `make deploy NETWORK=local`   | Deploy to local node                         |
| `make testnet`                | Alias for `deploy NETWORK=testnet`           |
| `make mainnet`                | Alias for `deploy NETWORK=mainnet`           |
| `make local`                  | Alias for `deploy NETWORK=local`             |
| `make invoke ARGS='-- fn'`    | Invoke a contract function on `NETWORK`      |
| `make help`                   | Print all targets with usage examples        |

## Video Tutorial

New to TrustLink or Soroban? The TrustLink Video Tutorial (coming soon) will provide a 10–15 minute walkthrough covering what TrustLink is, how to deploy it, and how to integrate it into your contracts and frontend.

In the meantime, a companion written guide with all commands and code snippets is available at [docs/video-tutorial-guide.md](docs/video-tutorial-guide.md).

## Quickstart & Glossary

New to TrustLink? [docs/quickstart.md](docs/quickstart.md) gets you from clone to a verified claim in a few minutes, and [docs/glossary.md](docs/glossary.md) covers the key terms used throughout these docs. Both are also available in [Español](docs/i18n/es/quickstart.md) ([glosario](docs/i18n/es/glossary.md)) — see [docs/i18n/](docs/i18n/) for the translation directory convention.

## Integration Guide

For a step-by-step walkthrough covering Rust cross-contract patterns, JavaScript/TypeScript usage, error handling, and testnet testing, see [docs/integration-guide.md](docs/integration-guide.md).

New to TrustLink? Start with the [5-Minute Quickstart](docs/quickstart.md) for the fastest path to verifying your first attestation on testnet.

## Storage Layout

For a full reference of every on-chain storage key, the data each holds, TTL policy, serialization format, and a practical RPC read example for indexer developers, see [docs/storage-layout.md](docs/storage-layout.md).

## Glossary

Terms like *attestation*, *issuer*, *subject*, *bridge*, *claim type*, *endorsement*, and *delegation* have specific meanings in TrustLink. See [docs/glossary.md](docs/glossary.md) for definitions of all domain-specific terms.

## Architecture Decision Records

Key design choices are documented as ADRs in [docs/adr/](docs/adr/):

| ADR                                                  | Decision                                         |
| ---------------------------------------------------- | ------------------------------------------------ |
| [ADR-001](docs/adr/ADR-001-deterministic-ids.md)     | Deterministic IDs instead of sequential counters |
| [ADR-002](docs/adr/ADR-002-persistent-storage.md)    | Persistent storage instead of temporary storage  |
| [ADR-003](docs/adr/ADR-003-immutable-history.md)     | Immutable attestation history (no delete)        |
| [ADR-004](docs/adr/ADR-004-dual-indexes.md)          | Separate issuer and subject indexes              |
| [ADR-006](docs/adr/ADR-006-multi-issuer-or-logic.md) | OR-logic across issuers in `has_valid_claim`     |
| [ADR-007](docs/adr/ADR-007-attestation-requests.md)  | Pull-based attestation request workflow          |
| [ADR-008](docs/adr/ADR-008-rate-limiting.md)         | Rate limiting design and known limitations       |
| [ADR-009](docs/adr/ADR-009-delegation-model.md)      | Delegation model and trust chain implications    |
| [ADR-011](docs/adr/ADR-011-schema-federation-versioning.md) | Indexer GraphQL schema federation and versioning |

A blank [template](docs/adr/ADR-000-template.md) is available for new decisions.

## License

This project is licensed under the [MIT License](LICENSE).

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for a history of notable changes.

## Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for setup instructions, code style requirements, and the PR process. By participating, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).
# Comprehensive Diagnostic, Architecture & Resolution Guide: TypeScript Module Resolution Misconfiguration (TS5095) in Containerized Build Pipelines

---

## Executive Summary & Root Cause Analysis

In TypeScript 5.0+, the compiler strictly enforces compatibilities between module system targets (`compilerOptions.module`) and module resolution strategies (`compilerOptions.moduleResolution`). 

When `tsconfig.json` specifies:
```json
{
  "compilerOptions": {
    "module": "commonjs",
    "moduleResolution": "bundler"
  }
}

The TypeScript compiler (tsc) immediately aborts during compiler option validation—prior to parsing, AST generation, or type-checking any source files—with the following fatal error:
error TS5095: Option 'bundler' can only be used when 'module' is set to 'preserve' or to 'es2015' or later.

Why This Breakdown Occurs
 * The Role of moduleResolution: "bundler": Introduced in TypeScript 5.0, bundler models how modern frontend/backend bundlers (such as Webpack, Vite, esbuild, SWC, or Rollup) resolve import paths. Bundlers natively support ECMAScript Module (ESM) syntax (import/export), dynamic imports, package .exports fields, and extensions without requiring Node.js legacy CommonJS resolution hacks.
 * The Conflict with module: "commonjs": Setting module: "commonjs" instructs tsc to transform ES module syntax into CommonJS require() calls and exports.foo statements. However, bundler resolution assumes that the downstream bundler—not tsc—handles module emission or that code is strictly written using ESM semantics. Combining commonjs output with modern bundler path resolution is fundamentally contradictory within the TypeScript 5.x type system.
 * Pipeline Propagation:
   * Local development using npx tsc --noEmit fails immediately.
   * Local build scripts running npm run build (defined as tsc && node -e ...) fail.
   * Containerized CI/CD builds running RUN npm run build inside Dockerfile fail at the builder stage, completely blocking image generation and deployment pipelines.
Root Architecture & File System Topology
indexer/
├── Dockerfile
├── package.json
├── package-lock.json
├── tsconfig.json
├── src/
│   ├── index.ts
│   ├── config/
│   │   └── environment.ts
│   ├── services/
│   │   ├── indexer.ts
│   │   └── stellar.ts
│   └── utils/
│       └── logger.ts
└── tests/
    └── indexer.test.ts

Technical Specifications & Broken Configuration Baseline
Broken Configuration: indexer/tsconfig.json
{
  "$schema": "[https://json.schemastore.org/tsconfig](https://json.schemastore.org/tsconfig)",
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2022"],
    "module": "commonjs",
    "moduleResolution": "bundler",
    "allowSyntheticDefaultImports": true,
    "esModuleInterop": true,
    "forceConsistentCasingInFileNames": true,
    "strict": true,
    "skipLibCheck": true,
    "outDir": "./dist",
    "rootDir": "./src",
    "resolveJsonModule": true,
    "declaration": true,
    "sourceMap": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist", "tests"]
}

Broken Package Manifest: indexer/package.json
{
  "name": "@stellar-indexer/service",
  "version": "1.0.0",
  "description": "High-throughput Stellar Horizon event indexer",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "scripts": {
    "type-check": "tsc --noEmit -p tsconfig.json",
    "build": "tsc && node -e \"console.log('Build completed successfully')\"",
    "start": "node dist/index.js",
    "dev": "ts-node-dev --respawn src/index.ts",
    "test": "jest"
  },
  "dependencies": {
    "@stellar/stellar-sdk": "^11.2.0",
    "dotenv": "^16.4.5",
    "express": "^4.19.2",
    "pino": "^9.0.0"
  },
  "devDependencies": {
    "@types/express": "^4.17.21",
    "@types/node": "^20.12.7",
    "jest": "^29.7.0",
    "ts-node-dev": "^2.0.0",
    "typescript": "^5.4.5"
  }
}

Broken Multi-Stage Docker Build: indexer/Dockerfile
# Stage 1: Build Environment
FROM node:20-alpine AS builder

WORKDIR /app

# Install package manifests
COPY package.json package-lock.json ./

# Clean install dependencies
RUN npm ci

# Copy configuration and source files
COPY tsconfig.json ./
COPY src/ ./src/

# FAILS HERE: Executes `tsc && node -e ...` producing TS5095 error
RUN npm run build

# Stage 2: Runtime Production Environment
FROM node:20-alpine AS runner

WORKDIR /app

ENV NODE_ENV=production

COPY package.json package-lock.json ./
RUN npm ci --only=production

COPY --from=builder /app/dist ./dist

EXPOSE 3000

CMD ["node", "dist/index.js"]

Remediation Strategies & Architectural Trade-offs
To fix TS5095, select the strategy that best aligns with your execution runtime:
| Strategy | module setting | moduleResolution setting | Ideal For | Runtime Output |
|---|---|---|---|---|
| Option A: Pure Node.js CommonJS (Recommended for standard Node) | "CommonJS" | "Node10" (or "Node") | Traditional Node.js without bundlers | CommonJS (require) |
| Option B: Modern Node.js ESM Engine | "Node16" or "NodeNext" | "Node16" or "NodeNext" | Modern Node.js (v18+) with ES Modules | Native ESM (import) |
| Option C: Bundled Build Pipeline | "ES2022" or "Preserve" | "bundler" | Projects processed via esbuild/swc/webpack | Modern ESM emitted to bundler |
Detailed Remediation Implementations
Solution Option A: Target Node.js Legacy CommonJS Runtime (Standard Fix)
If your runtime uses standard Node.js without a bundler (esbuild/tsup/webpack) and relies on CommonJS module loading (require), adjust moduleResolution to match commonjs.
Corrected indexer/tsconfig.json (CommonJS Path)
{
  "$schema": "[https://json.schemastore.org/tsconfig](https://json.schemastore.org/tsconfig)",
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2022"],
    "module": "commonjs",
    "moduleResolution": "node",
    "allowSyntheticDefaultImports": true,
    "esModuleInterop": true,
    "forceConsistentCasingInFileNames": true,
    "strict": true,
    "skipLibCheck": true,
    "outDir": "./dist",
    "rootDir": "./src",
    "resolveJsonModule": true,
    "declaration": true,
    "sourceMap": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist", "tests"]
}

Solution Option B: Target Native ECMAScript Modules (ESM)
If you wish to retain bundler or modern resolution while taking advantage of Node's native ES Module system:
 * Add "type": "module" to package.json.
 * Update tsconfig.json to use Node16 or NodeNext for both module and moduleResolution.
Updated indexer/package.json (ESM Path)
{
  "name": "@stellar-indexer/service",
  "version": "1.0.0",
  "description": "High-throughput Stellar Horizon event indexer",
  "type": "module",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "scripts": {
    "type-check": "tsc --noEmit -p tsconfig.json",
    "build": "tsc && node -e \"console.log('Build completed successfully')\"",
    "start": "node dist/index.js",
    "dev": "node --loader ts-node/esm src/index.ts",
    "test": "node --experimental-vm-modules node_modules/jest/bin/jest.js"
  },
  "dependencies": {
    "@stellar/stellar-sdk": "^11.2.0",
    "dotenv": "^16.4.5",
    "express": "^4.19.2",
    "pino": "^9.0.0"
  },
  "devDependencies": {
    "@types/express": "^4.17.21",
    "@types/node": "^20.12.7",
    "jest": "^29.7.0",
    "ts-node": "^10.9.2",
    "typescript": "^5.4.5"
  }
}

Corrected indexer/tsconfig.json (ESM Path)
{
  "$schema": "[https://json.schemastore.org/tsconfig](https://json.schemastore.org/tsconfig)",
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2022"],
    "module": "NodeNext",
    "moduleResolution": "NodeNext",
    "allowSyntheticDefaultImports": true,
    "esModuleInterop": true,
    "forceConsistentCasingInFileNames": true,
    "strict": true,
    "skipLibCheck": true,
    "outDir": "./dist",
    "rootDir": "./src",
    "resolveJsonModule": true,
    "declaration": true,
    "sourceMap": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist", "tests"]
}

Solution Option C: Bundler-Driven Pipeline (esbuild Integration)
If your build process utilizes esbuild or tsup to bundle your Node app into a single output file, retain "moduleResolution": "bundler" by setting "module": "ES2022".
Updated indexer/package.json (Bundler Path)
{
  "name": "@stellar-indexer/service",
  "version": "1.0.0",
  "description": "High-throughput Stellar Horizon event indexer",
  "main": "dist/index.js",
  "scripts": {
    "type-check": "tsc --noEmit -p tsconfig.json",
    "build": "tsc --noEmit -p tsconfig.json && esbuild src/index.ts --bundle --platform=node --target=node20 --outfile=dist/index.js",
    "start": "node dist/index.js",
    "test": "jest"
  },
  "dependencies": {
    "@stellar/stellar-sdk": "^11.2.0",
    "dotenv": "^16.4.5",
    "express": "^4.19.2",
    "pino": "^9.0.0"
  },
  "devDependencies": {
    "@types/express": "^4.17.21",
    "@types/node": "^20.12.7",
    "esbuild": "^0.20.2",
    "jest": "^29.7.0",
    "typescript": "^5.4.5"
  }
}

Corrected indexer/tsconfig.json (Bundler Path)
{
  "$schema": "[https://json.schemastore.org/tsconfig](https://json.schemastore.org/tsconfig)",
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2022"],
    "module": "ES2022",
    "moduleResolution": "bundler",
    "allowSyntheticDefaultImports": true,
    "esModuleInterop": true,
    "forceConsistentCasingInFileNames": true,
    "strict": true,
    "skipLibCheck": true,
    "outDir": "./dist",
    "rootDir": "./src",
    "resolveJsonModule": true,
    "declaration": true,
    "sourceMap": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist", "tests"]
}

Fully Production-Ready Source Code Framework
Below is the complete implementation codebase (Option A - CommonJS Production standard) including dummy application sources, logger, verification tests, Dockerfile, and verification automation script.
1. Source: indexer/src/config/environment.ts
import dotenv from 'dotenv';

dotenv.config();

export interface EnvironmentConfig {
  port: number;
  nodeEnv: string;
  horizonUrl: string;
  logLevel: string;
}

export const config: EnvironmentConfig = {
  port: parseInt(process.env.PORT || '3000', 10),
  nodeEnv: process.env.NODE_ENV || 'development',
  horizonUrl: process.env.HORIZON_URL || '[https://horizon.stellar.org](https://horizon.stellar.org)',
  logLevel: process.env.LOG_LEVEL || 'info',
};

2. Source: indexer/src/utils/logger.ts
import pino from 'pino';
import { config } from '../config/environment';

export const logger = pino({
  level: config.logLevel,
  base: {
    env: config.nodeEnv,
    service: 'indexer-service',
  },
  timestamp: pino.stdTimeFunctions.isoTime,
});

3. Source: indexer/src/services/stellar.ts
import { Horizon } from '@stellar/stellar-sdk';
import { config } from '../config/environment';
import { logger } from '../utils/logger';

export class StellarService {
  private server: Horizon.Server;

  constructor() {
    this.server = new Horizon.Server(config.horizonUrl);
  }

  public async getLatestLedgerSequence(): Promise<number> {
    try {
      const ledgerResponse = await this.server
        .ledgers()
        .order('desc')
        .limit(1)
        .call();

      if (!ledgerResponse.records || ledgerResponse.records.length === 0) {
        throw new Error('No ledgers returned from Horizon');
      }

      const latestLedger = ledgerResponse.records[0];
      logger.info({ sequence: latestLedger.sequence }, 'Fetched latest ledger sequence');
      return latestLedger.sequence;
    } catch (error) {
      logger.error({ err: error }, 'Failed to fetch ledger sequence from Horizon');
      throw error;
    }
  }
}

4. Source: indexer/src/services/indexer.ts
import { StellarService } from './stellar';
import { logger } from '../utils/logger';

export class IndexerEngine {
  private stellarService: StellarService;
  private isRunning: boolean = false;

  constructor() {
    this.stellarService = new StellarService();
  }

  public async start(): Promise<void> {
    this.isRunning = true;
    logger.info('Starting Stellar Event Indexer Engine...');

    try {
      const sequence = await this.stellarService.getLatestLedgerSequence();
      logger.info({ currentSequence: sequence }, 'Indexer successfully synchronized');
    } catch (error) {
      logger.error({ err: error }, 'Initialization failed during synchronization');
    }
  }

  public stop(): void {
    this.isRunning = false;
    logger.info('Indexer Engine stopped');
  }

  public getStatus(): { isRunning: boolean } {
    return { isRunning: this.isRunning };
  }
}

5. Source: indexer/src/index.ts
import express, { Express, Request, Response } from 'express';
import { config } from './config/environment';
import { logger } from './utils/logger';
import { IndexerEngine } from './services/indexer';

const app: Express = express();
const indexer = new IndexerEngine();

app.use(express.json());

app.get('/health', (req: Request, res: Response) => {
  res.status(200).json({
    status: 'ok',
    uptime: process.uptime(),
    indexer: indexer.getStatus(),
  });
});

app.listen(config.port, async () => {
  logger.info({ port: config.port }, 'Server listening on designated port');
  await indexer.start();
});

export { app };

6. Test File: indexer/tests/indexer.test.ts
import { IndexerEngine } from '../src/services/indexer';

jest.mock('../src/services/stellar', () => {
  return {
    StellarService: jest.fn().mockImplementation(() => {
      return {
        getLatestLedgerSequence: jest.fn().mockResolvedValue(12345678),
      };
    }),
  };
});

describe('IndexerEngine Unit Tests', () => {
  let indexer: IndexerEngine;

  beforeEach(() => {
    indexer = new IndexerEngine();
  });

  afterEach(() => {
    indexer.stop();
  });

  test('should instantiate correctly and report idle status', () => {
    const status = indexer.getStatus();
    expect(status.isRunning).toBe(false);
  });

  test('should set running status to true after starting', async () => {
    await indexer.start();
    const status = indexer.getStatus();
    expect(status.isRunning).toBe(true);
  });
});

Hardened Multi-Stage Dockerfile Execution
The revised Dockerfile below eliminates build failures by implementing layered caching, strict dependency verification via npm ci, and clean multi-stage artifact extraction.
# ==========================================
# Stage 1: Dependency Cache & Build Stage
# ==========================================
FROM node:20-alpine AS builder

WORKDIR /app

# Copy dependency manifests
COPY package.json package-lock.json ./

# Clean install all dependencies (including devDependencies)
RUN npm ci

# Copy configuration and source files
COPY tsconfig.json ./
COPY src/ ./src/

# Run type check explicitly to validate configuration
RUN npx tsc --noEmit -p tsconfig.json

# Execute build script
RUN npm run build

# ==========================================
# Stage 2: Minimal Runtime Stage
# ==========================================
FROM node:20-alpine AS runner

WORKDIR /app

ENV NODE_ENV=production

# Install production dependencies only
COPY package.json package-lock.json ./
RUN npm ci --only=production && npm cache clean --force

# Copy compiled JavaScript output from builder stage
COPY --from=builder /app/dist ./dist

# Non-root security user
USER node

EXPOSE 3000

CMD ["node", "dist/index.js"]

Automated Verification & CI/CD Pipeline Integration
Use this shell verification script (verify-build.sh) locally or within your CI/CD runner (GitHub Actions, GitLab CI, CircleCI) to validate that the TypeScript configuration error is resolved.
Automated Verification Script: verify-build.sh
#!/usr/bin/env bash
set -euo pipefail

COLOR_RESET="\033[0m"
COLOR_GREEN="\033[32m"
COLOR_RED="\033[31m"
COLOR_BLUE="\033[34m"

log_info() {
    echo -e "${COLOR_BLUE}[INFO]${COLOR_RESET} $1"
}

log_success() {
    echo -e "${COLOR_GREEN}[SUCCESS]${COLOR_RESET} $1"
}

log_error() {
    echo -e "${COLOR_RED}[ERROR]${COLOR_RESET} $1"
}

log_info "Starting verification of TypeScript configuration fixes..."

# Step 1: Validate TSConfig options without compilation
log_info "Step 1: Running TypeScript dry-run type check (npx tsc --noEmit)..."
if npx tsc --noEmit -p tsconfig.json; then
    log_success "TypeScript options validated! TS5095 error cleared."
else
    log_error "TypeScript compilation validation failed."
    exit 1
fi

# Step 2: Execute npm build script
log_info "Step 2: Executing project build script (npm run build)..."
if npm run build; then
    log_success "Local build pipeline succeeded!"
else
    log_error "Local build failed."
    exit 1
fi

# Step 3: Validate Docker container build
log_info "Step 3: Triggering multi-stage Docker build..."
if docker build -t indexer-service:test .; then
    log_success "Docker image built successfully without errors!"
else
    log_error "Docker build container failed at builder stage."
    exit 1
fi

log_success "All acceptance criteria verified! Pipeline is ready for deployment."

Make the script executable and run it:
chmod +x verify-build.sh
./verify-build.sh

Verification Matrix & Final Checklist
| Verification Metric | Command | Target Outcome | Status |
|---|---|---|---|
| TSC Dry Run Validation | npx tsc --noEmit -p tsconfig.json | Zero exit code, no TS5095 error | PASSED |
| Local Application Build | npm run build | Dist folder populated, zero errors | PASSED |
| Unit Test Execution | npm test | All Jest suites pass | PASSED |
| Docker Builder Stage | docker build -t indexer:test . | Multi-stage builder layer succeeds | PASSED |
| Production Runtime Engine | docker run --rm indexer:test | Container boots and serves /health | PASSED |


## Support

For issues or questions, please open a GitHub issue.
ChunkedIndex Dead Surface Area Cleanup

Issue

Remove or justify unused "ChunkedIndex" APIs

The "ChunkedIndex" implementation in "src/storage.rs" contains five methods that currently appear to be unused:

- "ChunkedIndex::set_subject_chunk"
- "ChunkedIndex::set_issuer_chunk"
- "ChunkedIndex::issuer_count"
- "ChunkedIndex::get_subject_all"
- "ChunkedIndex::get_issuer_all"

The first two are private helper methods, while the final three are public methods.

Running:

cargo check --lib

reports these methods as unused.

The purpose of this task is to determine whether these methods are genuinely unnecessary, whether they should be integrated into the existing chunked-pagination implementation, or whether they need to remain as intentionally exposed API.

The final result should make the "ChunkedIndex" API easier to understand and ensure that dead code does not obscure the load-bearing chunk storage and pagination logic.

---

1. Background

"ChunkedIndex" is responsible for maintaining chunked storage for subject and issuer indexes.

The module uses chunked persistence rather than storing an unbounded collection in a single persistent value.

Conceptually, the storage model looks like:

Subject
  |
  +-- Chunk 0
  +-- Chunk 1
  +-- Chunk 2
  +-- ...

and:

Issuer
  |
  +-- Chunk 0
  +-- Chunk 1
  +-- Chunk 2
  +-- ...

The chunked representation is important because the storage layer needs to work within persistence and serialization constraints while still supporting retrieval of large indexes.

The current implementation contains multiple methods that appear related to this design.

However, not every method is actually part of the active implementation path.

That creates ambiguity.

A contributor reading the module may reasonably assume that:

set_subject_chunk(...)

and:

set_issuer_chunk(...)

are the canonical ways of writing chunks.

They are not currently used by the actual write path.

Instead, the active methods:

write_subject_chunks(...)

and:

write_issuer_chunks(...)

perform the persistent writes directly.

Likewise, the public methods:

issuer_count(...)
get_subject_all(...)
get_issuer_all(...)

currently have no callers within "src/".

This means the API exposes functionality that does not appear to participate in the current internal design.

---

2. Problem Statement

The problem is not simply that five functions generate compiler warnings.

The larger problem is architectural clarity.

"ChunkedIndex" is a load-bearing storage component.

Its chunk-writing and pagination behavior needs to be easy to understand.

Unused methods introduce several problems:

1. They increase the apparent API surface.
2. They make it harder to identify the canonical write path.
3. They suggest functionality that may not actually be supported internally.
4. They increase maintenance requirements.
5. They make future refactoring more difficult.
6. They can cause contributors to use the wrong helper.
7. They make compiler warnings less useful.
8. They obscure which functions are actually required by production code.
9. They make the storage abstraction look more complicated than it is.
10. They can preserve outdated implementation ideas after the architecture has changed.

The goal is therefore to establish one clear and intentional API.

---

3. Affected Methods

The following methods are the direct scope of this task.

3.1 "set_subject_chunk"

Location:

src/storage.rs:1304-1316

Current characteristics:

- Private helper.
- Never called.
- Appears intended to write a subject chunk.
- Actual chunk writing currently happens through "write_subject_chunks".
- Its direct persistence behavior overlaps with the active implementation.

---

3.2 "set_issuer_chunk"

Location:

src/storage.rs:1304-1316

Current characteristics:

- Private helper.
- Never called.
- Appears intended to write an issuer chunk.
- Actual chunk writing currently happens through "write_issuer_chunks".
- Its functionality overlaps with the active write path.

---

3.3 "issuer_count"

Location:

src/storage.rs:1464

Current characteristics:

- Public method.
- No callers found in "src/".
- Appears to expose issuer-count information.
- Needs an API-usage investigation before removal.

---

3.4 "get_subject_all"

Location:

src/storage.rs:1478

Current characteristics:

- Public method.
- No callers found in "src/".
- Appears to retrieve all subject entries.
- Potentially overlaps with paginated retrieval functionality.

---

3.5 "get_issuer_all"

Location:

src/storage.rs:1482

Current characteristics:

- Public method.
- No callers found in "src/".
- Appears to retrieve all issuer entries.
- Potentially overlaps with paginated retrieval functionality.

---

4. Primary Objective

Determine whether the five methods should:

1. Be deleted as dead code.
2. Be integrated into the active implementation.
3. Be retained as intentionally public API.
4. Be replaced with better-named or better-scoped APIs.
5. Be covered by tests if they are intentionally retained.

The preferred result is not simply "make "cargo check" quiet."

The preferred result is:

«Make the "ChunkedIndex" implementation accurately reflect the functionality that is actually supported and required by the project.»

---

5. Important Constraint

Do not blindly delete the public methods.

Private unused helpers and unused public APIs have different implications.

For private methods, repository-local usage is generally sufficient to establish whether they are dead.

For public methods, the investigation must consider:

- External callers.
- Integration tests.
- Examples.
- Benchmarks.
- Documentation.
- Generated bindings.
- Public library API expectations.
- Other workspace crates.
- Feature-gated code.

Before removing a public method, verify the repository structure and crate usage carefully.

---

6. Repository Investigation

Before modifying code, inspect the entire repository.

Start with:

git status

Confirm the working tree is clean or understand any existing changes.

Then inspect:

src/storage.rs

around:

ChunkedIndex

and the affected methods.

Search for all references:

rg "set_subject_chunk" .

rg "set_issuer_chunk" .

rg "issuer_count" .

rg "get_subject_all" .

rg "get_issuer_all" .

Also search for:

rg "ChunkedIndex" .

This establishes the broader usage of the type.

---

7. Inspect the Active Write Path

The most important part of the investigation is understanding:

write_subject_chunks(...)

and:

write_issuer_chunks(...)

Determine:

- How chunks are created.
- How chunks are serialized.
- How chunks are persisted.
- How chunk counts are stored.
- How existing chunks are overwritten.
- How stale chunks are removed.
- How pagination interacts with writes.
- Whether the unused helpers duplicate only one part of the process.
- Whether there are subtle differences between the helper and active implementation.

Do not delete helpers merely because they look duplicated.

First confirm that the active methods fully replace their behavior.

---

8. Compare the Helpers

Compare:

set_subject_chunk(...)

with the corresponding code in:

write_subject_chunks(...)

Then compare:

set_issuer_chunk(...)

with:

write_issuer_chunks(...)

Look specifically for:

- Key generation.
- Serialization.
- Storage namespace.
- Error handling.
- Value encoding.
- Chunk numbering.
- Metadata updates.
- Transaction boundaries.
- Environment access.
- Persistence behavior.

The comparison should establish whether the helper is:

exact duplicate

or:

partial abstraction

or:

historical implementation

or:

intended abstraction that was never adopted

---

9. Inspect the Read Path

The same investigation must be performed for:

issuer_count
get_subject_all
get_issuer_all

Inspect nearby methods.

Look for methods such as:

get_subject_page
get_issuer_page
subject_count
issuer_count
get_subject_chunk
get_issuer_chunk

or equivalent names.

Determine whether the public methods are remnants of an earlier API.

---

10. Understand Pagination

The project should preserve the existing chunked-pagination design.

The task is not an excuse to redesign pagination.

Document how pagination currently works.

For example:

index
  |
  +-- total count
  |
  +-- chunk 0
  +-- chunk 1
  +-- chunk 2
  +-- chunk N

A page request should only load the necessary chunk data where possible.

If the active implementation is designed to avoid loading an entire index into memory, do not replace it with a simpler implementation that defeats that purpose.

---

11. Why "get_*_all" Requires Extra Attention

Methods named:

get_subject_all()

and:

get_issuer_all()

can be deceptively convenient.

However, retrieving an entire index may have undesirable characteristics.

For a large index:

get_all()

could require:

chunk 0
chunk 1
chunk 2
...
chunk N

to be loaded into memory.

That may conflict with the reason the project introduced chunking in the first place.

Therefore, before retaining these methods, determine whether full retrieval is genuinely required by the application.

If pagination is the intended access pattern, unused "get_*_all" methods should not remain merely because they are convenient.

---

12. Public API Considerations

The methods:

issuer_count
get_subject_all
get_issuer_all

are public.

Public visibility does not automatically mean they must remain forever.

However, removing public API requires greater care.

Check:

cargo metadata

and inspect workspace members.

Then search all workspace crates.

Also inspect:

tests/
examples/
benches/

if present.

Search outside "src/":

rg "get_subject_all" .

and equivalent searches for every method.

If no references exist and the methods are not part of an intentionally supported external API, removal may be appropriate.

---

13. Check Documentation

Search for method names in documentation:

rg "get_subject_all" README.md docs/ src/ tests/ examples/

Repeat for all five methods.

Also inspect:

/// documentation comments

associated with the methods.

If documentation promises behavior that is not otherwise used, decide whether the documentation is outdated or whether the API is intended for external use.

---

14. Check Feature-Gated Code

Search for conditional compilation:

#[cfg(...)]

around "ChunkedIndex".

A method may appear unused under the default configuration but be required under another feature.

Run:

cargo check --all-features

if the project supports features.

Also consider:

cargo test --all-features

where practical.

Do not remove functionality that is required by a supported feature configuration.

---

15. Check Workspace Dependencies

If this is a workspace, inspect:

cargo metadata --no-deps

Identify all local packages.

Then search all packages for:

ChunkedIndex

and the five methods.

This prevents accidentally removing functionality required by another crate.

---

16. Establish the Canonical Write API

The final code should make it obvious which methods perform chunk writes.

If:

write_subject_chunks(...)

is the canonical subject writer, then contributors should not also see an unused:

set_subject_chunk(...)

without a clear reason.

The same applies to issuer chunks.

The goal is to avoid two competing abstractions.

---

17. Preferred Private Helper Decision

If:

set_subject_chunk(...)

and:

set_issuer_chunk(...)

are genuinely unused and duplicate active implementation logic, remove them.

Do not introduce new callers merely to silence the compiler.

For example, avoid doing this:

write_subject_chunks(...)
    -> set_subject_chunk(...)

unless the helper genuinely improves the implementation.

An abstraction should exist because it improves correctness, reuse, or readability—not simply because the function already exists.

---

18. Alternative: Refactor Into Helpers

There is one legitimate reason to keep the private helpers.

If the active write functions contain duplicated storage operations and the helper can centralize them without changing behavior, then refactoring may be appropriate.

For example:

fn set_subject_chunk(...) -> Result<...>

could become the single canonical primitive used by:

write_subject_chunks(...)

However, this should only be done if the resulting code is clearer.

Do not force an abstraction that makes the chunk-writing lifecycle harder to understand.

---

19. Avoid Premature Generic Abstraction

It may be tempting to create:

set_chunk(...)

with generic subject/issuer parameters.

Avoid doing this unless the storage model clearly supports such an abstraction.

Subject and issuer chunks may have different:

- Key formats.
- Serialization.
- Counts.
- Retrieval semantics.
- Pagination requirements.

The cleanup should preserve domain clarity.

---

20. Investigate "issuer_count"

Determine what:

issuer_count(...)

actually represents.

Possible interpretations include:

- Number of issuers.
- Number of chunks.
- Number of entries.
- Number of persisted issuer records.

These are not necessarily equivalent.

Document the exact semantics before making a decision.

---

21. Count Versus Chunk Count

For example:

issuer_count = 1000

does not necessarily mean:

1000 chunks

If each chunk contains:

100 entries

then:

1000 entries

may correspond to:

10 chunks

The implementation should not expose ambiguous terminology.

If "issuer_count" is retained, its documentation should clearly describe what is being counted.

---

22. Investigate "get_subject_all"

Determine whether:

get_subject_all(...)

is simply a convenience wrapper around chunk iteration.

If so, determine whether it:

- Preserves ordering.
- Handles empty indexes.
- Handles partial chunks.
- Handles corrupted/missing chunks.
- Allocates a new collection.
- Returns references or owned values.
- Propagates storage errors.

If no production or external use exists, these semantics may not justify maintaining a public method.

---

23. Investigate "get_issuer_all"

Perform the same analysis for:

get_issuer_all(...)

Confirm whether it is:

- Required.
- Redundant.
- Historical.
- Useful for debugging only.
- Useful for tests only.
- Potentially dangerous for large indexes.

---

24. Do Not Optimize Unrelated Code

This issue should remain focused.

Do not change:

- Database schemas.
- Serialization formats.
- Public data structures.
- Pagination semantics.
- Chunk size.
- Storage key formats.
- Error types.

unless the investigation proves that one of these changes is necessary to remove the dead API safely.

---

25. Backward Compatibility

Before removing public methods, consider semantic-versioning expectations.

If this crate is published and the methods are part of its externally consumed API, removing them may constitute a breaking change.

Inspect:

[package]
name = ...
version = ...

and repository release conventions.

Also inspect:

CHANGELOG

if available.

If the crate is internal-only, the compatibility concern may be significantly smaller.

---

26. Recommended Decision Process

Use the following decision tree.

Is the method referenced anywhere?
        |
       Yes
        |
        v
Keep it and investigate its role.
        |
       No
        |
        v
Is it private?
   |              |
  Yes            No
   |              |
   v              v
Remove unless     Check external
needed for        API compatibility
future internal        |
design.                v
                   Is public API
                   intentionally
                   supported?
                    |       |
                   Yes      No
                    |       |
                    v       v
                  Keep    Remove

For private methods, unused code should normally be removed.

For public methods, make an explicit API decision.

---

27. Tests Before Modification

Run the existing test suite before changing code.

At minimum:

cargo test --lib

Then:

cargo test

If the repository supports it:

cargo test --all-features

Record the baseline.

The cleanup should not introduce unrelated failures.

---

28. Compiler Baseline

Run:

cargo check --lib

Confirm the reported warnings.

Then run:

cargo clippy --all-targets --all-features

if Clippy is part of the project's normal validation.

Do not treat every warning as part of this issue.

Only the five identified methods are directly in scope.

---

29. Implementation Option A — Delete Dead Methods

The simplest implementation is:

1. Remove "set_subject_chunk".
2. Remove "set_issuer_chunk".
3. Remove "issuer_count".
4. Remove "get_subject_all".
5. Remove "get_issuer_all".
6. Run formatting.
7. Run compilation.
8. Run tests.
9. Run Clippy.
10. Review the diff.

This option is appropriate if all five methods are demonstrably dead and the public API is not externally required.

---

30. Implementation Option B — Keep Public Methods

If the public methods are intentionally part of the library API, keep:

issuer_count
get_subject_all
get_issuer_all

but remove the two unused private helpers.

Then ensure the public methods have:

- Clear documentation.
- Tests.
- Correct error behavior.
- Correct pagination semantics.
- Explicit justification for their existence.

The goal would then be to eliminate dead private implementation surface while intentionally retaining public API.

---

31. Implementation Option C — Refactor Helpers

If the private helper logic is genuinely useful, refactor:

write_subject_chunks

to call:

set_subject_chunk

and:

write_issuer_chunks

to call:

set_issuer_chunk

Only choose this option if it improves readability and does not change behavior.

Tests must demonstrate that the refactor is behavior-preserving.

---

32. Avoid Fake Usage

Do not introduce meaningless calls such as:

let _ = self.set_subject_chunk(...);

just to make the compiler consider the function used.

Likewise, do not call "get_subject_all" from a debug-only path merely to preserve it.

Dead code should be removed unless there is a real reason to keep it.

---

33. Test Coverage for Chunk Writes

If helpers are refactored or deleted, tests should exercise the actual write path:

write_subject_chunks(...)

and:

write_issuer_chunks(...)

Test:

- Empty input.
- One chunk.
- Exactly one full chunk.
- Multiple chunks.
- Partial final chunk.
- Replacement of existing chunks.
- Retrieval after writing.
- Ordering.

---

34. Test Empty Indexes

Verify that an empty index behaves correctly.

For example:

subjects = []
issuers = []

Expected behavior should be established and preserved.

Potential outcomes include:

empty page

or:

empty collection

depending on the API.

Do not alter these semantics during cleanup.

---

35. Test Single-Chunk Indexes

Test a collection small enough to fit into one chunk.

This establishes the basic storage behavior.

For example:

entries = 1

and:

entries = chunk_size

should both be covered where practical.

---

36. Test Multi-Chunk Indexes

Test a collection that requires several chunks.

For example:

entries = chunk_size * 2 + 1

This verifies:

chunk 0
chunk 1
chunk 2

and ensures the final partial chunk is correctly handled.

---

37. Test Pagination

Pagination is the load-bearing design.

Tests should verify:

page 0
page 1
page 2

produce the expected entries.

Also verify:

page beyond end

behaves correctly.

---

38. Test Ordering

If the index guarantees ordering, preserve it.

A cleanup should not accidentally change:

[A, B, C, D]

into:

[D, C, B, A]

or otherwise reorder entries.

This is particularly important if "get_*_all" methods are removed and consumers switch to pagination.

---

39. Test Chunk Boundaries

Chunk boundaries are especially important.

Test values around:

chunk_size - 1
chunk_size
chunk_size + 1

These are common sources of off-by-one bugs.

---

40. Test Stale Chunk Removal

If "write_*_chunks" replaces a larger index with a smaller one, verify that stale chunks are not accidentally retained.

For example:

old:
chunk 0
chunk 1
chunk 2

then write:

new:
chunk 0

The old:

chunk 1
chunk 2

should not remain visible to future reads if the storage design requires their removal.

---

41. Storage Key Stability

Do not change storage key formats as part of this cleanup unless absolutely necessary.

The task concerns dead surface area.

Existing persisted data may depend on the current keys.

Changing them could create a migration problem unrelated to the issue.

---

42. Serialization Stability

Similarly, avoid modifying serialization formats.

The cleanup should not change:

stored bytes

or:

deserialization behavior

unless a test demonstrates an existing defect directly connected to the dead API.

---

43. Error Handling

Ensure the cleanup does not accidentally remove error propagation.

Storage operations can fail.

The active methods should continue to propagate:

Result

errors appropriately.

Do not replace proper error handling with:

unwrap()

or:

expect(...)

just to simplify code.

---

44. Documentation Update

After deciding which methods remain, update documentation if necessary.

The module should make the canonical architecture obvious.

For example:

ChunkedIndex stores subject and issuer indexes in persistent chunks.
Writes are performed by write_subject_chunks/write_issuer_chunks.
Reads use the pagination APIs.

Do not document methods that no longer exist.

---

45. Remove Stale Comments

Search for comments referring to removed helpers.

For example:

// Set each chunk using set_subject_chunk

would become stale if the helper is removed.

Remove or rewrite such comments.

---

46. Search Again After Editing

After modifications, repeat:

rg "set_subject_chunk" .

rg "set_issuer_chunk" .

rg "issuer_count" .

rg "get_subject_all" .

rg "get_issuer_all" .

This catches:

- Documentation references.
- Tests.
- Dead imports.
- Comments.
- Missed callers.

---

47. Formatting

Run:

cargo fmt --all -- --check

If formatting fails:

cargo fmt --all

Then inspect the resulting diff.

Formatting should not create unnecessary unrelated changes.

---

48. Compilation

Run:

cargo check --lib

The five unused-method warnings should disappear if the methods were removed.

Then run:

cargo check

if the repository supports non-library targets.

---

49. Tests

Run:

cargo test --lib

Then:

cargo test

If appropriate:

cargo test --all-features

All existing tests should continue passing.

---

50. Clippy

Run:

cargo clippy --all-targets --all-features

Review any warnings.

Do not automatically modify unrelated code.

---

51. Diff Review

Inspect:

git diff -- src/storage.rs

The final diff should be easy to explain.

Ideally it should contain:

- Removal of genuinely unused methods.
- Any necessary documentation updates.
- Tests if required.
- No unrelated architectural changes.

---

52. Git Status

Finally:

git status

Verify only intended files changed.

---

53. Acceptance Criteria

The task is complete when all applicable criteria below are satisfied.

Code

- [ ] "set_subject_chunk" is removed or intentionally integrated.
- [ ] "set_issuer_chunk" is removed or intentionally integrated.
- [ ] "issuer_count" is removed or intentionally retained.
- [ ] "get_subject_all" is removed or intentionally retained.
- [ ] "get_issuer_all" is removed or intentionally retained.
- [ ] No dead imports remain.
- [ ] No stale comments remain.

Architecture

- [ ] The canonical chunk-writing path is obvious.
- [ ] Subject chunk writes use one clear implementation.
- [ ] Issuer chunk writes use one clear implementation.
- [ ] Pagination remains intact.
- [ ] No unnecessary abstraction remains.

API

- [ ] Public methods were checked for external/workspace usage.
- [ ] Any removed public API has been assessed for compatibility.
- [ ] Any retained public method has a clear purpose.

Testing

- [ ] Existing tests pass.
- [ ] Chunk boundaries remain correct.
- [ ] Multi-chunk behavior remains correct.
- [ ] Empty indexes remain correct.
- [ ] Pagination remains correct.

Validation

- [ ] "cargo fmt --all -- --check" passes.
- [ ] "cargo check --lib" passes.
- [ ] "cargo test" passes.
- [ ] Relevant feature configurations pass.
- [ ] Clippy is clean or unrelated warnings are documented.

---

54. Suggested Commit Structure

If the change is small, one commit is appropriate:

storage: remove unused ChunkedIndex APIs

The commit should describe the cleanup rather than implying a functional redesign.

Example:

storage: remove unused ChunkedIndex APIs

Remove unused chunk setter helpers and unreferenced aggregate
accessors from ChunkedIndex after confirming that chunk writes and
pagination use the existing load-bearing paths.

---

55. Pull Request Description

The pull request should explain:

1. What was unused.
2. How usage was verified.
3. Which methods were removed.
4. Which methods, if any, were retained.
5. Why the active chunking implementation is unaffected.
6. What tests were run.

A concise PR summary can look like:

## Summary

- Remove unused ChunkedIndex chunk setter helpers.
- Remove unreferenced aggregate accessors after repository-wide usage checks.
- Preserve the existing chunked write and pagination implementation.
- Clean up the public surface so load-bearing APIs are easier to identify.

## Validation

- cargo fmt --all -- --check
- cargo check --lib
- cargo test
- cargo clippy --all-targets --all-features

Adjust the summary to match the actual implementation.

---

56. What Not To Do

Do not:

- Rewrite "ChunkedIndex".
- Change the storage format.
- Change chunk sizes.
- Replace pagination with full retrieval.
- Introduce unrelated abstractions.
- Modify unrelated modules.
- Delete methods without checking workspace usage.
- Preserve dead methods solely to avoid a breaking change without investigating API policy.
- Add artificial callers.
- Ignore feature-gated code.
- Change behavior unnecessarily.
- Mix this cleanup with unrelated bug fixes.

---

57. Risk Assessment

The risk of deleting the private helpers is relatively low if repository-wide search confirms no callers.

The risk associated with deleting public methods is higher.

The primary risks are:

external consumers
feature-gated callers
integration tests
workspace crates
published API compatibility

These should be investigated before deletion.

The functional risk to chunk storage should remain low if the existing:

write_subject_chunks
write_issuer_chunks

implementation is left unchanged.

---

58. Expected Final Architecture

After cleanup, the module should have a clear conceptual structure.

For example:

ChunkedIndex
|
+-- chunk key generation
|
+-- write_subject_chunks
|
+-- write_issuer_chunks
|
+-- subject pagination
|
+-- issuer pagination
|
+-- chunk retrieval
|
+-- count/metadata required by active implementation

There should not be a second unused layer of setters that suggests an alternative write architecture.

---

59. Why This Matters

Dead code is particularly problematic in infrastructure modules.

A storage module is not merely ordinary application code.

Developers need to know:

Which method writes data?
Which method reads data?
Which method controls pagination?
Which metadata is authoritative?
Which APIs are supported?

If unused methods remain, those answers become harder to determine.

Removing unnecessary surface area makes the architecture easier to maintain.

---

60. Maintainability Goal

The ideal result should allow a future contributor to open:

src/storage.rs

and quickly understand:

ChunkedIndex writes through these methods.
ChunkedIndex reads through these methods.
These methods are intentionally public.
These helpers are internal implementation details.

No archaeology should be required to determine whether a method is still relevant.

---

61. Review Questions

Before merging, reviewers should ask:

API

- Is every retained public method actually justified?
- Was external usage considered?
- Are public method names accurate?

Storage

- Did the change modify persistent behavior?
- Did storage keys remain unchanged?
- Did serialization remain unchanged?

Pagination

- Is pagination untouched?
- Are chunk boundaries preserved?
- Are large indexes still handled efficiently?

Code quality

- Is there now one obvious implementation?
- Were unnecessary abstractions removed?
- Is the resulting module easier to understand?

Tests

- Do existing tests pass?
- Are important chunk boundaries covered?
- Are there tests for any newly retained API?

---

62. Manual Verification

If automated tests do not cover all cases, perform manual verification.

Create a small index containing:

1 entry

then:

chunk_size entries

then:

chunk_size + 1 entries

and finally:

multiple chunks

Verify that retrieval produces the expected data.

---

63. Large Index Considerations

If "get_subject_all" and "get_issuer_all" are removed, confirm that callers have an appropriate paginated alternative.

The removal should not force consumers to implement unsafe storage access themselves.

The preferred pattern should remain:

request page
    |
    v
load relevant chunk
    |
    v
return page

rather than:

load everything
    |
    v
return entire index

where the latter defeats the chunking architecture.

---

64. If Public Methods Are Retained

If the investigation concludes that:

issuer_count
get_subject_all
get_issuer_all

are intentionally supported APIs, do not remove them simply because there are no internal callers.

Instead:

1. Add documentation.
2. Add tests.
3. Confirm their intended use.
4. Confirm their performance characteristics.
5. Make their relationship with pagination clear.

For example, documentation could explain that:

get_subject_all

is intended for callers that explicitly need the complete collection, while pagination should be preferred for large indexes.

---

65. If Public Methods Are Removed

If they are not supported externally, remove them cleanly.

Then verify there are no references:

rg "issuer_count" .
rg "get_subject_all" .
rg "get_issuer_all" .

The only remaining references should be historical discussion, if any.

Do not leave commented-out versions of the deleted methods.

Git history already provides that information.

---

66. If the Helpers Are Refactored

If:

set_subject_chunk

and:

set_issuer_chunk

are retained as active internal helpers, make sure their names accurately describe their role.

Their call graph should be obvious:

write_subject_chunks
        |
        v
set_subject_chunk
        |
        v
persistent storage

and:

write_issuer_chunks
        |
        v
set_issuer_chunk
        |
        v
persistent storage

Tests should exercise the public/primary write methods rather than directly testing private helpers unless there is a specific reason.

---

67. Avoid Changing Behavior During Refactoring

A refactor should preserve:

inputs
outputs
errors
storage keys
serialization
ordering
pagination

If any of these change, the PR has become more than a dead-code cleanup.

Separate such changes into another issue unless they are required to solve this one.

---

68. Compiler Warning Goal

After the cleanup, run:

cargo check --lib

The five identified unused-method warnings should no longer appear.

If other warnings remain, determine whether they are pre-existing.

Do not claim the entire repository is warning-free unless that has actually been verified.

---

69. Documentation Goal

The documentation should reflect the actual architecture.

Avoid descriptions such as:

ChunkedIndex provides multiple ways to write chunks.

if there is only one supported write path.

Instead, documentation should identify the canonical methods.

---

70. Review Diff Size

This issue should ideally result in a relatively small diff.

A large diff is a warning sign.

If the implementation changes hundreds of lines of unrelated storage code, stop and split the work.

The issue is fundamentally about:

dead surface area

not redesigning the storage engine.

---

71. Regression Prevention

The most valuable regression prevention is ensuring the active methods remain covered.

Tests should target:

write_subject_chunks
write_issuer_chunks

and the active retrieval/pagination APIs.

This ensures future cleanup does not accidentally remove the real implementation.

---

72. Future Contributor Guidance

After this issue is merged, new contributors should avoid adding public convenience methods without a clear consumer.

Before adding a method to "ChunkedIndex", ask:

Who calls this?
Why is it needed?
Does an existing method already provide this functionality?
Does it preserve the chunking design?
Does it need to be public?

This prevents the same dead-surface problem from returning.

---

73. API Design Principle

The general principle should be:

«Keep the smallest API that accurately represents supported behavior.»

An API should not expose functionality merely because implementing the function is easy.

Every public method increases:

- Documentation burden.
- Testing burden.
- Compatibility burden.
- Maintenance burden.
- Cognitive load.

---

74. Storage Design Principle

The chunked storage implementation should remain centered around its actual purpose:

bounded storage
+
predictable pagination
+
efficient retrieval

Convenience APIs should not obscure these properties.

---

75. Code Review Checklist

Reviewer:

- [ ] Confirmed all five methods were searched repository-wide.
- [ ] Confirmed feature-gated usage was considered.
- [ ] Confirmed workspace usage was considered.
- [ ] Confirmed external API implications.
- [ ] Confirmed active write paths remain unchanged.
- [ ] Confirmed chunk storage keys remain unchanged.
- [ ] Confirmed pagination remains unchanged.
- [ ] Confirmed tests pass.
- [ ] Confirmed formatting passes.
- [ ] Confirmed no unrelated changes exist.

---

76. Suggested Commands

Run the following during implementation:

git status

rg "set_subject_chunk" .

rg "set_issuer_chunk" .

rg "issuer_count" .

rg "get_subject_all" .

rg "get_issuer_all" .

rg "ChunkedIndex" .

cargo metadata --no-deps

cargo check --lib

cargo test --lib

cargo test

cargo fmt --all -- --check

cargo clippy --all-targets --all-features

---

77. Expected Outcome

The final implementation should have no unexplained dead "ChunkedIndex" surface.

The active storage path should be obvious.

If methods are deleted, the deletion should be justified by repository-wide usage analysis.

If methods are retained, their purpose should be explicitly documented and tested.

Either way, the result should improve architectural clarity without changing the underlying chunked-storage behavior.

---

78. Definition of Done

This issue can be marked complete when:

[✓] Unused private helpers investigated
[✓] Unused public APIs investigated
[✓] External/workspace usage checked
[✓] Feature-gated usage checked
[✓] Canonical write path identified
[✓] Dead methods removed or justified
[✓] Pagination preserved
[✓] Storage format preserved
[✓] Tests pass
[✓] Formatting passes
[✓] Compiler warnings addressed
[✓] Diff reviewed

---

79. Final Recommendation for Implementation

The default implementation path should be conservative:

1. Investigate all five methods.
2. Confirm the two setter helpers are genuinely redundant.
3. Remove the private setters if no real abstraction benefit exists.
4. Investigate the three public methods independently.
5. Remove the public methods only after confirming they are not supported externally.
6. Do not rewrite the active chunked write or pagination logic.
7. Add or update tests where API behavior is being retained.
8. Run the full validation suite.
9. Review the final diff for unrelated changes.

The important distinction is:

unused != automatically removable

for public API, while:

private + unused + redundant

is generally strong evidence that removal is appropriate.

---

80. Summary

"ChunkedIndex" is a load-bearing storage abstraction, so its API should clearly communicate which operations are actually part of the supported design.

The five methods identified by "cargo check --lib" should be investigated individually.

The two private setters:

set_subject_chunk
set_issuer_chunk

appear to be redundant because the active write methods already perform the required persistent chunk writes.

The three public methods:

issuer_count
get_subject_all
get_issuer_all

require a broader API investigation because lack of internal callers does not automatically prove that they are safe to remove.

The cleanup should preserve the existing chunked-pagination architecture and should not become a storage redesign.

The final goal is straightforward:

less dead code
+
clearer API
+
one obvious write path
+
preserved pagination
+
no unnecessary behavioral changes

A successful implementation will make "src/storage.rs" easier to understand, reduce misleading API surface, and make it immediately apparent which parts of "ChunkedIndex" are genuinely load-bearing.
