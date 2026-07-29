# Indexer Idempotency and Event Resilience

## Overview

The TrustLink indexer processes events from the Stellar blockchain to maintain an off-chain read model in PostgreSQL. Due to RPC reconnections, network delays, and backfill overlaps, events can arrive:

1. **Out of order** — events from a later ledger arrive before earlier ledgers
2. **Duplicated** — the same event is processed twice (RPC reconnection backfill)
3. **Both** — combined chaos scenario during network instability

This document describes the idempotency guarantees and how the indexer ensures database consistency despite these conditions.

## Event Idempotency Guarantees

### Attestation Events (`created`, `imported`, `bridged`)

**Idempotent pattern:** `upsert`

```typescript
const attestation = await db.attestation.upsert({
  where: { id },
  update: { subject, ...extra },
  create: { /* attestation */ },
});
```

**Behavior:**
- If attestation ID already exists → update only fields (no-op if unchanged)
- If attestation ID is new → create new record
- Processing the same event twice produces identical state

### Revocation Events (`revoked`)

**Idempotent pattern:** `updateMany` with conditional WHERE clause

```typescript
await db.attestation.updateMany({
  where: { id: attestationId, isRevoked: false },
  data: { isRevoked: true },
});
```

**Behavior:**
- Only updates records that are NOT already revoked
- Second duplicate revoke finds `isRevoked: true`, so no update occurs
- Processing the same revoke event multiple times results in exactly 1 state change

### Multi-Sig Proposals

All multi-sig handlers use idempotent patterns:
- `ms_prop`: `upsert` with no-op on duplicate
- `ms_sign`: Deduplicates signers before updating
- `ms_actv`: `updateMany` with `finalized: false` guard

### Attestation Requests

All handlers use status guards:
- `att_req`: `upsert` creates or no-op
- `req_ful`: `updateMany` transitions only from `PENDING`
- `req_rej`: `updateMany` transitions only from `PENDING`

### Endorsements

Uses composite unique key:

```typescript
await db.endorsement.upsert({
  where: { attestationId_endorser: { attestationId, endorser } },
  update: {},
  create: { /* endorsement */ },
});
```

## Out-of-Order Event Handling

### Limitation: Revoke-Before-Create

If a revoke event arrives before the corresponding create event:
- The revoke is a no-op (record doesn't exist to update)
- The create then makes the attestation appear active
- Result: Temporary inconsistency until reindex

**Mitigation:** Periodic reindex reconciles state with on-chain contract.

## Known Limitations

### Metric Counter Inflation

- `attestationsTotal` increments on every upsert (including no-op duplicates)
- Source of truth is on-chain `GlobalStats`, not indexer metrics

### Subscription Duplicate Notifications

If a duplicate event is processed, GraphQL subscribers may receive duplicate notifications. Subscribers must deduplicate client-side.

## Recommendations

1. **Run periodic reindex:** Cron job every 1-6 hours
2. **Monitor event lag:** Track time between ledger close and indexer processing
3. **Deduplicate in subscriptions:** Use event ID or attestation ID
4. **Watch for inconsistency:** Monitor attestations that should be revoked but appear active

## Testing

Run the chaos test suite:

```bash
npm run -w indexer test -- indexer-chaos.test.ts
```

---
