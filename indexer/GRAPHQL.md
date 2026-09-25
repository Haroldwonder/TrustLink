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
  revocationReason: String
  metadata: String
  imported: Boolean!
  bridged: Boolean!
  sourceChain: String
  sourceTx: String
  createdAt: String!
  updatedAt: String!
}

type AuditEntry {
  id: Int!
  attestationId: String!
  action: String!
  actor: String!
  details: String
  ledger: Int!
  timestamp: String!
  createdAt: String!
}

type MultisigProposal {
  id: String!
  subject: String!
  proposer: String!
  claimType: String!
  threshold: Int!
  signers: [String!]!
  signatureCount: Int!
  finalized: Boolean!
  expiresAt: String!
  createdAt: String!
  updatedAt: String!
}

type Issuer {
  address: String!
  name: String!
  url: String
  description: String
  tier: String!
  registeredAt: String!
  updatedAt: String!
}

type IssuerList {
  items: [Issuer!]!
  total: Int!
}

type HealthStatus {
  status: String!
  lastLedger: Int
  timestamp: String!
  schemaVersion: String!
}

type AttestationRevoked {
  id: String!
  issuer: String!
  subject: String!
  claimType: String!
  revokedAt: String!
}

type IssuerRegistered {
  issuer: String!
  registeredAt: String!
}

enum RequestStatus {
  PENDING
  FULFILLED
  REJECTED
}

type AttestationRequest {
  id: String!
  subject: String!
  issuer: String!
  claimType: String!
  requestedAt: String!
  expiresAt: String!
  status: RequestStatus!
  fulfillmentId: String
  rejectionReason: String
  createdAt: String!
  updatedAt: String!
}

type Endorsement {
  id: String!
  attestationId: String!
  endorser: String!
  timestamp: String!
  createdAt: String!
}

type Template {
  id: String!
  templateId: String!
  issuer: String!
  claimType: String!
  createdAt: String!
}

type Delegation {
  id: String!
  delegator: String!
  delegate: String!
  claimType: String!
  expiresAt: String!
  revoked: Boolean!
  createdAt: String!
}

type WhitelistEntry {
  id: String!
  issuer: String!
  subject: String!
  createdAt: String!
}

type CouncilAction {
  id: String!
  actionId: String!
  proposer: String!
  approvals: [String!]!
  executed: Boolean!
  createdAt: String!
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
  rateLimit: Int
}
```

---

## Queries

### `healthCheck`

Synthetic liveness probe — returns `status: "ok"` when the indexer is healthy.

```graphql
query {
  healthCheck {
    status
    lastLedger
    timestamp
    schemaVersion
  }
}
```

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
    rateLimit
  }
}
```

### `issuer`

Get a single issuer by address. Returns `null` when the address is not registered.

```graphql
query {
  issuer(address: "G...") {
    address
    name
    url
    description
    tier
    registeredAt
    updatedAt
  }
}
```

**Parameters:**
- `address` (required): Issuer Stellar address

### `issuers`

List all issuers with offset-based pagination.

```graphql
query {
  issuers(start: 0, limit: 50) {
    items {
      address
      name
      tier
      registeredAt
    }
    total
  }
}
```

**Parameters:**
- `start` (optional): Pagination offset
- `limit` (optional): Pagination limit

### `proposal`

Get a single multi-sig proposal by ID. Returns `null` when not found.

```graphql
query {
  proposal(id: "prop_abc123") {
    id
    subject
    proposer
    claimType
    threshold
    signers
    signatureCount
    finalized
    expiresAt
    createdAt
    updatedAt
  }
}
```

**Parameters:**
- `id` (required): Proposal ID

### `proposals`

List multi-sig proposals with optional filters.

```graphql
query {
  proposals(subject: "G...", finalized: false) {
    id
    subject
    proposer
    claimType
    threshold
    signatureCount
    finalized
  }
}
```

**Parameters:**
- `subject` (optional): Filter by subject address
- `finalized` (optional): Filter by finalization status

### `auditLog`

Get audit log entries for an attestation (#774).

```graphql
query {
  auditLog(attestationId: "att_abc123") {
    id
    attestationId
    action
    actor
    details
    ledger
    timestamp
    createdAt
  }
}
```

**Parameters:**
- `attestationId` (required): Attestation ID

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

Real-time stream of newly created attestations. All filter arguments are optional and applied with AND logic when combined.

```graphql
subscription {
  onAttestationCreated(subject: "G...", issuer: "G...", claimType: "KYC_PASSED", topics: ["created"]) {
    id
    issuer
    subject
    claimType
    timestamp
  }
}
```

**Parameters:**
- `subject` (optional): Only emit events for this subject address
- `issuer` (optional): Only emit events issued by this issuer address
- `claimType` (optional): Only emit events for this claim type (e.g. `"KYC_PASSED"`)
- `topics` (optional): Only emit events if `'created'` is included in this topic allowlist

### `onAttestationRevoked`

Real-time stream of attestation revocation events. All filter arguments are optional and applied with AND logic when combined.

```graphql
subscription {
  onAttestationRevoked(subject: "G...", issuer: "G...", claimType: "KYC_PASSED", topics: ["revoked"]) {
    id
    issuer
    subject
    claimType
    revokedAt
  }
}
```

**Parameters:**
- `subject` (optional): Only emit events for this subject address
- `issuer` (optional): Only emit events issued by this issuer address
- `claimType` (optional): Only emit events for this claim type (e.g. `"KYC_PASSED"`)
- `topics` (optional): Only emit events if `'revoked'` is included in this topic allowlist

### `onIssuerRegistered`

Real-time stream of issuer registration events.

```graphql
subscription {
  onIssuerRegistered {
    issuer
    registeredAt
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
