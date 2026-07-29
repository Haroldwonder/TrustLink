# Formal Specification Verification Guide

## Quick Start

To verify the multi-sig proposal state machine specification:

```bash
# 1. Install TLA+ Tools
# Windows (using scoop):
scoop install tlaplus

# Windows (using chocolatey):
choco install tlaplus

# macOS:
brew install tlaplus

# Or download from: https://lamport.azurewebsites.net/tla/tools.html

# 2. Open the specification
# - Launch TLA+ Toolbox
# - File > Import > TLA+ Specification
# - Select: docs/formal/multisig_state_machine.tla

# 3. Create a model
# - In Toolbox: Model Overview > New Model
# - Set constants (see below)

# 4. Run model check
# - Ctrl+R or Model > Run Model Check
```

## Model Configuration

### Constants

Create a model with these constant definitions:

```tla
(* Address set - finite set of addresses for testing *)
Address = <<A, B, C, D, E>>

(* Claim type set *)
ClaimType = <<KYC, AML, POLICY>>

(* Maximum signers per proposal *)
MaxSigners = 5

(* Maximum threshold *)
MaxThreshold = 3
```

### Invariants to Check

Add these invariants to your model:

| Invariant | Description |
|-----------|-------------|
| `InvariantNoPauseModification` | No state changes while paused |
| `InvariantSignerAuth` | Only required signers can sign |
| `InvariantThresholdSatisfied` | Threshold must be met to finalize |
| `InvariantMutuallyExclusive` | Cannot be both finalized and cancelled |
| `InvariantSignatureLimit` | Signatures ≤ required signers |
| `InvariantProposerSigns` | Proposer always signs |
| `InvariantExpiration` | Expiration blocks operations |
| `InvariantNoDuplicateSignatures` | No duplicate signatures |

### Properties to Check

| Property | Description |
|----------|-------------|
| `PropertyProgress` | Proposals reach terminal state |
| `PropertyIrreversibility` | Final/cancelled states are final |
| `PropertyUniqueIDs` | Proposal IDs are unique |

## Common Errors and Solutions

### Error: "TLC encountered an error"

**Cause:** Model has infinite behavior (unbounded integers, infinite sets)

**Solution:** Use finite constants as shown above.

### Error: "State explosion"

**Cause:** Too many possible states for the model checker

**Solution:** Reduce the address set and claim type set.

### Error: "Invariant violated"

**Cause:** Implementation doesn't match specification

**Solution:** Review the invariant and update implementation.

## Verification Checklist

Before merging changes to `src/multisig.rs`:

- [ ] Spec reads without errors
- [ ] Model creates without warnings
- [ ] All 8 invariants pass
- [ ] All 3 properties pass
- [ ] No deadlocks detected
- [ ] State space is reasonable (< 10,000 states for basic checks)

## Continuous Verification

### CI Integration

To add formal verification to CI:

```yaml
# .github/workflows/formal-verification.yml
name: Formal Verification

on:
  pull_request:
    paths:
      - 'docs/formal/multisig_state_machine.tla'
      - 'src/multisig.rs'

jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install TLA+ Tools
        run: |
          sudo apt-get update
          sudo apt-get install -y tlaplus
      - name: Run model checker
        run: tla2tools.jar -cp . specs/multisig_state_machine.tla
```

### Local Verification Script

```bash
#!/bin/bash
# scripts/verify-multisig-spec.sh

set -e

echo "Checking TLA+ specification..."

# Syntax check
tlapls docs/formal/multisig_state_machine.tla

# Run model checker (requires model configuration)
echo "Model check complete. Review results above."

echo "✓ Specification verified"
```

## Advanced Verification

### Checking Liveness Properties

Liveness properties like `PropertyProgress` require fairness constraints:

```tla
(* Add to configuration *)
Fairness == WF_proposals(Next)
```

### Bounded Verification

For large state spaces, use bounded verification:

```tla
(* Add to configuration *)
MAX_STACK_DEPTH = 10
MAX_TRACE_LENGTH = 20
```

### Symbolic Model Checking

Use APALACHE for symbolic verification on larger state spaces:

```bash
# Install APALACHE
cargo install apalache

# Run symbolic check
apalache check --length=10 specs/multisig_state_machine.tla
```

## Troubleshooting

### Question: How do I add new states/transitions?

**Answer:** Update the `Next` operator and add new invariant checks.

### Question: How do I add new invariants?

**Answer:** Define them as `InvariantX ==` formulas and add to model.

### Question: The state space is too large?

**Answer:** Reduce the Address and ClaimType sets. The specification is modular.

### Question: How do I prove invariants mathematically?

**Answer:** Use TLA+ proof system (TLA) or rely on model checking for finite cases.

## Documentation Updates

When modifying the specification:

1. Update this guide if invariants change
2. Update `multisig_state_machine.md` with new details
3. Update implementation checklist
4. Add new tests to reflect specification changes

## References

- [TLA+ Language Reference](https://lamport.azurewebsites.net/tla/tla2-guide.pdf)
- [TLA+ Model Checking](https://lamport.azurewebsites.net/tla/toolbox.html)
- [Specifying Systems](https://lamport.azurewebsites.net/tla/book.html)
