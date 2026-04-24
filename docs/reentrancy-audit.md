# Reentrancy Audit for TrustLink

## Overview

This document provides a comprehensive reentrancy analysis for the TrustLink attestation contract, specifically focusing on the `create_attestation` function and its interaction with external token contracts.

## Risk Assessment

### High-Risk Function: `create_attestation`

The `create_attestation` function performs the following operations:
1. Validation checks
2. Attestation state creation
3. **External token transfer** (potential reentrancy vector)
4. Event emission

### Vulnerability: Unguarded Fee Transfer

**Before mitigation**: The function called `charge_attestation_fee()` (which invokes an external token contract via `TokenClient::transfer()`) *before* persisting the attestation state to storage. This creates a reentrancy window:

```
create_attestation()
  └─> charge_attestation_fee()
        └─> external token contract (malicious code executes here)
              └─> could call create_attestation() again while state is inconsistent
  └─> store_attestation()  [TOO LATE]
```

A malicious token contract could:
- Re-enter `create_attestation()` before the original call completes
- Create attestations with partially-initialized state
- Potentially bypass validators or duplicate attestations
- Corrupt per-issuer or per-subject attestation counts

### Attack Scenario

```rust
// Attacker-controlled token contract
impl Token for MaliciousToken {
    fn transfer(from, to, amount) {
        // Before the original create_attestation stores its state,
        // re-enter with a new attestation request on the same issuer
        contract.create_attestation(
            issuer,
            subject2,
            claim_type2,
            expiration,
            metadata, 
            tags,
        );
        // Original create_attestation still hasn't stored its state!
    }
}
```

### Impact

- **State Corruption**: Attestation state might be partially written or duplicated
- **Double-Issuance**: Multiple attestations created in a single transaction
- **Bypass Validation**: Rate limiting or storage limits might be bypassed
- **Inconsistent Counters**: `total_issued` per-issuer statistics could become incorrect

## Mitigation: Reentrancy Guard via State Ordering

### Solution: Check-Effects-Interactions Pattern

The contract now follows the **Check-Effects-Interactions** pattern:

```rust
fn create_attestation_internal(...) {
    // 1. CHECKS: Validate all inputs and state requirements
    Validation::require_not_paused(&env)?;
    Validation::require_issuer(&env, &issuer)?;
    Validation::validate_claim_type(&claim_type)?;
    check_rate_limit(env, &issuer)?;  // NEW: Rate limit check
    
    // 2. EFFECTS: Update internal state FIRST
    let attestation = Attestation { ... };
    store_attestation(env, &attestation);              // Store state
    Storage::increment_total_attestations(env, 1);      // Update counters
    Storage::append_audit_entry(...);                   // Record action
    Storage::set_last_issuance_time(...);               // Record timestamp
    
    // 3. INTERACTIONS: Call external contracts LAST
    charge_attestation_fee(env, &issuer)?;  // AFTER state is persisted
    
    // 4. Emit event
    Events::attestation_created(env, &attestation);
}
```

### Why This Prevents Reentrancy

By storing attestation state **before calling the external token contract**:
- When the malicious token attempts to re-enter `create_attestation()`, the first attestation is already persisted
- The rate limit check will see the timestamp and block the reentrancy attempt
- Storage counters are already updated, preventing double-counting
- Audit logs are locked in, providing an immutable record

### Rate Limiting as Additional Defense

Issue #257 implements per-issuer rate limiting via `min_issuance_interval`:
- Each successful `create_attestation()` now records a timestamp
- Subsequent calls from the same issuer must wait the configured interval
- This provides defense-in-depth: even if reentrancy occurs, the rate limit prevents rapid-fire duplicate creations

## Verification

### Invariants Maintained

1. **Every stored attestation has a corresponding audit entry**
   - Audit entry created in step 2 (EFFECTS)
   - Attestation stored in step 2 (EFFECTS)
   - If token transfer fails in step 3, both are already on-chain

2. **IssuerStats.total_issued matches actual attestation count**
   - Incremented in step 2 before external call
   - Never double-incremented due to reentrancy

3. **Rate limit timestamps are always recorded**
   - Recorded before fee transfer
   - Prevents rapid reentrancy cycles

### Testing

The contract includes tests that verify:
- Attestations are stored before fees are charged
- Reentrant calls are blocked by rate limiting
- Storage counts remain consistent under reentrancy attempts

See `tests/integration_test.rs` for comprehensive reentrancy scenarios.

## Soroban SDK Context

On Soroban (Stellar's smart contract platform), reentrancy is somewhat mitigated by the WebAssembly execution model and the Stellar network's transaction model. However:

- **Callback contracts** (via `ExpirationHook` and custom integrations) can still perform reentrancy
- **Cross-contract calls** during a single transaction may create reentrancy windows
- **These mitigations provide defense-in-depth** for any external contract interaction

## Conclusion

TrustLink is protected against reentrancy attacks through:
1. **Check-Effects-Interactions pattern**: State updates before external calls
2. **Rate limiting**: Per-issuer timestamp tracking prevents rapid reentrancy
3. **Immutable audit logs**: All actions recorded before external I/O
4. **Strong counter invariants**: Total statistics always accurate

These defenses ensure that even if a token contract or callback attempts to re-enter the attestation system, the contract state remains consistent and secure.
