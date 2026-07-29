#!/bin/bash
# Verification script for TLA+ specification

set -e

echo "======================================"
echo "Multi-Sig State Machine Verification"
echo "======================================"
echo ""

# Check if tlaplc is available
if ! command -v tlaplc &> /dev/null; then
    echo "⚠️  TLA+ Tools (tlaplc) not found."
    echo "   Install from: https://lamport.azurewebsites.net/tla/tools.html"
    echo ""
    echo "Note: Syntax checking requires tlaplc."
    echo "The specification can still be reviewed manually."
    echo ""
    echo "Skipping syntax check..."
    echo ""
else
    echo "✅ TLA+ Tools found"
fi

# Check if spec file exists
if [ ! -f "multisig_state_machine.tla" ]; then
    echo "❌ Specification file not found"
    exit 1
fi

echo "✅ Specification file found: multisig_state_machine.tla"
echo ""

# Count invariants
INFRA_COUNT=$(grep -c "^Invariant" multisig_state_machine.tla || true)
echo "📊 Invariants defined: $INFRA_COUNT"

# Count properties
PROP_COUNT=$(grep -c "^Property" multisig_state_machine.tla || true)
echo "📊 Properties defined: $PROP_COUNT"

# Count transitions
TRANS_COUNT=$(grep -c "^Can[A-Z]" multisig_state_machine.tla || true)
echo "📊 Transitions defined: $TRANS_COUNT"

echo ""
echo "======================================"
echo "Specification Summary"
echo "======================================"
echo ""
echo "States:"
echo "  - Proposed: Initial state"
echo "  - Signed: After cosignatures"
echo "  - Finalized: Threshold reached"
echo "  - Cancelled: By proposer"
echo "  - Expired: Time-based"
echo ""
echo "Invariants:"
echo "  1. NoPauseModification"
echo "  2. SignerAuth"
echo "  3. ThresholdSatisfied"
echo "  4. MutuallyExclusive"
echo "  5. SignatureLimit"
echo "  6. ProposerSigns"
echo "  7. Expiration"
echo "  8. NoDuplicateSignatures"
echo ""
echo "Properties:"
echo "  1. Progress"
echo "  2. Irreversibility"
echo "  3. UniqueIDs"
echo ""

if command -v tlaplc &> /dev/null; then
    echo "======================================"
    echo "Syntax Check"
    echo "======================================"
    tlaplc -check multisig_state_machine.tla
    echo ""
    echo "✅ Syntax check passed"
else
    echo "Run 'make check' after installing TLA+ Tools for syntax verification."
fi

echo ""
echo "======================================"
echo "For full verification:"
echo "======================================"
echo "1. Install TLA+ Tools"
echo "2. Load in TLA+ Toolbox"
echo "3. Configure model with finite constants"
echo "4. Run model checker"
echo ""
echo "See VERIFICATION.md for detailed instructions."
