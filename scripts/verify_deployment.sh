#!/usr/bin/env bash
# verify_deployment.sh — End-to-end verification of a deployed TrustLink contract.
#
# Usage:
#   ./scripts/verify_deployment.sh [OPTIONS]
#
# Options:
#   --contract  CONTRACT_ID   Contract ID to verify (required)
#   --source    KEY_ALIAS     Stellar key alias for the admin/deployer (required)
#   --network   NETWORK       testnet | mainnet (default: testnet)
#
# Example:
#   ./scripts/verify_deployment.sh \
#     --contract CAK7PYYSWWQH6ML3ZPO4OB2EIONODOEESE3MIV3YGFDMHEU4EUOBUJQN \
#     --source deployer \
#     --network testnet
#
# Exit codes:
#   0 — all checks passed
#   1 — a verification step failed

set -euo pipefail

# ── Defaults ────────────────────────────────────────────────────────────────
NETWORK="testnet"
CONTRACT_ID=""
SOURCE=""

# ── Argument parsing ─────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    --contract) CONTRACT_ID="$2"; shift 2 ;;
    --source)   SOURCE="$2";      shift 2 ;;
    --network)  NETWORK="$2";     shift 2 ;;
    *) echo "Unknown option: $1"; exit 1 ;;
  esac
done

if [[ -z "$CONTRACT_ID" || -z "$SOURCE" ]]; then
  echo "Usage: $0 --contract CONTRACT_ID --source KEY_ALIAS [--network testnet|mainnet]"
  exit 1
fi

# ── Helpers ──────────────────────────────────────────────────────────────────
log()  { echo "[$(date -u +%H:%M:%S)] $*"; }
pass() { echo "[$(date -u +%H:%M:%S)] ✅ $*"; }
fail() { echo "[$(date -u +%H:%M:%S)] ❌ $*"; exit 1; }

invoke() {
  stellar contract invoke \
    --id "$CONTRACT_ID" \
    --source "$SOURCE" \
    --network "$NETWORK" \
    -- "$@"
}

# ── Derive admin address from key alias ──────────────────────────────────────
ADMIN_ADDRESS=$(stellar keys address "$SOURCE")
log "Admin address : $ADMIN_ADDRESS"
log "Contract ID   : $CONTRACT_ID"
log "Network       : $NETWORK"
log "──────────────────────────────────────────"

# ── Step 1: Verify get_admin ─────────────────────────────────────────────────
log "Step 1: Verifying get_admin..."
RETURNED_ADMIN=$(invoke get_admin | tr -d '"')
if [[ "$RETURNED_ADMIN" != "$ADMIN_ADDRESS" ]]; then
  fail "get_admin returned '$RETURNED_ADMIN', expected '$ADMIN_ADDRESS'"
fi
pass "get_admin returned correct admin address"

# ── Step 2: Generate a temporary test issuer identity ────────────────────────
log "Step 2: Generating temporary test issuer..."
TEST_ISSUER_ALIAS="trustlink_verify_issuer_$$"
stellar keys generate "$TEST_ISSUER_ALIAS" --network "$NETWORK" 2>/dev/null
TEST_ISSUER_ADDRESS=$(stellar keys address "$TEST_ISSUER_ALIAS")
log "Test issuer   : $TEST_ISSUER_ADDRESS"

# Fund the test issuer on testnet
if [[ "$NETWORK" == "testnet" ]]; then
  log "Funding test issuer via Friendbot..."
  curl -sf "https://friendbot.stellar.org/?addr=${TEST_ISSUER_ADDRESS}" -o /dev/null \
    || fail "Friendbot funding failed for $TEST_ISSUER_ADDRESS"
  pass "Test issuer funded"
fi

# ── Step 3: Register test issuer ─────────────────────────────────────────────
log "Step 3: Registering test issuer..."
invoke register_issuer \
  --admin "$ADMIN_ADDRESS" \
  --issuer "$TEST_ISSUER_ADDRESS" > /dev/null
pass "Test issuer registered"

# ── Step 4: Generate a temporary test subject identity ───────────────────────
log "Step 4: Generating temporary test subject..."
TEST_SUBJECT_ALIAS="trustlink_verify_subject_$$"
stellar keys generate "$TEST_SUBJECT_ALIAS" --network "$NETWORK" 2>/dev/null
TEST_SUBJECT_ADDRESS=$(stellar keys address "$TEST_SUBJECT_ALIAS")
log "Test subject  : $TEST_SUBJECT_ADDRESS"

# ── Step 5: Create a test attestation ────────────────────────────────────────
log "Step 5: Creating test attestation..."
ATTESTATION_ID=$(stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source "$TEST_ISSUER_ALIAS" \
  --network "$NETWORK" \
  -- create_attestation \
  --issuer "$TEST_ISSUER_ADDRESS" \
  --subject "$TEST_SUBJECT_ADDRESS" \
  --claim_type '"VERIFY_TEST"' \
  --expiration null \
  --metadata null \
  --tags null | tr -d '"')
log "Attestation ID: $ATTESTATION_ID"
pass "Test attestation created"

# ── Step 6: Verify has_valid_claim returns true ───────────────────────────────
log "Step 6: Verifying has_valid_claim returns true..."
CLAIM_RESULT=$(invoke has_valid_claim \
  --subject "$TEST_SUBJECT_ADDRESS" \
  --claim_type '"VERIFY_TEST"')
if [[ "$CLAIM_RESULT" != "true" ]]; then
  fail "has_valid_claim returned '$CLAIM_RESULT', expected 'true'"
fi
pass "has_valid_claim returned true"

# ── Step 7: Revoke the attestation ───────────────────────────────────────────
log "Step 7: Revoking test attestation..."
stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source "$TEST_ISSUER_ALIAS" \
  --network "$NETWORK" \
  -- revoke_attestation \
  --issuer "$TEST_ISSUER_ADDRESS" \
  --attestation_id "$ATTESTATION_ID" > /dev/null
pass "Test attestation revoked"

# ── Step 8: Verify has_valid_claim returns false ──────────────────────────────
log "Step 8: Verifying has_valid_claim returns false after revocation..."
CLAIM_RESULT=$(invoke has_valid_claim \
  --subject "$TEST_SUBJECT_ADDRESS" \
  --claim_type '"VERIFY_TEST"')
if [[ "$CLAIM_RESULT" != "false" ]]; then
  fail "has_valid_claim returned '$CLAIM_RESULT' after revocation, expected 'false'"
fi
pass "has_valid_claim returned false after revocation"

# ── Step 9: Cleanup temporary identities ─────────────────────────────────────
log "Step 9: Cleaning up temporary test identities..."
stellar keys rm "$TEST_ISSUER_ALIAS"  2>/dev/null || true
stellar keys rm "$TEST_SUBJECT_ALIAS" 2>/dev/null || true
pass "Cleanup complete"

# ── Done ─────────────────────────────────────────────────────────────────────
log "──────────────────────────────────────────"
pass "All verification steps passed. Contract is functioning correctly."
exit 0
