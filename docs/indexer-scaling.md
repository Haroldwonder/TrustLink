# Indexer Horizontal Scaling & Sharding Guide

This document describes how to run multiple TrustLink indexer instances in
parallel to handle high attestation-creation throughput while keeping a single
consistent GraphQL read surface for clients.

---

## When do you need this?

A single indexer instance can sustain roughly **600–800 events/second** before
the PostgreSQL write path becomes the bottleneck (empirically, with the
bundled schema and a `db.r7g.large`-class RDS instance). If your projected
peak attestation rate exceeds that figure, or if you need sub-second query
latency during bursts, you should shard.

Typical signals that you have outgrown a single instance:

- `trustlink_indexer_lag_ledgers` Prometheus gauge grows monotonically during
  normal operation.
- PostgreSQL `pg_stat_activity` shows persistent write queue depth > 100.
- `processRange` spans in your tracing backend show median latency > 2 s.

---

## Architecture overview

```
   Stellar RPC (getEvents)
          │
          │  each shard calls getEvents with a disjoint ledger range OR
          │  a disjoint set of contract IDs (once multi-contract support lands)
          │
  ┌───────┴──────────────────────────┐
  │  Shard 0             Shard N     │  ← N indexer processes, each with its
  │  (ledgers 0–999,999) (1M–1.999M) │    own START_LEDGER / END_LEDGER env vars
  └──────────────────────────────────┘
          │
     Shared PostgreSQL  ←── all shards write to the same DB (upsert-safe)
          │
   Single GraphQL API   ←── one read-only replica or pooled connection
```

The key invariant is that **each shard owns a disjoint portion of the event
stream** so writes never conflict. Reads are always served from the shared DB,
so clients see a unified view regardless of which shard produced a record.

---

## Sharding strategies

### Strategy A: Ledger-range sharding (recommended)

Divide the ledger sequence space into non-overlapping bands and assign one
band to each indexer instance.

```
Shard 0:  START_LEDGER=0          END_LEDGER=999999
Shard 1:  START_LEDGER=1000000    END_LEDGER=1999999
Shard 2:  START_LEDGER=2000000    (open — live tip)
```

Only the highest-numbered shard needs to follow the live tip; lower shards
catch up once and then idle (or process historical re-indexes).

**Pros:** Simple, no coordination required between shards.  
**Cons:** You must re-assign ranges manually as Stellar ledger numbers grow.
Hot periods are handled only by the tip shard; lower shards sit idle once
backfill is done.

#### Example Kubernetes Deployment (3-shard ledger split)

```yaml
# indexer-shard-0.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: indexer-shard-0
spec:
  replicas: 1
  template:
    spec:
      containers:
        - name: indexer
          image: ghcr.io/afurious/trustlink-indexer:latest
          env:
            - name: START_LEDGER
              value: "0"
            - name: END_LEDGER       # set END_LEDGER to freeze the shard at backfill
              value: "999999"
            - name: DATABASE_URL
              valueFrom:
                secretKeyRef:
                  name: indexer-secrets
                  key: DATABASE_URL
---
# indexer-shard-live.yaml  — follows the live tip
apiVersion: apps/v1
kind: Deployment
metadata:
  name: indexer-shard-live
spec:
  replicas: 1
  template:
    spec:
      containers:
        - name: indexer
          image: ghcr.io/afurious/trustlink-indexer:latest
          env:
            - name: START_LEDGER
              value: "2000000"     # beginning of live window
            - name: DATABASE_URL
              valueFrom:
                secretKeyRef:
                  name: indexer-secrets
                  key: DATABASE_URL
```

Each shard writes its own `Checkpoint` row. To avoid the singleton `id=1`
conflict, set `SHARD_ID` and use `id = SHARD_ID` for the checkpoint:

```typescript
// indexer.ts — replace hardcoded id: 1 with:
const SHARD_ID = parseInt(process.env.SHARD_ID ?? "1", 10);

await db.checkpoint.upsert({
  where:  { id: SHARD_ID },
  update: { ledger: lastProcessed },
  create: { id: SHARD_ID, ledger: lastProcessed },
});
```

Apply a corresponding migration to drop the `@default(1)` on `Checkpoint.id`.

---

### Strategy B: Issuer-based sharding

Route events to shards by hashing the issuer address. Each shard only calls
`handleEvent` for events whose issuer falls in its hash-bucket.

```typescript
// Shard selection — each instance sets SHARD_INDEX and SHARD_COUNT
const SHARD_INDEX = parseInt(process.env.SHARD_INDEX ?? "0", 10);
const SHARD_COUNT = parseInt(process.env.SHARD_COUNT ?? "1", 10);

function ownsEvent(issuerAddress: string): boolean {
  if (SHARD_COUNT === 1) return true;
  let hash = 0;
  for (const ch of issuerAddress) hash = (hash * 31 + ch.charCodeAt(0)) >>> 0;
  return hash % SHARD_COUNT === SHARD_INDEX;
}
```

All shards still **fetch the same events from the RPC** (Stellar has no
server-side issuer filter), but each shard skips events it doesn't own.

**Pros:** All shards stay at the live tip; latency is uniform across all
issuers; no manual range rotation.  
**Cons:** All shards issue identical RPC calls (wasted network bandwidth).
Requires consistent hashing if shard count changes. An issuer that generates
bursts will still saturate a single shard.

---

### Strategy C: Contract-ID sharding (future — multi-contract support)

Once the indexer supports `CONTRACT_IDS` (plural) instead of a single
`CONTRACT_ID`, assign each shard a disjoint subset of contract addresses. This
is the lowest-coordination approach because Stellar's `getEvents` supports
filtering by `contractIds`.

This strategy is not yet available but the `processRange` loop is structured
to support it — the `CONTRACT_ID` constant would simply become a per-shard
environment variable from the contract pool.

---

## Checkpoint schema migration for multi-shard

```sql
-- migration: make Checkpoint support multiple shards
ALTER TABLE "Checkpoint" DROP CONSTRAINT "Checkpoint_pkey";
ALTER TABLE "Checkpoint" ADD COLUMN "shard_id" INTEGER NOT NULL DEFAULT 1;
ALTER TABLE "Checkpoint" ADD PRIMARY KEY ("shard_id");
```

Or via Prisma migration:

```prisma
model Checkpoint {
  shardId Int @id   // 1-based shard identifier; default 1 for single-instance deployments
  ledger  Int
}
```

---

## Shared GraphQL read surface

All shards write to the same PostgreSQL database. The GraphQL API reads from a
**read replica** (or a connection pool pointing at the primary) and is unaware
of sharding:

```
                        ┌──────────────────────┐
   Client → GraphQL API │  indexer/src/index.ts │
                        │  (read-only replica)  │
                        └──────────┬───────────┘
                                   │  SELECT
                                   ▼
                          Shared PostgreSQL
                          ▲       ▲       ▲
                      Shard 0  Shard 1  Shard 2  (each INSERT/UPDATE its own ranges)
```

Set `GRAPHQL_ONLY=true` in the API process to skip starting the indexer loop:

```typescript
// index.ts
if (!process.env.GRAPHQL_ONLY) {
  await startIndexer(db, redis);
}
```

This lets you deploy separate indexer pods and a separate API pod that never
runs `processRange`.

---

## Interaction with archival

Each shard independently triggers its archival job via `scheduleArchivalJob`.
This can lead to duplicate archive attempts for the same ledger range. To
prevent conflicts:

1. Use the `ArchivedEventBatch.fromLedger / toLedger` unique index to detect
   already-archived ranges before writing.
2. Or designate a single **archival coordinator** shard (e.g. `SHARD_INDEX=0`)
   that runs the archival job; all other shards set
   `ARCHIVAL_INTERVAL_HOURS=0` to disable archival.

---

## Interaction with reconciliation

The reconciliation endpoint (`/admin/reconcile`) compares the indexer DB
against the chain. In a sharded deployment, trigger reconciliation once per
shard passing `START_LEDGER` and `END_LEDGER` bounds matching that shard's
ownership range. Avoid running concurrent reconciliation jobs over overlapping
ledger ranges.

---

## Dead-letter queue in multi-shard deployments

Each shard writes failed events to the shared `EventDeadLetter` table (added
in #975). The `eventType` and `ledger` columns let you identify which shard
produced a failure without needing shard-specific tables. A future
`shardId` column can be added if per-shard filtering becomes necessary.

To reprocess dead-letter events, reset `status` to `PENDING` and restart (or
hotpatch) the owning shard:

```sql
UPDATE event_dead_letters
SET status = 'PENDING', "updatedAt" = NOW()
WHERE status = 'RETRYING'
  AND ledger BETWEEN 0 AND 999999;  -- shard 0's range
```

---

## Recommended configuration by deployment tier

| Tier             | Attestations/day | Shards | Strategy          | DB instance   |
|------------------|-----------------|--------|-------------------|---------------|
| Development      | < 10k           | 1      | Single instance   | db.t3.micro   |
| Small production | 10k–500k        | 1      | Single instance   | db.r7g.large  |
| Medium           | 500k–5M         | 2–4    | Ledger-range      | db.r7g.xlarge |
| Large            | 5M+             | 4–8    | Issuer-hash       | db.r7g.2xlarge + read replica |
| Very large       | 50M+            | 8+     | Contract-ID (future) | Aurora cluster |

---

## Operational checklist for a new sharded deployment

- [ ] Assign non-overlapping `START_LEDGER` / `END_LEDGER` (or `SHARD_INDEX` /
      `SHARD_COUNT`) to each pod.
- [ ] Set a unique `SHARD_ID` per pod and apply the Checkpoint migration.
- [ ] Deploy the GraphQL API as a separate `GRAPHQL_ONLY=true` process pointing
      at the read replica.
- [ ] Configure Prometheus to scrape all shards; add a `shard` label via
      `SHARD_ID` so per-shard lag is visible in Grafana.
- [ ] Confirm `trustlink_indexer_lag_ledgers` converges to ≤ 1 on all shards.
- [ ] Verify dead-letter table is empty (or has an acceptable backlog) after
      initial backfill.
- [ ] Set up a Slack/PagerDuty alert when `event_dead_letters` row count
      exceeds a threshold (suggested: 100 PENDING rows).

---

## Further reading

- [docs/monitoring.md](monitoring.md) — Prometheus metrics and Grafana
  dashboards.
- [docs/canary-deployment.md](canary-deployment.md) — Progressive rollout
  strategy for indexer upgrades.
- [docs/indexer-idempotency.md](indexer-idempotency.md) — Why upserts make
  multi-shard writes safe.
- [ADR-011](adr/ADR-011-graphql-federation-schema-versioning.md) — Schema
  versioning and future GraphQL federation strategy.
