# TrustLink Disaster Recovery Guide

This document provides procedures and timelines for recovering from major operational failures affecting TrustLink on Stellar mainnet.

**Target Recovery Times (RTO):**
- Indexer database loss: 30 minutes
- RPC provider outage: 15 minutes
- Compromised admin key: 4 hours (key rotation + re-initialization)

---

## Overview

The TrustLink system has three critical components:

| Component | Purpose | Failure impact | Recovery strategy |
|-----------|---------|-----------------|------------------|
| **Smart Contract** | On-chain attestation logic | Cannot create/revoke attestations | N/A — immutable once deployed |
| **Indexer** | Off-chain event stream and query API | Cannot query attestations; events not indexed | Rebuild from on-chain logs |
| **RPC Provider** | Blockchain access (read/write) | Cannot deploy or initialize | Switch to backup RPC |
| **Admin Key** | Contract administration | Cannot manage issuers or fees; attestations still valid | Key rotation procedure |

---

## Scenario 1: Indexer Database Loss

**Detection:** Indexer health check fails, lag increases, or dashboard returns no data.

**Impact:** Query API down; attestation data unavailable to applications. On-chain data is safe.

**RTO:** 30 minutes

### Recovery Procedure

#### Step 1 — Verify database is indeed lost (2 min)

```bash
# Check indexer logs
kubectl logs -f deployment/trustlink-indexer

# Expected error:
# "Cannot connect to database" or "Database corrupted"

# Try to query a known contract event
curl -X POST http://indexer-api:4000/graphql \
  -H "Content-Type: application/json" \
  -d '{ "query": "{ attestations(first: 1) { id } }" }'

# Expected: Connection refused or empty result
```

#### Step 2 — Stop the indexer (1 min)

```bash
kubectl scale deployment trustlink-indexer --replicas=0

# Verify stopped
kubectl get pods -l app=trustlink-indexer
# Expected: No running pods
```

#### Step 3 — Snapshot the RPC blockchain state (2 min)

Before rebuilding, record the current ledger height so you can replay events from a known point:

```bash
# Get the current ledger sequence
curl -s http://rpc-provider.example.com/soroban/rpc -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger","params":[]}' \
  | jq '.result.sequence'

# Export this value — you will use it if you need to replay from a specific ledger
export CURRENT_LEDGER=<value>
echo "Current ledger: $CURRENT_LEDGER" >> /var/log/trustlink-recovery.log
```

#### Step 4 — Recreate the database (3 min)

```bash
# Drop existing database
kubectl exec -it postgresql-pod -- psql -U postgres -c "DROP DATABASE trustlink_indexer;"

# Recreate schema
kubectl exec -it postgresql-pod -- psql -U postgres -c "CREATE DATABASE trustlink_indexer OWNER indexer_user;"

# Apply migrations
cd indexer && npm run migrate
```

#### Step 5 — Reset indexer cursor (1 min)

The indexer tracks the last processed ledger in its state. Clear it to force a replay from genesis:

```bash
# Connect to indexer database
kubectl exec -it postgresql-pod -- psql -U indexer_user -d trustlink_indexer

-- Clear the indexer progress
DELETE FROM indexer_state;

-- Exit
\q
```

Alternatively, if you want to start from the current ledger to save time:

```sql
-- Set cursor to current ledger (faster recovery)
INSERT INTO indexer_state (id, last_processed_ledger) VALUES (1, $CURRENT_LEDGER);
```

#### Step 6 — Restart the indexer (5 min)

```bash
kubectl scale deployment trustlink-indexer --replicas=1

# Watch logs for event processing
kubectl logs -f deployment/trustlink-indexer --tail=50

# Expected: "Starting indexer from ledger X" → "Processing events..."
```

#### Step 7 — Verify recovery (2 min)

```bash
# Query attestations endpoint
curl -X POST http://indexer-api:4000/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "{ attestations(first: 10) { id issuer subject claimType } }"
  }'

# Verify against on-chain contract
soroban contract invoke --id <CONTRACT_ID> --network mainnet \
  -- get_global_stats

# Confirm indexer total_attestations matches contract total_attestations
```

**Recovery complete when:**
- Indexer logs show "Processing events..." (not lagging)
- Query API returns attestations
- Indexer stats match contract stats

---

## Scenario 2: RPC Provider Outage

**Detection:** Contract invocations fail with "connection refused" or persistent timeout errors.

**Impact:** Cannot read on-chain state; cannot create/revoke attestations. Attestations remain valid on-chain.

**RTO:** 15 minutes

### Prerequisites

Maintain a list of backup RPC providers and update it quarterly:

```bash
# docs/rpc-failover-list.md (example)
1. Primary: https://soroban-mainnet.stellar.org (Stellar Foundation)
2. Backup 1: https://rpc.soroban.io (Stellar Development Foundation)
3. Backup 2: Private RPC node in-house (if available)
```

### Recovery Procedure

#### Step 1 — Confirm RPC outage (1 min)

```bash
# Test primary RPC endpoint
curl -s https://soroban-mainnet.stellar.org/soroban/rpc \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger","params":[]}' \
  | jq '.result'

# Expected on failure: null or connection error
# Expected on success: {"sequence": 123456, "protocolVersion": 20, ...}
```

#### Step 2 — Switch to backup RPC (2 min)

Update the environment variable or config file used by clients and the indexer:

```bash
# For indexer
kubectl set env deployment/trustlink-indexer \
  RPC_URL=https://rpc.soroban.io

# For CLI tools / local testing
export SOROBAN_RPC_URL=https://rpc.soroban.io

# For contract invocations
export SOROBAN_RPC_HOST=rpc.soroban.io
export SOROBAN_RPC_PORT=443
```

#### Step 3 — Test backup RPC (3 min)

```bash
# Verify backup is responding
curl -s https://rpc.soroban.io/soroban/rpc \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger","params":[]}' \
  | jq '.result.sequence'

# Invoke contract to verify full read/write access
soroban contract invoke --id <CONTRACT_ID> --network mainnet \
  -- get_admin

# Expected: Returns admin address
```

#### Step 4 — Restart dependent services (5 min)

Restart services that cache RPC connections to ensure they pick up the new endpoint:

```bash
# Restart indexer
kubectl rollout restart deployment/trustlink-indexer

# Restart API gateway (if applicable)
kubectl rollout restart deployment/trustlink-api

# Watch logs
kubectl logs -f deployment/trustlink-indexer --tail=20
```

#### Step 5 — Monitor health (4 min)

```bash
# Check indexer is processing
curl http://localhost:4000/health | jq '.status'

# Perform a test transaction
soroban contract invoke --id <CONTRACT_ID> --network mainnet \
  -- get_global_stats

# Confirm statistics are current
```

**Recovery complete when:**
- Backup RPC responds to requests
- Contract invocations succeed
- Indexer is processing events again

**Escalation:** If backup RPC is also degraded, contact Stellar Foundation or spin up a private RPC node (1–2 hours).

---

## Scenario 3: Compromised Admin Key

**Detection:** Unauthorized `register_issuer`, `remove_issuer`, or `set_fee` invocations; or physical key compromise (e.g., hardware wallet stolen).

**Impact:** Attacker can modify fee settings, add rogue issuers, or lock out legitimate issuers. Existing attestations remain valid.

**RTO:** 4 hours (key rotation + re-initialization + verification)

### Prerequisites

Before mainnet deployment, establish:

1. **Backup admin key:** Generate and store securely offline (e.g., paper backup, separate hardware wallet).
2. **Rotation procedure:** Document the steps to generate a new key pair and transfer admin role.
3. **Stakeholder notification:** List all external parties who need to be informed (indexer operators, issuer partners, monitoring).

### Recovery Procedure

#### Step 1 — Contain the breach (5 min)

If the admin key is physically compromised (e.g., hardware wallet stolen), immediately:

```bash
# Announce to team immediately
# Do NOT use the compromised key for any further operations

# Document the timeline
echo "Key compromised at: $(date)" >> /var/log/trustlink-incident.log
```

#### Step 2 — Generate a new admin key pair (5 min)

Generate a fresh key using a secure, offline process:

```bash
# Option A: Hardware wallet (recommended)
# Use Ledger, Trezor, or equivalent to generate a new Stellar account
# Export the public key only

# Option B: Local key generation (if hardware wallet unavailable)
# Use Soroban CLI with appropriate security:
stellar keys generate admin-new --show-seed

# Output:
# Public Key: GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
# Secret Key: SXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX

# CRITICAL: Store the secret key offline. Never commit to version control.
```

#### Step 3 — Fund the new admin account (10 min)

The new admin account must have enough XLM for transaction fees:

```bash
# Option A: Transfer from a trusted source
soroban transfer \
  --source-account <TRUSTED_ACCOUNT> \
  --destination <NEW_ADMIN_PUBLIC> \
  --amount 100  # sufficient for ~500 transactions

# Option B: Have an issuer or partner transfer funds temporarily

# Verify funding
soroban account balance --account <NEW_ADMIN_PUBLIC> --network mainnet
# Expected: >= 1 XLM
```

#### Step 4 — Transfer admin role (30 min)

Unfortunately, **Soroban does not yet support admin role transfer directly via contract logic**. The current workaround requires re-deploying the contract with the new admin key or using a bridge/governance contract.

**Current option:** Contact Stellar Foundation for guidance on admin key rotation procedures, or:

**Workaround (for immediate operations):**
1. Designate a temporary issuer as a "quasi-admin" to continue operations.
2. Plan for full contract re-initialization in the next mainnet upgrade cycle.

```bash
# Register the new key as an issuer (temporary measure)
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source "$COMPROMISED_ADMIN_SECRET" \
  --network mainnet \
  -- register_issuer \
  --admin <COMPROMISED_ADMIN_PUBLIC> \
  --issuer <NEW_ADMIN_PUBLIC>

# Issue emergency guidance to partners that operations are restricted
```

#### Step 5 — Update all dependent systems (20 min)

Update every system that holds the old admin key:

```bash
# Update CI/CD secrets
# GitHub Actions / GitLab CI: Update ADMIN_SECRET environment variable in settings

# Update deployment tools
sed -i "s|$OLD_ADMIN_SECRET|$NEW_ADMIN_SECRET|g" ~/.config/trustlink/deploy.env

# Restart services that cache the key
# Note: Services should only read secrets from secure stores, never hardcoded

# Verify no traces of old key in logs or configs
grep -r "$OLD_ADMIN_SECRET" ~/.config/ /var/log/ 2>/dev/null || echo "Key not found in configs/logs"
```

#### Step 6 — Notify stakeholders (15 min)

```bash
# Email all issuer partners, indexer operators, monitoring teams:

Subject: TrustLink Admin Key Rotation - ACTION REQUIRED

Dear Partners,

We have rotated the TrustLink admin key due to [REASON: e.g., "security incident", "routine rotation"].

Old admin address: GXXX...
New admin address: GYYY...

Action required:
- If you invoke contract admin functions, update your ADMIN_SECRET environment variable.
- No changes to existing attestations or on-chain data.
- Continue normal operations.

Recovery will be complete by [TIME].

Contact: security@trustlink.io
```

#### Step 7 — Verify operations (30 min)

```bash
# Test a standard operation with new admin key
export ADMIN_SECRET=$NEW_ADMIN_SECRET

# Register a test issuer
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source "$ADMIN_SECRET" \
  --network mainnet \
  -- register_issuer \
  --admin <NEW_ADMIN_PUBLIC> \
  --issuer GTEST1234567890123456789012345678901234567890

# Verify admin is correctly set
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network mainnet \
  -- get_admin

# Expected: <NEW_ADMIN_PUBLIC>

# Remove the test issuer
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source "$ADMIN_SECRET" \
  --network mainnet \
  -- remove_issuer \
  --admin <NEW_ADMIN_PUBLIC> \
  --issuer GTEST1234567890123456789012345678901234567890
```

#### Step 8 — Post-incident review (60 min, scheduled later)

Schedule a post-incident review for within 24 hours:

```markdown
# Post-Incident Review Checklist

- [ ] Timeline reconstruction: when was key first compromised?
- [ ] Scope assessment: which operations were performed with the compromised key?
- [ ] Attestation integrity: did the attacker create, modify, or revoke any attestations?
- [ ] Root cause: how was the key compromised (physical theft, etc.)?
- [ ] Preventive measures: what changes prevent recurrence?
- [ ] Documentation updates: reflect learnings in security.md and runbooks
- [ ] Team debrief scheduled
```

**Recovery complete when:**
- New admin key is in use for all operations
- All stakeholders have been notified
- Operations have resumed normally
- Monitoring confirms no unauthorized activity

---

## Disaster Recovery Drill Checklist

Use this checklist to execute and document annual drills of each scenario.

### Drill Scheduling

| Scenario | Frequency | Owner | Scheduled date |
|----------|-----------|-------|-----------------|
| Indexer DB loss | Q2 | DevOps Lead | _____ |
| RPC provider outage | Q3 | DevOps Lead | _____ |
| Compromised admin key | Q4 | Security Lead | _____ |

### Pre-Drill Setup (30 min before)

- [ ] Notify all team members of drill (non-production impact)
- [ ] Set up Slack channel: `#trustlink-dr-drill`
- [ ] Designate incident commander and scribe
- [ ] Prepare: timers, backup RPC URLs, recovery tools
- [ ] Take baseline snapshots of indexer stats and contract state
- [ ] Ensure drill environment isolated from production (testnet preferred)

### Indexer DB Loss Drill

**Scenario:** Indexer database is corrupted; rebuild from blockchain logs.

**Setup:**
```bash
# Simulate on testnet (safer than mainnet)
export CONTRACT_ID=<TESTNET_CONTRACT_ID>
export NETWORK=testnet

# Create a baseline of known attestations
soroban contract invoke --id $CONTRACT_ID --network $NETWORK \
  -- get_global_stats
# Record: total_attestations, total_revocations
```

**Drill steps (follow Scenario 1 above):**

1. [ ] Stop indexer (1 min)
2. [ ] Snapshot RPC state (2 min)
3. [ ] Recreate database (3 min)
4. [ ] Reset cursor (1 min)
5. [ ] Restart indexer (5 min)
6. [ ] Verify recovery (2 min)

**Verification:**
```bash
# Confirm indexer recovered all events
curl -X POST http://indexer-api:4000/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ attestations { totalCount } }"}'

# Compare to baseline
# Expected: totalCount == original total_attestations
```

**Results:**
- [ ] All steps completed within 15 minutes
- [ ] Indexer recovered all attestations
- [ ] No data loss
- [ ] Document any deviations in comments below

**Comments:**
```
Drill date: ________________
Incident commander: ________________
Actual recovery time: __________ min (target: 30 min)
Issues encountered: 
  - [List any blockers or unexpected issues]

Improvements for next drill:
  - [List learnings and improvements]
```

### RPC Provider Outage Drill

**Scenario:** Primary RPC is unavailable; switch to backup.

**Setup:**
```bash
# Pre-drill: verify backup RPC is accessible
curl -s https://rpc.soroban.io/soroban/rpc \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger","params":[]}' | jq '.result'

# Record baseline
export PRIMARY_RPC=https://soroban-mainnet.stellar.org
export BACKUP_RPC=https://rpc.soroban.io
```

**Drill steps (follow Scenario 2 above):**

1. [ ] Confirm RPC outage (simulate by blocking DNS or firewall) (1 min)
2. [ ] Switch to backup RPC (2 min)
3. [ ] Test backup RPC (3 min)
4. [ ] Restart services (5 min)
5. [ ] Monitor health (4 min)

**Verification:**
```bash
# Confirm operations on backup RPC
soroban contract invoke --id $CONTRACT_ID --network mainnet \
  -- get_admin

# Expected: Returns admin address from backup RPC
```

**Results:**
- [ ] All steps completed within 15 minutes
- [ ] Contract operations succeed on backup RPC
- [ ] No loss of functionality
- [ ] Document any deviations below

**Comments:**
```
Drill date: ________________
Incident commander: ________________
Actual failover time: __________ min (target: 15 min)
Backup RPC tested: ________________
Issues encountered:
  - [List any blockers or unexpected issues]

Improvements for next drill:
  - [List learnings and improvements]
```

### Compromised Admin Key Drill

**Scenario:** Admin key is compromised; rotate to a new key and verify operations.

**Setup (testnet only):**
```bash
# Do NOT perform this drill on mainnet with real keys
export NETWORK=testnet
export CONTRACT_ID=<TESTNET_CONTRACT_ID>

# Prepare: new admin key (do not use in production)
stellar keys generate admin-new-drill --show-seed
# Output: Public key and secret key for the drill
```

**Drill steps (follow Scenario 3 above):**

1. [ ] Contain the breach (log incident) (5 min)
2. [ ] Generate new admin key (5 min)
3. [ ] Fund new admin account (10 min)
4. [ ] Attempt to transfer admin role (30 min) — NOTE: may not be fully possible yet
5. [ ] Update dependent systems (20 min)
6. [ ] Notify stakeholders (simulate) (5 min)
7. [ ] Verify operations with new key (30 min)

**Verification:**
```bash
# With new key, attempt standard admin operations:

# Register a test issuer
soroban contract invoke \
  --id $CONTRACT_ID \
  --source "$NEW_ADMIN_SECRET_DRILL" \
  --network testnet \
  -- register_issuer \
  --admin <NEW_ADMIN_PUBLIC> \
  --issuer GTEST1234567890

# Confirm it was registered
soroban contract invoke \
  --id $CONTRACT_ID \
  --network testnet \
  -- is_issuer \
  --issuer GTEST1234567890
# Expected: true

# Clean up: remove test issuer
soroban contract invoke \
  --id $CONTRACT_ID \
  --source "$NEW_ADMIN_SECRET_DRILL" \
  --network testnet \
  -- remove_issuer \
  --admin <NEW_ADMIN_PUBLIC> \
  --issuer GTEST1234567890
```

**Results:**
- [ ] New admin key generated securely
- [ ] New admin can execute all operations
- [ ] Old key is no longer used
- [ ] Team aware of key rotation procedure
- [ ] Document any deviations below

**Comments:**
```
Drill date: ________________
Incident commander: ________________
Total drill time: __________ min (target: <180 min)
Challenges encountered:
  - [List any blockers — especially around admin role transfer]

Improvements for next drill:
  - [List learnings]

Recommendations for production:
  - [Any changes to process, tooling, or documentation]
```

---

## General Guidelines

### Before Disaster Strikes

1. **Test recovery procedures quarterly.** Use the drill checklist above.
2. **Maintain backup RPC list.** Update quarterly and test each provider.
3. **Secure admin key.** Store on hardware wallet; keep backup offline.
4. **Document contacts.** Keep incident commander and escalation list current.
5. **Automate health checks.** Monitor indexer lag, RPC uptime, and contract stats.

### During Recovery

1. **Activate incident command.** Assign one person to lead; others support.
2. **Use a status channel.** Post updates to Slack / Discord every 5 minutes.
3. **Follow the procedure.** Don't skip steps or deviate without approval.
4. **Verify each step.** Confirm recovery before moving to the next step.
5. **Document everything.** Log commands, outputs, and timing for post-incident review.

### After Recovery

1. **Post-incident review.** Schedule within 24 hours; include full team.
2. **Update runbooks.** Incorporate learnings and new procedures.
3. **Inform stakeholders.** Send a summary of what happened and what changed.
4. **Schedule next drill.** Plan the next scenario drill immediately.

---

## Contacts & Escalation

| Role | Name | Email | Phone | Notes |
|------|------|-------|-------|-------|
| Incident Commander | _____ | _____ | _____ | |
| DevOps Lead | _____ | _____ | _____ | |
| Security Lead | _____ | _____ | _____ | |
| Stellar Foundation POC | _____ | _____ | _____ | RPC/network support |
| Issuer Partner (1) | _____ | _____ | _____ | |
| Issuer Partner (2) | _____ | _____ | _____ | |

---

## Related Documentation

- [docs/mainnet-runbook.md](./mainnet-runbook.md) — Standard deployment and initialization
- [docs/mainnet-checklist.md](./mainnet-checklist.md) — Pre-deployment verification
- [docs/monitoring.md](./monitoring.md) — Health checks and alerting
- [docs/security.md](./security.md) — Trust hierarchy and threat model
- [docs/key-rotation-runbook.md](./key-rotation-runbook.md) — Key management

---

## Version History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-06-27 | Disaster Recovery Team | Initial release |

---

*Last updated: 2026-06-27*  
*Next drill: [Quarter] 20__*
