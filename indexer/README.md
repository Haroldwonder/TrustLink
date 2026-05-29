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

| Variable | Description | Default |
|---|---|---|
| `DATABASE_URL` | PostgreSQL connection string | — |
| `CONTRACT_ID` | Deployed TrustLink contract ID | — |
| `RPC_URL` | Soroban RPC endpoint | `https://soroban-testnet.stellar.org` |
| `GENESIS_LEDGER` | First ledger to index (contract deployment ledger) | `0` |
| `PORT` | HTTP port for the REST API | `3000` |

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

## Database Schema

| Column | Type | Description |
|---|---|---|
| `id` | `text` PK | Deterministic contract hash ID |
| `issuer` | `text` | Issuer address |
| `subject` | `text` | Subject address |
| `claimType` | `text` | e.g. `KYC_PASSED` |
| `timestamp` | `bigint` | Ledger timestamp at creation |
| `expiration` | `bigint?` | Optional expiry timestamp |
| `isRevoked` | `bool` | Set to `true` on `revoked` event |
| `metadata` | `text?` | Issuer-supplied metadata |
| `imported` | `bool` | `true` for imported attestations |
| `bridged` | `bool` | `true` for bridged attestations |
| `sourceChain` | `text?` | Origin chain (bridged only) |
| `sourceTx` | `text?` | Origin tx reference (bridged only) |

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

| Field   | Type   | Description                                      |
|---------|--------|--------------------------------------------------|
| `event` | string | Event type, e.g. `attestation_created`           |
| `data`  | object | Event-specific payload                           |
| `ts`    | number | Unix timestamp in milliseconds when the event was dispatched |

**Verifying the signature in your receiver (Node.js example):**

```ts
import { createHmac, timingSafeEqual } from "crypto";

function verifyWebhook(secret: string, rawBody: string, signature: string): boolean {
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
