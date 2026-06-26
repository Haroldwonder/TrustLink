# ADR-010: Custom Validation Hooks for Attestations

## Status
Accepted

## Context

Currently, all attestation validation logic is hardcoded in `src/validation.rs`. Issuers with domain-specific requirements (e.g., requiring a minimum metadata length for a specific claim type) cannot extend validation without forking the contract.

Two approaches were evaluated:

### Option A: Cross-Contract Validation Hooks
- Issuer registers a validator contract address
- `create_attestation` calls the validator before storing the attestation
- Validator can enforce arbitrary rules (metadata format, content validation, etc.)
- **Pros**: Maximum flexibility, issuer-specific logic
- **Cons**: Cross-contract calls add gas cost, latency, potential failure modes

### Option B: Built-in Per-Claim-Type Rules
- Admin registers constraint rules per claim type (e.g., min/max metadata length)
- `create_attestation` applies rules during validation
- **Pros**: Simple, gas-efficient, deterministic
- **Cons**: Limited extensibility, requires contract upgrade for new rules

## Decision

**Implement Option B (Built-in Per-Claim-Type Rules)** as the primary solution, with a clear path to Option A in a future release.

**Rationale:**
- Aligns with immutable history principle: contract-level validation is predictable and auditable
- Avoids cross-contract call overhead and failure cascades
- Claim type registry already exists; adding constraints is a natural extension
- Safer during pre-mainnet period

## Implementation

### Schema
```rust
pub struct ClaimTypeConstraints {
    pub min_metadata_len: Option<u32>,
    pub max_metadata_len: Option<u32>,
    pub require_metadata: bool,
    pub require_unique_per_issuer_per_subject: bool,
}
```

### Storage
- New storage key: `ClaimTypeConstraints(claim_type: String)` → `ClaimTypeConstraints`

### API Changes

#### Admin Functions
- `set_claim_type_constraints(admin, claim_type, constraints)` — register or update constraints
- `get_claim_type_constraints(claim_type)` — retrieve constraints

#### Validation
- `validate_claim_against_constraints()` — called during `create_attestation`

## Consequences

### Positive
- Issuers can enforce custom business logic without contract forking
- Admin can react to emerging validation needs (e.g., require metadata for a specific claim type)
- No gas overhead from cross-contract calls
- Validation stays on-chain and auditable

### Negative
- Still requires contract upgrade if new constraint types are needed
- Does not support arbitrary issuer-defined logic (e.g., "validate that metadata is valid JSON matching my schema")
- Cross-contract hooks remain a future enhancement

## Future Enhancement (ADR-011)
A follow-up ADR will explore issuer-registered cross-contract hooks for truly custom validation, with cost/safety analysis.
