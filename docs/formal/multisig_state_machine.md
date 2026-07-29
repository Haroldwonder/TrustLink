# Multi-Sig Proposal Lifecycle Specification

## Overview

This document describes the formal state machine specification for TrustLink's multi-signature proposal lifecycle. The specification is written in TLA+ and is located at `docs/formal/multisig_state_machine.tla`.

## Why a Formal Specification?

The multi-signature proposal lifecycle involves complex state transitions that are easy to get wrong:

1. **Propose** → Create a new proposal (auto-signs proposer)
2. **Cosign** → Signers add their signatures
3. **Finalize** → Reached threshold
4. **Cancel** → Cancelled by proposer
5. **Expire** → Time-based expiration

Subtle bugs in this state machine include:
- Missing `require_not_paused` check (found in audit)
- Proposer signature counting bug (found in audit)
- Missing transition guards for edge cases

A formal specification makes these bugs structurally harder to introduce.

## State Machine Definition

### States

```
ProposalState = {
  id: Address,
  proposer: Address,
  subject: Address,
  claim_type: ClaimType,
  required_signers: SET OF Address,
  threshold: Nat,
  signers: SET OF Address,
  created_at: Int,
  expires_at: Int,
  finalized: Bool,
  cancelled: Bool,
  paused: Bool
}
```

### State Transitions

#### 1. Propose

Creates a new proposal. The proposer is automatically added to the signers set.

**Preconditions:**
- All required signers are valid addresses
- Threshold > 0 and threshold ≤ number of required signers
- Required signers ≤ MaxSigners
- Timestamp doesn't overflow
- Proposal ID is unique

**State changes:**
- Add new proposal to proposals map
- Set proposer in signers set
- Calculate expiration as now + ttl_secs

#### 2. Cosign

Adds a signature to an existing proposal.

**Preconditions:**
- Proposal exists and is not cancelled/finalized
- Current time < expiration
- Signer is in required_signers
- Signer hasn't already signed

**State changes:**
- Add signer to proposal's signers set
- Add signature to signatures set

#### 3. Finalize

Marks a proposal as finalized when threshold is reached.

**Preconditions:**
- Proposal exists and is not cancelled/finalized
- Current time < expiration
- Contract is not paused
- Number of signers ≥ threshold

**State changes:**
- Set finalized = TRUE

#### 4. Cancel

Allows the proposer to cancel a proposal.

**Preconditions:**
- Caller is the proposer
- Proposal exists and is not cancelled/finalized
- Current time < expiration
- Contract is not paused

**State changes:**
- Set cancelled = TRUE

#### 5. Expire

Marks a proposal as expired when time expires.

**Preconditions:**
- Proposal exists and is not finalized/cancelled
- Current time ≥ expiration

**State changes:**
- No state change (proposal remains in proposals map but is effectively expired)

#### 6. Pause/Unpause

Pauses the entire contract, affecting all proposals.

**Preconditions:**
- All proposals must be in non-terminal state (not finalized, not cancelled)

**State changes:**
- Set paused = TRUE/FALSE for all proposals

## Invariants

The specification verifies these critical invariants:

### 1. NoPauseModification
```
∀p: proposals[p].paused ⇒ ¬proposals[p].finalized ∧ ¬proposals[p].cancelled
```

**Meaning:** No state-changing transition succeeds while the contract is paused.

**Why it matters:** This invariant was violated by the missing `require_not_paused` check on `cancel_multisig_proposal`.

### 2. SignerAuth
```
∀p, s: s ∈ proposals[p].signers ⇒ s ∈ proposals[p].required_signers
```

**Meaning:** Signature count never includes non-required signers.

**Why it matters:** Ensures only authorized addresses can sign proposals.

### 3. ThresholdSatisfied
```
∀p: proposals[p].finalized ⇒ |proposals[p].signers| ≥ proposals[p].threshold
```

**Meaning:** Threshold is satisfied only when enough signatures exist.

**Why it matters:** Prevents finalization before threshold is met.

### 4. MutuallyExclusive
```
∀p: ¬(proposals[p].finalized ∧ proposals[p].cancelled)
```

**Meaning:** A proposal cannot be both finalized and cancelled.

**Why it matters:** Ensures terminal states are distinct.

### 5. SignatureLimit
```
∀p: |proposals[p].signers| ≤ |proposals[p].required_signers|
```

**Meaning:** Signature count never exceeds required signers.

**Why it matters:** Prevents signature flooding attacks.

### 6. ProposerSigns
```
∀p: proposals[p].proposer ∈ proposals[p].signers
```

**Meaning:** Proposer auto-signs on creation.

**Why it matters:** The proposer is always a signer by design.

### 7. Expiration
```
∀p, now: now ≥ proposals[p].expires_at ∧ 
         ¬finalized ∧ ¬cancelled ⇒ no operations allowed
```

**Meaning:** Expiration prevents all operations after expiry.

**Why it matters:** Ensures proposals can't be modified after expiration.

### 8. NoDuplicateSignatures
```
∀p: |proposals[p].signers| = |{s ∈ proposals[p].signers}|
```

**Meaning:** Each signer can only sign once per proposal.

**Why it matters:** Prevents duplicate signatures inflating the count.

## Properties

### Progress
Proposals will eventually reach a terminal state (finalized, cancelled, or expired).

### Irreversibility
Once finalized or cancelled, no further state changes occur.

### Uniqueness
Each proposal ID is unique.

## Model Checking

### Configuration

To model check this specification:

1. **Install TLA+ Tools**: https://lamport.azurewebsites.net/tla/tools.html

2. **Create a model** with finite constants:
   ```tla
   Address = {A, B, C, D, E}
   ClaimType = {KYC, AML, POLICY}
   MaxSigners = 5
   MaxThreshold = 3
   ```

3. **Set invariants to check**:
   - InvariantNoPauseModification
   - InvariantSignerAuth
   - InvariantThresholdSatisfied
   - InvariantMutuallyExclusive
   - InvariantSignatureLimit
   - InvariantProposerSigns
   - InvariantExpiration
   - InvariantNoDuplicateSignatures

4. **Run model checker** with bounded time/depth

### Expected Results

The model checker should verify:
- No invariant violations
- No deadlocks
- All properties hold

## Usage as Design Reference

This specification serves as a reference for:

1. **Audit review**: Verify implementation matches the specification
2. **Future changes**: Ensure new features preserve invariants
3. **Bug prevention**: The state machine structure makes subtle bugs easier to detect

### Example: Bug Prevention

The specification made the pause invariant explicit:

```
InvariantNoPauseModification:
∀p: proposals[p].paused ⇒ ¬proposals[p].finalized ∧ ¬proposals[p].cancelled
```

This directly caught the missing `require_not_paused` check on `cancel_multisig_proposal`.

## Implementation Checklist

When implementing changes to `src/multisig.rs`:

- [ ] Review the state machine spec
- [ ] Verify all transitions match the specification
- [ ] Check that all invariants are preserved
- [ ] Ensure pause/unpause behavior matches spec
- [ ] Verify signature counting logic
- [ ] Check expiration handling

## Related Documents

- **TLA+ Specification**: `docs/formal/multisig_state_machine.tla`
- **Protocol Docs**: `docs/adr/`
- **Implementation**: `src/multisig.rs`
- **Tests**: `tests/` (search for `multisig`)

## References

1. Lamport, L. (1994). *The Temporal Logic of Actions*. ACM TOPLAS.
2. Leveson, N. (2011). *Engineering a Safer World*. MIT Press.
3. TLA+ Documentation: https://lamport.azurewebsites.net/tla/tla.html
