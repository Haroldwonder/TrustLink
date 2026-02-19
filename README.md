# TrustLink - On-Chain Attestation & Verification System

TrustLink is a Soroban smart contract that provides a reusable trust layer for the Stellar blockchain. It enables trusted issuers (anchors, fintech apps, institutions) to create, manage, and revoke attestations about wallet addresses, allowing other contracts and applications to verify claims before executing financial operations.

## Overview

TrustLink solves the problem of decentralized identity verification and trust establishment on-chain. Instead of each application building its own KYC/verification system, TrustLink provides a shared attestation infrastructure that can be queried by any smart contract or dApp.

### Key Features

- **Authorized Issuers**: Admin-controlled registry of trusted attestation issuers
- **Flexible Claims**: Support for any claim type (KYC_PASSED, ACCREDITED_INVESTOR, MERCHANT_VERIFIED, etc.)
- **Expiration Support**: Optional time-based expiration for attestations
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
    id: String,              // Deterministic hash-based ID
    issuer: Address,         // Who issued the attestation
    subject: Address,        // Who the attestation is about
    claim_type: String,      // Type of claim (e.g., "KYC_PASSED")
    timestamp: u64,          // When it was created
    expiration: Option<u64>, // Optional expiration time
    revoked: bool            // Revocation status
}
```

**Storage Keys:**
- `Admin`: Contract administrator address
- `Issuer(Address)`: Authorized issuer registry
- `Attestation(String)`: Individual attestation data
- `SubjectAttestations(Address)`: Index of attestations per subject
- `IssuerAttestations(Address)`: Index of attestations per issuer

## Usage

### Initialization

```rust
// Deploy and initialize with admin
contract.initialize(&admin_address);
```

### Register Issuers

```rust
// Admin registers a trusted issuer
contract.register_issuer(&admin, &issuer_address);

// Check if address is authorized
let is_authorized = contract.is_issuer(&issuer_address);
```

### Create Attestations

```rust
// Issuer creates a KYC attestation
let attestation_id = contract.create_attestation(
    &issuer,
    &user_address,
    &String::from_str(&env, "KYC_PASSED"),
    &None  // No expiration
);

// Create attestation with expiration
let expiration_time = current_timestamp + 365 * 24 * 60 * 60; // 1 year
let attestation_id = contract.create_attestation(
    &issuer,
    &user_address,
    &String::from_str(&env, "ACCREDITED_INVESTOR"),
    &Some(expiration_time)
);
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
```

### Revoke Attestations

```rust
// Issuer revokes an attestation
contract.revoke_attestation(&issuer, &attestation_id);
```

### Query Attestations

```rust
// Get specific attestation
let attestation = contract.get_attestation(&attestation_id);

// Check status
let status = contract.get_attestation_status(&attestation_id);
// Returns: Valid, Expired, or Revoked

// List user's attestations (paginated)
let attestations = contract.get_subject_attestations(&user_address, &0, &10);

// List issuer's attestations
let issued = contract.get_issuer_attestations(&issuer_address, &0, &10);
```

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

## License

MIT

## Contributing

Contributions welcome! Please ensure all tests pass and code is formatted before submitting PRs.

## Support

For issues or questions, please open a GitHub issue.
