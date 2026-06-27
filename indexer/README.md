# TrustLink Event Indexer

Off-chain indexer that listens to TrustLink contract events on Stellar, persists them to PostgreSQL, and exposes a REST API.

## Architecture

```
Stellar RPC  →  indexer.ts (poll getEvents)  →  PostgreSQL (Prisma)
                                                      ↑
                                              Fastify REST API
```

- **Backfill**: on startup the indexer reads the last processed ledger from the `Checkpoint` table and replays any missed events up to the current tip.
- **Live polling**: after backfill, the indexer polls `getEvents` every 5 seconds.
- **Persistence**: `Attestation` rows are upserted so re-processing is idempotent.

## Environment Variables

Copy `.env.example` to `.env` and fill in the values:

| Variable         | Description                                            | Default                               |
| ---------------- | ------------------------------------------------------ | ------------------------------------- |
| `DATABASE_URL`   | PostgreSQL connection string                           | —                                     |
| `CONTRACT_ID`    | Deployed TrustLink contract ID                         | —                                     |
| `RPC_URL`        | Soroban RPC endpoint                                   | `https://soroban-testnet.stellar.org` |
| `GENESIS_LEDGER` | First ledger to index (contract deployment ledger)     | `0`                                   |
| `START_LEDGER`   | Override starting ledger (overrides stored checkpoint) | —                                     |
| `PORT`           | HTTP port for the REST API                             | `3000`                                |

## Quick Start (Docker)

```bash
cp .env.example .env
# Edit .env — set CONTRACT_ID and GENESIS_LEDGER at minimum

docker compose up --build
```

The API will be available at `http://localhost:3000`.

## Quick Start (local dev)

```bash
cp .env.example .env   # fill in values
npm install
npx prisma migrate deploy
npm run dev
```

## REST API

### `GET /attestations/:subject`

Returns all attestations for a subject address.

```bash
curl http://localhost:3000/attestations/GABC...XYZ
```

### `GET /attestations/issuer/:issuer`

Returns all attestations issued by a specific issuer.

```bash
curl http://localhost:3000/attestations/issuer/GDEF...UVW
```

Both endpoints return an array of `Attestation` objects ordered by `timestamp` descending.

### `GET /health`

Returns the health status of the indexer including database connectivity.

```bash
curl http://localhost:3000/health
```

Response (200 OK):

```json
{
  "status": "ok",
  "db": "connected",
  "lastLedger": 12345
}
```

Response (503 if database unreachable):

```json
{
  "status": "error",
  "db": "disconnected",
  "lastLedger": 12345
}
```

### `POST /admin/reindex?from=LEDGER`

Triggers a historical backfill from a specific ledger. If `from` is not provided, starts from the last checkpoint.

```bash
# Reindex from a specific ledger
curl -X POST "http://localhost:3000/admin/reindex?from=10000"

# Reindex from last checkpoint
curl -X POST "http://localhost:3000/admin/reindex"
```

This is useful for reprocessing events after a crash or for catching up missed events.

## Event Archival & Cold Storage

To prevent unbounded database growth over mainnet deployment, the indexer implements a configurable archival strategy that moves old processed events to cold storage (object storage or local filesystem).

### Archival Architecture

```
RawEvent table (hot) ──[older than N days]──> Archive files (cold)
└─ 30-day retention              │
                                 ├─> ArchivedEventBatch (metadata)
                                 └─> Compressed JSONL files
```

**Flow:**

1. **Event Ingestion** — Real-time events stored in `RawEvent` table
2. **Event Processing** — Derived tables (`Attestation`, `Issuer`, etc.) updated from raw events
3. **Archival Job** — Every 6 hours, events older than retention window (default: 30 days) are:
   - Grouped by ledger range
   - Exported to compressed JSONL files
   - Checksum verified (SHA-256)
   - Metadata recorded in `ArchivedEventBatch` table
   - Deleted from `RawEvent` (optional: dry-run mode)
4. **Restore** — Events can be recovered from archive for auditing or recovery

### Configuration

#### Environment Variables

| Variable                  | Type   | Default                         | Description                                             |
| ------------------------- | ------ | ------------------------------- | ------------------------------------------------------- |
| `ARCHIVAL_INTERVAL_HOURS` | int    | `6`                             | How often to run archival job (hours)                   |
| `ARCHIVE_PATH`            | string | `s3://trustlink-events-archive` | Destination for archived files (S3, GCS, or local path) |

#### Database Configuration

The `ArchivalConfig` table (singleton, id=1) controls archival behavior:

```sql
-- Read current config
SELECT * FROM "ArchivalConfig" WHERE id = 1;

-- Enable archival (one-time)
UPDATE "ArchivalConfig"
SET "archiveEnabled" = true
WHERE id = 1;

-- Adjust retention window to 7 days (aggressive) or 60 days (conservative)
UPDATE "ArchivalConfig"
SET "retentionDaysRaw" = 14
WHERE id = 1;

-- Disable compression (if CPU-bound)
UPDATE "ArchivalConfig"
SET "compressionEnabled" = false
WHERE id = 1;
```

### Storage Locations

#### Local Filesystem

Archive files are organized by date:

```
/archive/trustlink-events/
├── 2026/
│   ├── 01/
│   │   ├── 15/ledger_1000000-1009999.json.gz
│   │   ├── 16/ledger_1010000-1019999.json.gz
│   │   └── ...
│   └── ...
```

**Setup:**

```bash
# Create archive directory
mkdir -p /archive/trustlink-events

# Update config
UPDATE "ArchivalConfig"
SET "archivePath" = '/archive/trustlink-events'
WHERE id = 1;
```

#### AWS S3

Archive files are stored in S3 with automatic lifecycle policies:

```
s3://trustlink-events-archive/
├── 2026/
│   ├── 01/
│   │   ├── 15/ledger_1000000-1009999.json.gz
│   │   └── ...
```

**Setup:**

```bash
# Create S3 bucket
aws s3api create-bucket \
  --bucket trustlink-events-archive \
  --region us-east-1

# Enable versioning and lifecycle policies
aws s3api put-bucket-versioning \
  --bucket trustlink-events-archive \
  --versioning-configuration Status=Enabled

# Transition old objects to Glacier after 90 days
aws s3api put-bucket-lifecycle-configuration \
  --bucket trustlink-events-archive \
  --lifecycle-configuration '{
    "Rules": [{
      "Id": "archive-to-glacier",
      "Status": "Enabled",
      "Transitions": [{
        "Days": 90,
        "StorageClass": "GLACIER"
      }],
      "Expiration": {"Days": 2555}
    }]
  }'

# Update config
UPDATE "ArchivalConfig"
SET "archivePath" = 's3://trustlink-events-archive'
WHERE id = 1;
```

Requires AWS credentials in environment:

- `AWS_ACCESS_KEY_ID`
- `AWS_SECRET_ACCESS_KEY`
- `AWS_REGION` (default: us-east-1)

#### Google Cloud Storage

Archive files stored in GCS with custom lifecycle:

```
gs://trustlink-events-archive/
├── 2026/
│   ├── 01/
│   │   ├── 15/ledger_1000000-1009999.json.gz
```

**Setup:**

```bash
# Create GCS bucket
gsutil mb -p PROJECT_ID -l us-central1 gs://trustlink-events-archive

# Set lifecycle policy
gsutil lifecycle set - gs://trustlink-events-archive <<EOF
{
  "lifecycle": {
    "rule": [{
      "action": {"type": "SetStorageClass", "storageClass": "NEARLINE"},
      "condition": {"age": 30}
    },
    {
      "action": {"type": "SetStorageClass", "storageClass": "COLDLINE"},
      "condition": {"age": 90}
    }]
  }
}
EOF

# Update config
UPDATE "ArchivalConfig"
SET "archivePath" = 'gs://trustlink-events-archive'
WHERE id = 1;
```

Requires GCP credentials:

- `GOOGLE_APPLICATION_CREDENTIALS` (path to service account JSON)

### Operational Commands

#### Check Archival Status

```bash
# Last run
SELECT
  "lastArchivedAt",
  "lastArchivedAttemptAt",
  "lastArchivalError"
FROM "ArchivalConfig"
WHERE id = 1;

-- View archived batches
SELECT
  "fromLedger",
  "toLedger",
  "recordCount",
  "compressedSize",
  "uncompressedSize",
  ROUND(100.0 * "compressedSize" / "uncompressedSize", 1) as compression_ratio,
  "archivedAt"
FROM "ArchivedEventBatch"
ORDER BY "archivedAt" DESC
LIMIT 10;
```

#### Manual Archival

```bash
# Trigger one-time archival (via API)
curl -X POST http://localhost:3000/admin/archive

# Dry-run mode (don't delete)
curl -X POST "http://localhost:3000/admin/archive?dryRun=true"

# Custom retention window (archive events > 7 days old)
curl -X POST "http://localhost:3000/admin/archive?maxAgeDays=7"
```

#### Restore from Archive

For auditing or recovery after accidental deletion:

```bash
# Restore events from ledger range
curl -X POST "http://localhost:3000/admin/restore?fromLedger=1000000&toLedger=1010000"

# Restored events are re-inserted into RawEvent (idempotent)
# Does NOT re-process derived tables — attestations remain unchanged
```

### Storage Cost Estimates

**Raw events (before archival):**

- Average event: ~500 bytes
- 10,000 events: ~5 MB
- 365 days (30-day retention): ~150 GB hot storage

**Archived events (after compression):**

- Compression ratio: ~10:1 (typical with gzip)
- 365 days (cold storage): ~15 GB

**S3 Cost (us-east-1):**
| Tier | Cost per GB/month |
|---|---|
| Standard | $0.023 |
| Intelligent-Tiering | $0.0125 |
| Glacier | $0.004 |

**365 days of events:**

- Hot (30 days, Standard): $3.45/month
- Cold (335 days, Glacier): $0.18/month
- **Total: ~$3.63/month** for event archival

### Rollback Procedures

#### Disable Archival (Emergency)

If archival is consuming too much compute or storage:

```sql
-- Pause archival job
UPDATE "ArchivalConfig"
SET "archiveEnabled" = false
WHERE id = 1;

-- Events continue to accumulate in RawEvent
-- Run manual cleanup later when ready
```

#### Restore All Deleted Events

If events were accidentally deleted and need recovery:

```bash
# List all archived batches
SELECT "fromLedger", "toLedger", "archivedAt" FROM "ArchivedEventBatch";

# Restore a range (e.g., ledgers 1M–2M)
curl -X POST "http://localhost:3000/admin/restore?fromLedger=1000000&toLedger=2000000"

# Verify restored count
SELECT COUNT(*) FROM "RawEvent" WHERE ledger BETWEEN 1000000 AND 2000000;
```

#### Rollback to Full Event History

If you need to disable archival and keep all events forever:

```sql
-- Stop archival
UPDATE "ArchivalConfig"
SET "archiveEnabled" = false
WHERE id = 1;

-- Restore archived events (run for all batches)
SELECT DISTINCT "fromLedger", "toLedger" FROM "ArchivedEventBatch"
ORDER BY "fromLedger";

-- For each batch range:
-- curl -X POST "http://localhost:3000/admin/restore?fromLedger=X&toLedger=Y"

-- Once restored, RawEvent will contain all history
-- Continue operating normally (no archival)
```

### Database Maintenance

#### TTL Policies

Recommend setting Prisma TTL to avoid expired entries:

```prisma
model RawEvent {
  // ... fields ...
  createdAt DateTime @default(now())

  // Soft TTL: archival deletes after N days
  // Hard TTL: database auto-evicts after 2x retention window
  // (Can be configured in PostgreSQL or Prisma)
}
```

#### Backup Strategy

Before running large archival jobs:

```bash
# Full database backup
pg_dump $DATABASE_URL | gzip > backup_$(date +%Y%m%d).sql.gz

# Archive to S3
aws s3 cp backup_$(date +%Y%m%d).sql.gz s3://trustlink-backups/

# Keep backups for 90 days
```

#### Monitoring

Add alerting for archival failures:

```sql
-- Alert if archival hasn't run in 12 hours
SELECT
  EXTRACT(EPOCH FROM (NOW() - "lastArchivedAttemptAt")) / 3600 as hours_since_last_attempt,
  "lastArchivalError"
FROM "ArchivalConfig"
WHERE id = 1
  AND "lastArchivedAttemptAt" < NOW() - INTERVAL '12 hours';
```

### Performance Impact

**Before Archival (unbounded RawEvent table):**

- Query latency: 50–100 ms (full table scan)
- Storage: grows ~5 MB/day
- Backup time: O(history size)

**After Archival (30-day retention):**

- Query latency: <5 ms (small table)
- Storage: steady ~150 GB
- Backup time: 5–10 minutes

### Troubleshooting

#### Archival Job Fails

```bash
# Check error message
SELECT "lastArchivalError" FROM "ArchivalConfig" WHERE id = 1;

# Common issues:
# 1. Out of disk space: free up /tmp
# 2. Permission denied: check archive path ownership
# 3. S3 credentials invalid: verify AWS env vars
# 4. Network timeout: check internet connectivity
```

#### Archive Files Corrupted

```bash
# Verify checksum
SELECT
  "id",
  "archivePath",
  "checksum"
FROM "ArchivedEventBatch"
WHERE "archivedAt" > NOW() - INTERVAL '24 hours';

# If mismatch detected:
# 1. Re-archive the batch (archival service detects and repairs)
# 2. Or restore from S3 versioning / GCS version history
```

#### Storage Growing Too Fast

```sql
-- Reduce retention window (from 30 to 7 days)
UPDATE "ArchivalConfig"
SET "retentionDaysRaw" = 7
WHERE id = 1;

-- Or increase archival frequency (from 6h to 2h)
-- Set ARCHIVAL_INTERVAL_HOURS=2 in .env and restart
```

---

## Webhooks

| Column        | Type      | Description                        |
| ------------- | --------- | ---------------------------------- |
| `id`          | `text` PK | Deterministic contract hash ID     |
| `issuer`      | `text`    | Issuer address                     |
| `subject`     | `text`    | Subject address                    |
| `claimType`   | `text`    | e.g. `KYC_PASSED`                  |
| `timestamp`   | `bigint`  | Ledger timestamp at creation       |
| `expiration`  | `bigint?` | Optional expiry timestamp          |
| `isRevoked`   | `bool`    | Set to `true` on `revoked` event   |
| `metadata`    | `text?`   | Issuer-supplied metadata           |
| `imported`    | `bool`    | `true` for imported attestations   |
| `bridged`     | `bool`    | `true` for bridged attestations    |
| `sourceChain` | `text?`   | Origin chain (bridged only)        |
| `sourceTx`    | `text?`   | Origin tx reference (bridged only) |

## Webhooks

The indexer can deliver real-time event notifications to registered HTTP endpoints.

### Signature Verification

Every outbound webhook request is signed with **HMAC-SHA256** using the webhook's secret key. The signature is sent in the `X-TrustLink-Signature` HTTP header as a lowercase hex string.

**Signature algorithm:**

```
X-TrustLink-Signature = HMAC-SHA256(secret, body)
```

Where `body` is the raw JSON request body (UTF-8 encoded) and `secret` is the per-webhook secret configured in the database.

**Request body shape:**

```json
{
  "event": "<event_type>",
  "data": { ... },
  "ts": 1700000000000
}
```

| Field   | Type   | Description                                                  |
| ------- | ------ | ------------------------------------------------------------ |
| `event` | string | Event type, e.g. `attestation_created`                       |
| `data`  | object | Event-specific payload                                       |
| `ts`    | number | Unix timestamp in milliseconds when the event was dispatched |

**Verifying the signature in your receiver (Node.js example):**

```ts
import { createHmac, timingSafeEqual } from "crypto";

function verifyWebhook(
  secret: string,
  rawBody: string,
  signature: string,
): boolean {
  const expected = createHmac("sha256", secret).update(rawBody).digest("hex");
  const a = Buffer.from(expected, "hex");
  const b = Buffer.from(signature, "hex");
  return a.length === b.length && timingSafeEqual(a, b);
}
```

Always use a constant-time comparison (e.g. `timingSafeEqual`) to prevent timing-based attacks.

### Retry Policy

Failed deliveries are retried up to **5 times** with exponential backoff (200 ms base, capped at 10 s). HTTP `4xx` responses are not retried (they indicate a client-side misconfiguration).

### Running the Tests

```bash
cd indexer
npm install
npm test
```
