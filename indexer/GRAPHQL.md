# TrustLink GraphQL API

The indexer exposes a GraphQL endpoint alongside the existing REST API.

## Endpoints

| Endpoint | Protocol | Description |
|---|---|---|
| `http://localhost:4000/graphql` | HTTP | Queries & Mutations |
| `ws://localhost:4000/graphql` | WebSocket | Subscriptions |

The Apollo Sandbox (interactive playground) is available at `http://localhost:4000/graphql` in development.

Set `GQL_PORT` env var to change the port (default: `4000`).

---

## Schema

### Enums

```graphql
enum Status {
  ACTIVE
  REVOKED
}
```

### Types

```graphql
type Attestation {
  id: String!
  issuer: String!
  subject: String!
  claimType: String!
  timestamp: String!       # BigInt serialized as string
  expiration: String       # BigInt serialized as string, nullable
  isRevoked: Boolean!
  metadata: String
  imported: Boolean!
  bridged: Boolean!
  sourceChain: String
  sourceTx: String
  createdAt: String!
  updatedAt: String!
}

type IssuerStats {
  issuer: String!
  total: Int!
  active: Int!
  revoked: Int!
  claimTypes: [String!]!
}
```

---

## Queries

### `attestations`

Fetch attestations with optional filters.

```graphql
query {
  attestations(subject: "G...", claimType: "KYC", status: ACTIVE) {
    id
    issuer
    subject
    claimType
    timestamp
    isRevoked
  }
}
```

All arguments are optional — omitting them returns all attestations.

### `issuerStats`

Aggregate stats for a given issuer address.

```graphql
query {
  issuerStats(issuer: "G...") {
    issuer
    total
    active
    revoked
    claimTypes
  }
}
```

---

## Subscriptions

### `onAttestationCreated`

Real-time stream of newly created attestations. Optionally filter by subject.

```graphql
subscription {
  onAttestationCreated(subject: "G...") {
    id
    issuer
    subject
    claimType
    timestamp
  }
}
```

Connect via WebSocket to `ws://localhost:4000/graphql` using the `graphql-ws` protocol.

---

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `GQL_PORT` | `4000` | GraphQL server port |
| `PORT` | `3000` | REST (Fastify) server port |
