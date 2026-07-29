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

type PageInfo {
  hasNextPage: Boolean!
  hasPreviousPage: Boolean!
  startCursor: String
  endCursor: String
}

type AttestationEdge {
  node: Attestation!
  cursor: String!
}

type AttestationConnection {
  edges: [AttestationEdge!]!
  pageInfo: PageInfo!
  totalCount: Int!
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

Fetch attestations with optional filters and cursor-based pagination.

```graphql
query {
  attestations(
    subject: "G...", 
    claimType: "KYC", 
    status: ACTIVE,
    first: 10,
    after: "eyJpZCI6ImF0dF8xMjM0NTY3ODkifQ=="
  ) {
    edges {
      node {
        id
        issuer
        subject
        claimType
        timestamp
        isRevoked
      }
      cursor
    }
    pageInfo {
      hasNextPage
      hasPreviousPage
      startCursor
      endCursor
    }
    totalCount
  }
}
```

**Parameters:**
- `subject` (optional): Filter by subject address
- `claimType` (optional): Filter by claim type
- `status` (optional): Filter by ACTIVE or REVOKED status
- `first` (optional): Number of results to return (default: 50, max: 100)
- `after` (optional): Cursor for pagination

### `attestationsByIssuer`

Fetch attestations by issuer with cursor-based pagination.

```graphql
query {
  attestationsByIssuer(
    issuer: "G...",
    first: 20,
    after: "eyJpZCI6ImF0dF85ODc2NTQzMjEifQ=="
  ) {
    edges {
      node {
        id
        subject
        claimType
        timestamp
        isRevoked
      }
      cursor
    }
    pageInfo {
      hasNextPage
      hasPreviousPage
      startCursor
      endCursor
    }
    totalCount
  }
}
```

**Parameters:**
- `issuer` (required): Issuer address to filter by
- `first` (optional): Number of results to return (default: 50, max: 100)
- `after` (optional): Cursor for pagination

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

## Pagination Examples

### Basic Pagination

```graphql
# First page
query {
  attestations(first: 10) {
    edges {
      node { id subject claimType }
      cursor
    }
    pageInfo {
      hasNextPage
      endCursor
    }
  }
}

# Next page using endCursor from previous response
query {
  attestations(first: 10, after: "eyJpZCI6ImF0dF8xMjM0NTY3ODkifQ==") {
    edges {
      node { id subject claimType }
      cursor
    }
    pageInfo {
      hasNextPage
      endCursor
    }
  }
}
```

### Paginated Filtering

```graphql
# Get active KYC attestations for a subject with pagination
query {
  attestations(
    subject: "GDXLKEY5TR4IDEVSTRYUNYY3DPXQKQNSTDJ7HIVNFTJYQHOZXB7CRQME",
    claimType: "KYC",
    status: ACTIVE,
    first: 25
  ) {
    edges {
      node {
        id
        timestamp
        expiration
        metadata
      }
    }
    pageInfo {
      hasNextPage
      totalCount
    }
  }
}
```

### Issuer-Specific Pagination

```graphql
# Get all attestations issued by a specific issuer
query {
  attestationsByIssuer(
    issuer: "GCKFBEIYTKP6RCZX6LRQW2JVDVKV6WATK4BKDNFPVAH6TWMA6N2JQHSR",
    first: 50
  ) {
    edges {
      node {
        id
        subject
        claimType
        timestamp
        isRevoked
      }
    }
    pageInfo {
      hasNextPage
      endCursor
    }
    totalCount
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
