# Formal Specifications

This directory contains formal specifications for TrustLink's critical protocols.

## Multi-Sig Proposal Lifecycle Specification

### Files

- **multisig_state_machine.tla** - TLA+ specification of the multi-sig proposal state machine
- **multisig_state_machine.md** - Human-readable documentation of the specification

### Overview

The multi-signature proposal lifecycle is a state machine where proposals transition through:
1. **Proposed** - Initial state after `propose_attestation`
2. **Signed** - After signers have cosigned
3. **Finalized** - When threshold is reached
4. **Cancelled** - Cancelled by proposer
5. **Expired** - Time-based expiration

### State Machine Diagram

```
┌─────────────┐
│  Proposed   │  ← Initial state (auto-signs proposer)
└──────┬──────┘
       │ cosign()
       │
       ▼
┌─────────────┐
│   Signed    │  ← 0 < signatures < threshold
└──────┬──────┘
       │ finalize() / threshold reached
       ├───────────────┐
       │               │ cancel() / proposer cancels
       ▼               ▼
┌─────────────┐  ┌─────────────┐
│  Finalized  │  │  Cancelled  │  ← Terminal states
└─────────────┘  └─────────────┘
       │
       │ expire() / time expires
       ▼
┌─────────────┐
│   Expired   │  ← Terminal state
└─────────────┘
```

### Invariants Verified

The specification verifies these critical invariants:

| Invariant | Description |
|-----------|-------------|
| **NoPauseModification** | No state-changing transition succeeds while contract is paused |
| **SignerAuth** | Signature count never includes non-required signers |
| **ThresholdSatisfied** | Threshold is satisfied only when enough signatures exist |
| **MutuallyExclusive** | A proposal cannot be finalized and cancelled simultaneously |
| **SignatureLimit** | Signature count never exceeds required signers count |
| **ProposerSigns** | Proposer auto-signs on creation |
| **Expiration** | Expiration prevents all operations after expiry |
| **NoDuplicateSignatures** | Each signer can only sign once per proposal |

### Safety Properties

1. **Progress**: Proposals will eventually reach a terminal state (finalized, cancelled, or expired)
2. **Irreversibility**: Once finalized or cancelled, no further state changes occur
3. **Uniqueness**: Each proposal ID is unique

### Model Checking

To verify the specification:

1. Install [TLA+ Tools](https://lamport.azurewebsites.net/tla/tools.html)
2. Load `multisig_state_machine.tla` in the TLA+ Toolbox
3. Create a model with finite constants:
   - `Address`: {A, B, C, D, E}
   - `ClaimType`: {KYC, AML, POLICY}
   - `MaxSigners`: 5
   - `MaxThreshold`: 3
4. Check the invariants listed above

### Usage as Design Reference

This specification serves as a reference for:

- **Audit review**: Verify implementation matches the specification
- **Future changes**: Ensure new features preserve invariants
- **Bug prevention**: The state machine structure makes subtle bugs easier to detect

### Related Code

- Implementation: `src/multisig.rs`
- Test suite: `tests/` (search for `multisig`)
- Protocol docs: `docs/adr/ADR-*.md`

## Writing New Specifications

When adding new formal specifications:

1. Use TLA+ for protocol specifications
2. Document invariants and properties
3. Include model checking configuration
4. Reference the corresponding source code
5. Update this README with new specs
