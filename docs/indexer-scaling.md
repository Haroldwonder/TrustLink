# Indexer Horizontal Scaling and Sharding Guide

This document explains how to run multiple TrustLink indexer workers in parallel
for high-throughput attestation deployments, while keeping a single consistent
GraphQL read surface for your clients.

---

## When do you need this?

A single indexer worker polling one RPC node can comfortably handle several
hundred attestation-creation events per ledger (≈5-second blocks). If your
deployment expects:

- **> 500 attestation events / ledger**, or
- **> 10,000 attestations / day**, or
- **multiple Stellar contracts** being indexed simultaneously,

then horizontal sharding across multiple workers is recommended.

---

## Architecture overview

```
         ┌──────────────────────────────────────────────────┐
         │                Stellar Network / RPC              │
         └───────────────────────┬──────────────────────────┘
                                 │ getEvents (per contract)
         ┌───────────────────────▼──────────────────────────┐
         │            Shared PostgreSQL database             │
         │  (Attestation, Checkpoint_*, EventDeadLetter …)  │
         └──────┬──────────────────────────┬────────────────┘
                │                          │
  ┌─────────────▼──────────┐  ┌────────────▼────────────────┐
  │   Indexer Worker A     │  │   Indexer Worker B          │
  │  Ledger range 0–1M     │  │  Ledger range 1M+           │
  │  CONTRACT_ID=C_MAIN    │  │  CONTRACT_ID=C_MAIN         │
  └────────────────────────┘  └─────────────────────────────┘
                │                          │
         ┌──────▼──────────────────────────▼──────┐
         │         GraphQL / REST read layer        │
         │  (read-only, connects to same Postgres)  │
         └──────────────────────────────────────────┘
```

The key insight is that **write workers** (polling RPC, writing attestations to
Postgres) are separated from the **read layer** (GraphQL server). Workers share
the same database but own disjoint ledger ranges or disjoint contracts.

---

## Sharding strategies

### Strategy 1: Ledger-range sharding (recommended for single contract)

Assign each worker a disjoint range of ledger sequence numbers. Each worker
maintains its own `Checkpoint` row keyed by a worker ID.

**Schema change required:** the `Checkpoint` model uses a singleton (`id=1`).
For multi-worker deployments, add a `workerId` column:

```sql
-- Extend Checkpoint for per-worker tracking
ALTER TABLE "Checkpoint" ADD COLUMN "workerId" TEXT NOT NULL DEFAULT 'default';
ALTER TABLE "Checkpoint" DROP CONSTRAINT "Checkpoint_pkey";
ALTER TABLE "Checkpoint" ADD PRIMARY KEY ("workerId");
```

Or use the `START_LEDGER` / range-cap environment variables to pin each worker
to a fixed window and let them advance naturally without a shared cursor.

**Environment variables per worker:**

| Variable | Worker A | Worker B |
|---|---|---|
| `CONTRACT_ID` | `C_MAIN` | `C_MAIN` |
| `START_LEDGER` | `0` | `1000000` |
| `END_LEDGER` *(new cap)* | `999999` | *(unlimited — live tail)* |
| `WORKER_ID` | `worker-a` | `worker-b` |
| `DATABASE_URL` | *(shared)* | *(shared)* |

Worker A backfills the historical range and then exits (or idles). Worker B
tails live ledgers. Both write to the same `Attestation` table; `upsert` on
the `id` primary key prevents duplicates.

**Concurrency setting:** within each worker, tune `EVENT_CONCURRENCY` to the
number of CPU cores available:

```bash
EVENT_CONCURRENCY=16  # for an 8-core node with I/O-bound DB writes
```

### Strategy 2: Contract-ID sharding (for multi-contract deployments)

When TrustLink supports multiple deployed contract instances (e.g. one per
chain environment or one per tenant), assign each worker a distinct
`CONTRACT_ID`. Workers are fully independent — no coordination required.

```bash
# Worker A: mainnet contract
CONTRACT_ID=CCMAIN... WORKER_ID=mainnet

# Worker B: testnet contract
CONTRACT_ID=CCTEST... WORKER_ID=testnet
```

Each worker's `Checkpoint` row is keyed by `WORKER_ID`, so their cursors never
collide.

### Strategy 3: Topic-type sharding (advanced, rarely needed)

Route different event topics to different workers. Worker A handles `created`
and `imported`; Worker B handles `revoked`, `ms_*`, and `iss_*`. Each worker
filters the events it cares about, ignoring the rest.

This is only valuable if one event type dominates volume so heavily that it
saturates a single worker's DB write capacity. In practice, `created` events
dominate, so a 1-worker-per-contract design (Strategy 2) combined with
`EVENT_CONCURRENCY` tuning is usually sufficient.

---

## Dead-letter handling across workers

Each worker routes its own failures to the `event_dead_letters` table (see
`indexer/src/indexer.ts` — `routeToDeadLetter`). Because all workers share
the same Postgres database, dead-letter records from all shards are visible in
one place.

**Reprocessing a dead-letter event:**

```bash
# Retrieve pending dead-letter records
psql $DATABASE_URL -c "
  SELECT id, ledger, \"eventType\", \"errorMessage\", \"failedAt\"
  FROM event_dead_letters
  WHERE status = 'PENDING'
  ORDER BY \"failedAt\"
  LIMIT 20;
"

# Mark a record for retry (the indexer's /admin/dead-letter endpoint
# will pick it up on its next poll cycle)
psql $DATABASE_URL -c "
  UPDATE event_dead_letters
  SET status = 'RETRYING', \"updatedAt\" = NOW()
  WHERE id = '<dead-letter-id>';
"
```

In a sharded deployment, the worker that originally failed the event does **not**
need to be the one to retry it — any worker can consume RETRYING records from
the shared table.

---

## Read layer: single consistent GraphQL surface

Workers only write. The GraphQL API is a **read-only** process that connects to
the same Postgres replica:

```bash
# Read-only GraphQL process (no START_LEDGER, no RPC_URL needed)
GRAPHQL_ONLY=true DATABASE_URL=$REPLICA_URL node dist/index.js
```

Add `if (process.env.GRAPHQL_ONLY === 'true') { startGraphQL(); return; }` at
the top of `index.ts` to skip the `startIndexer` call.

For very high read traffic, deploy the read layer behind a load balancer pointed
at a Postgres read replica. PostgreSQL streaming replication introduces < 100 ms
lag in most configurations, which is negligible relative to the 5-second ledger
cadence.

---

## Archival interaction

The archival job (`scheduleArchivalJob` in `archival.ts`) runs as a cron within
a single designated worker. In a sharded deployment, **only one worker should
run the archival job** to avoid double-archiving. Set:

```bash
# On the archival-responsible worker only
ARCHIVAL_ENABLED=true
ARCHIVAL_INTERVAL_HOURS=6

# On all other workers
ARCHIVAL_ENABLED=false
```

Guard the `scheduleArchivalJob` call:

```typescript
if (process.env.ARCHIVAL_ENABLED === 'true') {
  scheduleArchivalJob(db, ARCHIVAL_INTERVAL_HOURS);
}
```

---

## Reconciliation

After a sharded backfill completes, run the reconciliation script to detect
gaps or duplicate IDs:

```bash
# Check for ledger gaps between worker checkpoints
psql $DATABASE_URL -c "
  SELECT workerId, ledger FROM \"Checkpoint\" ORDER BY ledger;
"

# Check for duplicate attestation IDs (should return 0 rows)
psql $DATABASE_URL -c "
  SELECT id, COUNT(*) FROM \"Attestation\" GROUP BY id HAVING COUNT(*) > 1;
"
```

Because `handleEvent` uses `upsert` on the deterministic `id` field,
overlap between worker ranges produces no duplicates — the second write is a
no-op update.

---

## Kubernetes / Helm

The `indexer/helm/` chart supports a `replicaCount` and per-worker environment
variable overrides. For a two-worker ledger-range split:

```yaml
# values-worker-a.yaml
replicaCount: 1
env:
  WORKER_ID: "worker-a"
  START_LEDGER: "0"
  END_LEDGER: "999999"
  EVENT_CONCURRENCY: "16"
  ARCHIVAL_ENABLED: "true"

# values-worker-b.yaml
replicaCount: 1
env:
  WORKER_ID: "worker-b"
  START_LEDGER: "1000000"
  EVENT_CONCURRENCY: "16"
  ARCHIVAL_ENABLED: "false"
```

Deploy with:

```bash
helm upgrade --install trustlink-worker-a ./indexer/helm -f values-worker-a.yaml
helm upgrade --install trustlink-worker-b ./indexer/helm -f values-worker-b.yaml
```

The read-only GraphQL Deployment is a third release with `GRAPHQL_ONLY=true`
and a higher `replicaCount` for horizontal scaling.

---

## Performance tuning reference

| Parameter | Default | Recommendation |
|---|---|---|
| `EVENT_CONCURRENCY` | `8` | Set to 2× DB connection pool size |
| `PAGE_LIMIT` | `200` | Increase to `500` for backfill workers on fast RPC nodes |
| `POLL_MS` | `5000` | Keep at 5000 for live workers (matches ledger cadence) |
| `ARCHIVAL_INTERVAL_HOURS` | `6` | Increase to `24` for non-critical environments |
| Postgres `max_connections` | varies | Set to `(workers × EVENT_CONCURRENCY) + read_replicas × 10` |

---

## See also

- [ADR-011: GraphQL Schema Federation and Versioning](../adr/ADR-011-schema-federation-versioning.md)
- [docs/monitoring.md](./monitoring.md) — alerting and canary deployment
- [indexer/helm/README.md](../indexer/helm/README.md) — Helm chart reference
- [indexer/README.md](../indexer/README.md) — general indexer setup
