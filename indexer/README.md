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
