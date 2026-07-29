# ADR-010: Issuer-Defined Custom Validation Hooks for Attestation Creation

**Status:** Accepted
**Date:** 2026-06-26
**Author:** TrustLink Team

## Problem Statement

All attestation validation logic in `src/validation.rs` is fixed at the contract level. An issuer with domain-specific requirements (e.g., requiring minimum metadata length, validating subject format against an external allowlist, enforcing metadata schema compliance) has no extension point and must fork the contract.

## Proposed Solution

Implement a **cross-contract validation hook pattern** that allows issuers to register a custom validator contract. When an attestation is created, the contract optionally invokes the issuer's validator to approve or reject the operation.

### Design Approach: Cross-Contract Validation

**Why cross-contract?**
- **Flexibility:** Issuers define validation logic in their own contracts, not constrained by TrustLink's runtime limits
- **Upgradeable:** Issuers can deploy new validator contracts and re-register without touching TrustLink
- **Safe:** Validator failures don't block attestation creation (configurable per issuer)
- **Auditability:** Each validator invocation is observable on-chain

### Hook Interface

Validators implement a single function:

```rust
pub fn validate_attestation(
    env: Env,
    subject: Address,
    claim_type: String,
    metadata: Option<String>,
) -> Result<bool, ContractError>
```

Returns `true` if validation passes, `false` to reject with `Error::CustomValidationFailed`.

### Storage

**New types:**
- `IssuerValidator(issuer_address)` → `Option<Address>` of registered validator contract

**Behavior:**
- If no validator is registered for the issuer → no hook is called (skip validation)
- If validator is registered → `create_attestation` invokes it before storing the attestation
- Validator failure returns `Error::CustomValidationFailed`

### Integration Point

In `create_attestation`, after standard checks:

```rust
if let Some(validator) = Storage::get_issuer_validator(&env, &issuer)? {
    let result = TrustLinkValidator::new(&env, &validator)
        .validate_attestation(&subject, &claim_type, &metadata)?;
    if !result {
        return Err(Error::CustomValidationFailed);
    }
}
```

### Admin Functions

```rust
fn register_validator(env: Env, admin: Address, issuer: Address, validator: Address)
fn remove_validator(env: Env, admin: Address, issuer: Address)
fn get_issuer_validator(env: Env, issuer: Address) -> Result<Option<Address>, Error>
```

### Example: Metadata Schema Validator

An issuer creates a validator contract that requires metadata to be valid JSON:

```rust
#[contract]
pub struct KYCValidator;

#[contractimpl]
impl KYCValidator {
    pub fn validate_attestation(
        env: Env,
        subject: Address,
        claim_type: String,
        metadata: Option<String>,
    ) -> Result<bool, ContractError> {
        // Only enforce for KYC claims
        if claim_type != String::from_str(&env, "KYC_PASSED") {
            return Ok(true);
        }

        match metadata {
            Some(meta) => {
                // Validate JSON structure
                serde_json::from_str(&meta.clone())
                    .map(|_| true)
                    .map_err(|_| ContractError::InvalidMetadata)
            }
            None => Err(ContractError::MetadataRequired),
        }
    }
}
```

## Alternative Approaches Considered

### 1. **Built-in Per-Claim-Type Rules** ❌
- **Pros:** Simple, predictable, no cross-contract overhead
- **Cons:** Inflexible, hard to update, limited rule expressiveness, storage explosion
- **Decision:** Rejected—too restrictive for issuer-specific domains

### 2. **Admin-Managed Validation Registry** ❌
- **Pros:** Centralized, auditable
- **Cons:** Scalability issue, single point of failure, violates principle of issuer autonomy
- **Decision:** Rejected—moves authority away from issuers

## Tradeoffs

| Aspect | Pro | Con |
|--------|-----|-----|
| **Flexibility** | Issuers define own logic | Added CU cost per `create_attestation` |
| **Upgradability** | New validators without contract change | Requires issuer coordination |
| **Safety** | Optional (no validator = no overhead) | Validator bug can block issuances |
| **Complexity** | Well-scoped cross-contract pattern | Debugging requires looking at two contracts |

## Implementation Steps

1. **Phase 1: Core Hook System**
   - Add `IssuerValidator` storage key
   - Implement `register_validator` / `remove_validator` / `get_issuer_validator`
   - Integrate hook check into `create_attestation`
   - Add `CustomValidationFailed` error type

2. **Phase 2: Example Validator**
   - Create `examples/validation-hooks/` with metadata schema validator
   - Document the validator interface

3. **Phase 3: Testing**
   - Unit tests: hook registration, invocation, failure scenarios
   - Integration test: cross-contract validation in action

4. **Phase 4: Documentation**
   - Update README with validator pattern
   - Add validator development guide

## Security Considerations

- **Validator DoS:** If validator contract is buggy or adversarial, it can slow down attestation creation. Mitigated by Soroban CU limits.
- **State modification:** Validator functions are read-only (cannot modify TrustLink state).
- **Unregistered validator:** If issuer registers an invalid contract address, `create_attestation` fails with clear error.

## Testing

```bash
# Unit tests
cargo test --test validation_hooks_test

# Example validator
cargo test --example validation-hooks
```

## Future Enhancements

- **Async validators:** Support off-chain validation via oracle pattern
- **Validator registry:** Admin-managed list of vetted validators
- **Revocation hooks:** Allow validators to trigger revocation on updated conditions
- **Hook chains:** Multiple validators per claim type (in series)

