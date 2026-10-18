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

#### `Status`

Lifecycle state of an attestation.

```graphql
enum Status {
  ACTIVE
  REVOKED
}
```

#### `RequestStatus`

Lifecycle state of an attestation request.

```graphql
enum RequestStatus {
  PENDING
  FULFILLED
  REJECTED
}
```

### Types

#### `Attestation`

Core attestation entity. Ledger values (`timestamp`, `expiration`) and the
`createdAt`/`updatedAt` bookkeeping fields are BigInt values serialized as strings.

```graphql
type Attestation {
  id: String!
  issuer: String!
  subject: String!
  claimType: String!
  timestamp: String!
  expiration: String
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
```

`revocationReason` is populated when the attestation was revoked with a reason
(`null` otherwise). `imported`/`bridged`/`sourceChain`/`sourceTx` mark
attestations that originated outside this chain.

#### `AttestationConnection`, `AttestationEdge`, `PageInfo`

Cursor-based pagination wrappers returned by `attestations` and
`attestationsByIssuer`.

```graphql
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
```

#### `Issuer`

An issuer entity, keyed by its unique Stellar address
(federation key candidate: `address`).

```graphql
type Issuer {
  address: String!
  name: String!
  url: String
  description: String
  tier: String!
  registeredAt: String!
  updatedAt: String!
}
```

#### `IssuerList`

Paginated issuer list returned by the `issuers` query.

```graphql
type IssuerList {
  items: [Issuer!]!
  total: Int!
}
```

#### `IssuerStats`

Aggregate statistics for one issuer.

```graphql
type IssuerStats {
  issuer: String!
  total: Int!
  active: Int!
  revoked: Int!
  claimTypes: [String!]!
  # #775: rate limit from set_rate_limit events
  rateLimit: Int
}
```

`rateLimit` is the issuer's configured rate limit from `set_rate_limit` events
(`null` when the issuer has never set one).

#### `HealthStatus`

Result of the `healthCheck` liveness probe.

```graphql
type HealthStatus {
  """ok or degraded"""
  status: String!
  """Last ledger sequence number processed by the indexer"""
  lastLedger: Int
  """ISO-8601 timestamp of this response"""
  timestamp: String!
  """
  Semantic version of the GraphQL schema (e.g. "1.0.0").
  Clients can use this to detect schema-level breaking changes independently
  of the process version. See ADR-011.
  """
  schemaVersion: String!
}
```

#### `MultisigProposal`

A multi-signature attestation proposal awaiting signatures/execution.

```graphql
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
```

#### `AuditEntry`

Audit log entry for an attestation (`#774`).

```graphql
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
```

#### `AttestationRequest`

An attestation requested by a subject from an issuer, with its fulfilment
status.

```graphql
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
```

`fulfillmentId` is set when the request is `FULFILLED`; `rejectionReason` when
it is `REJECTED`.

#### `Endorsement`

An endorsement of an existing attestation by a third party.

```graphql
type Endorsement {
  id: String!
  attestationId: String!
  endorser: String!
  timestamp: String!
  createdAt: String!
}
```

#### `Template`

A claim-type template published by an issuer.

```graphql
type Template {
  id: String!
  templateId: String!
  issuer: String!
  claimType: String!
  createdAt: String!
}
```

#### `Delegation`

A delegation of attestation-issuing rights from one address to another.

```graphql
type Delegation {
  id: String!
  delegator: String!
  delegate: String!
  claimType: String!
  expiresAt: String!
  revoked: Boolean!
  createdAt: String!
}
```

#### `WhitelistEntry`

A whitelisted issuer → subject pair.

```graphql
type WhitelistEntry {
  id: String!
  issuer: String!
  subject: String!
  createdAt: String!
}
```

#### `CouncilAction`

A governance council action and its approvals.

```graphql
type CouncilAction {
  id: String!
  actionId: String!
  proposer: String!
  approvals: [String!]!
  executed: Boolean!
  createdAt: String!
}
```

#### `AttestationRevoked`, `IssuerRegistered`

Subscription payload types (see [Subscriptions](#subscriptions)).

```graphql
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
```

---

## Queries

### `healthCheck`

Synthetic liveness probe — returns status `ok` when the indexer is healthy
(`degraded` otherwise).

Signature: `healthCheck: HealthStatus!`

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

Signature: `attestations(subject: String, claimType: String, status: Status, first: Int, after: String): AttestationConnection!`

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

Signature: `attestationsByIssuer(issuer: String!, first: Int, after: String): AttestationConnection!`

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

Signature: `issuerStats(issuer: String!): IssuerStats!`

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

**Parameters:**
- `issuer` (required): Issuer address to aggregate over

### `issuer`

Get a single issuer by address. Returns `null` when no issuer is registered for
that address.

Signature: `issuer(address: String!): Issuer`

```graphql
query {
  issuer(address: "GCKFBEIYTKP6RCZX6LRQW2JVDVKV6WATK4BKDNFPVAH6TWMA6N2JQHSR") {
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

List registered issuers with offset-based pagination.

Signature: `issuers(start: Int, limit: Int): IssuerList!`

```graphql
query {
  issuers(start: 0, limit: 20) {
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
- `start` (optional): Offset to start from
- `limit` (optional): Maximum number of issuers to return

### `proposal`

Get a multi-sig proposal by ID. Returns `null` when the proposal does not exist.

Signature: `proposal(id: String!): MultisigProposal`

```graphql
query {
  proposal(id: "42") {
    id
    subject
    proposer
    claimType
    threshold
    signers
    signatureCount
    finalized
    expiresAt
  }
}
```

**Parameters:**
- `id` (required): Multi-sig proposal ID

### `proposals`

List all multi-sig proposals, optionally filtered.

Signature: `proposals(subject: String, finalized: Boolean): [MultisigProposal!]!`

```graphql
query {
  proposals(subject: "G...", finalized: false) {
    id
    proposer
    claimType
    threshold
    signatureCount
    finalized
    expiresAt
  }
}
```

**Parameters:**
- `subject` (optional): Filter by subject address
- `finalized` (optional): Filter by finalization state

### `auditLog`

Get audit log entries for an attestation (`#774`).

Signature: `auditLog(attestationId: String!): [AuditEntry!]!`

```graphql
query {
  auditLog(attestationId: "a1b2c3...") {
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
- `attestationId` (required): Attestation ID to fetch the audit trail for

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

All filter arguments are optional and applied with AND logic when combined.

### `onAttestationCreated`

Real-time stream of newly created attestations.

Signature: `onAttestationCreated(subject: String, issuer: String, claimType: String, topics: [String!]): Attestation!`

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

**Parameters:**
- `subject` (optional): Only emit events for this subject address
- `issuer` (optional): Only emit events issued by this issuer address
- `claimType` (optional): Only emit events for this claim type (e.g. `"KYC_PASSED"`)
- `topics` (optional): Only emit events if `created` is included in this topic allowlist

### `onAttestationRevoked`

Real-time stream of attestation revocation events.

Signature: `onAttestationRevoked(subject: String, issuer: String, claimType: String, topics: [String!]): AttestationRevoked!`

```graphql
subscription {
  onAttestationRevoked(claimType: "KYC_PASSED") {
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
- `topics` (optional): Only emit events if `revoked` is included in this topic allowlist

### `onIssuerRegistered`

Real-time stream of issuer registration events. Takes no arguments.

Signature: `onIssuerRegistered: IssuerRegistered!`

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

---

## Coverage Map

Every field of the SDL's `Query` and `Subscription` root types, and every named
type/enum defined in [`src/schema.graphql`](src/schema.graphql), mapped to the
section of this document that describes it. **Counts: 9/9 queries, 3/3
subscriptions, 22/22 named types & enums.**

### Queries (`type Query`)

| # | Query field | Documented in |
|---|---|---|
| 1 | `healthCheck` | [Queries → `healthCheck`](#healthcheck) |
| 2 | `attestations` | [Queries → `attestations`](#attestations) |
| 3 | `attestationsByIssuer` | [Queries → `attestationsByIssuer`](#attestationsbyissuer) |
| 4 | `issuer(address)` | [Queries → `issuer`](#issuer) |
| 5 | `issuers(start, limit)` | [Queries → `issuers`](#issuers) |
| 6 | `issuerStats(issuer)` | [Queries → `issuerStats`](#issuerstats) |
| 7 | `proposal(id)` | [Queries → `proposal`](#proposal) |
| 8 | `proposals(subject, finalized)` | [Queries → `proposals`](#proposals) |
| 9 | `auditLog(attestationId)` | [Queries → `auditLog`](#auditlog) |

### Subscriptions (`type Subscription`)

| # | Subscription field | Documented in |
|---|---|---|
| 1 | `onAttestationCreated` | [Subscriptions → `onAttestationCreated`](#onattestationcreated) |
| 2 | `onAttestationRevoked` | [Subscriptions → `onAttestationRevoked`](#onattestationrevoked) |
| 3 | `onIssuerRegistered` | [Subscriptions → `onIssuerRegistered`](#onissuerregistered) |

### Types & enums

| # | Type / enum | Kind | Documented in |
|---|---|---|---|
| 1 | `Status` | enum | [Schema → Enums → `Status`](#status) |
| 2 | `RequestStatus` | enum | [Schema → Enums → `RequestStatus`](#requeststatus) |
| 3 | `Query` | root type | [Queries](#queries) (table above) |
| 4 | `Subscription` | root type | [Subscriptions](#subscriptions) (table above) |
| 5 | `Attestation` | object | [Schema → Types → `Attestation`](#attestation) |
| 6 | `AttestationConnection` | object | [Schema → Types → `AttestationConnection`, `AttestationEdge`, `PageInfo`](#attestationconnection-attestationedge-pageinfo) |
| 7 | `AttestationEdge` | object | [Schema → Types → `AttestationConnection`, `AttestationEdge`, `PageInfo`](#attestationconnection-attestationedge-pageinfo) |
| 8 | `PageInfo` | object | [Schema → Types → `AttestationConnection`, `AttestationEdge`, `PageInfo`](#attestationconnection-attestationedge-pageinfo) |
| 9 | `Issuer` | object | [Schema → Types → `Issuer`](#issuer-1) |
| 10 | `IssuerList` | object | [Schema → Types → `IssuerList`](#issuerlist) |
| 11 | `IssuerStats` | object | [Schema → Types → `IssuerStats`](#issuerstats-1) |
| 12 | `HealthStatus` | object | [Schema → Types → `HealthStatus`](#healthstatus) |
| 13 | `MultisigProposal` | object | [Schema → Types → `MultisigProposal`](#multisigproposal) |
| 14 | `AuditEntry` | object | [Schema → Types → `AuditEntry`](#auditentry) |
| 15 | `AttestationRequest` | object | [Schema → Types → `AttestationRequest`](#attestationrequest) |
| 16 | `Endorsement` | object | [Schema → Types → `Endorsement`](#endorsement) |
| 17 | `Template` | object | [Schema → Types → `Template`](#template) |
| 18 | `Delegation` | object | [Schema → Types → `Delegation`](#delegation) |
| 19 | `WhitelistEntry` | object | [Schema → Types → `WhitelistEntry`](#whitelistentry) |
| 20 | `CouncilAction` | object | [Schema → Types → `CouncilAction`](#councilaction) |
| 21 | `AttestationRevoked` | object | [Schema → Types → `AttestationRevoked`, `IssuerRegistered`](#attestationrevoked-issuerregistered) |
| 22 | `IssuerRegistered` | object | [Schema → Types → `AttestationRevoked`, `IssuerRegistered`](#attestationrevoked-issuerregistered) |

### SDL notes / ambiguities

- The SDL's `Query` type defines **9** fields; issue #1304's title says 8. The
  discrepancy is in the issue text — `src/schema.graphql:134-171` lists
  `healthCheck`, `attestations`, `attestationsByIssuer`, `issuer`, `issuers`,
  `issuerStats`, `proposal`, `proposals`, `auditLog`.
- `AttestationRequest`, `Endorsement`, `Template`, `Delegation`, `WhitelistEntry`
  and `CouncilAction` are defined in `src/schema.graphql` (`:227-281`) but are
  **not reachable** from `type Query`: the SDL declares no query field returning
  them. They are documented here as type definitions only; this document does not
  invent query fields for them.
- `first` on `attestations`/`attestationsByIssuer` is declared as `first: Int`
  with no default or maximum in the SDL; the 50/100 default/max noted above were
  carried over from the previous revision of this document (server-side
  behaviour, not schema metadata).
