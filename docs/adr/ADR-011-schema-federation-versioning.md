# ADR-011: GraphQL Schema Federation and Versioning Strategy

**Status:** Accepted  
**Date:** 2026-07-29  
**Deciders:** TrustLink indexer team  
**Issues:** [#976](https://github.com/afurious/TrustLink/issues/976)

---

## Context

The TrustLink indexer currently exposes a monolithic GraphQL schema
(`indexer/src/schema.graphql`) that covers attestations, multi-sig proposals,
audit entries, issuers, webhooks, attestation requests, endorsements,
delegations, templates, whitelists, and council actions — all served by a
single Node.js process.

As the protocol's on-chain feature set expands, the schema will grow. At some
threshold, splitting into independently-deployable subgraphs becomes
operationally attractive:

- **Scaling:** attestation-read traffic is orders of magnitude higher than
  multi-sig or council-action traffic; they should scale independently.
- **Team ownership:** separate teams could own separate subgraphs without
  coordinating a shared deployment.
- **Schema evolution:** a subgraph can introduce breaking changes on its own
  release cadence without halting the entire API surface.

Retrofitting federation *after* clients depend on today's monolithic schema is
significantly more disruptive than adding the metadata now. The goal of this
ADR is to make a future split **additive** rather than **breaking**.

---

## Decision

### 1. Adopt Apollo Federation v2 as the target federation standard

Apollo Federation v2 is the de-facto standard for GraphQL service composition
in production environments. It is supported by Apollo Router, Apollo Gateway,
and several open-source alternatives (e.g. [Cosmo](https://cosmo-docs.wundergraph.com/),
[Bramble](https://movio.github.io/bramble/)). Choosing it now costs nothing
(metadata only) and makes the migration path well-documented.

**We do NOT deploy a gateway or split services today.** This ADR only:
1. Identifies `@key` candidates on core types.
2. Adds schema-version metadata.
3. Documents the recommended split topology for when the time comes.

### 2. Schema versioning

Add an explicit `schemaVersion` field to `HealthStatus` so consumers can
detect schema-level breaking changes independently of the process version:

```graphql
type HealthStatus {
  status: String!
  lastLedger: Int
  timestamp: String!
  schemaVersion: String!   # semver string, e.g. "1.0.0"
}
```

The `schemaVersion` value is kept in a dedicated constant
(`SCHEMA_VERSION = "1.0.0"`) in `graphql.ts` and incremented on every
change that would require client adaptation.

### 3. Federation-ready `@key` candidates

The following types are identified as subgraph entity candidates. When a split
occurs, these become `@key`-annotated entities resolved by their respective
subgraphs.

#### `Attestation` — primary key: `id`

```graphql
# future subgraph annotation (not active today)
type Attestation @key(fields: "id") {
  id: String!
  issuer: String!
  subject: String!
  claimType: String!
  # ... remaining fields
}
```

`id` is a deterministic hash produced by the Soroban contract and is globally
unique. It is the natural primary key for federation `@key`.

#### `Issuer` — primary key: `address`

```graphql
# future subgraph annotation (not active today)
type Issuer @key(fields: "address") {
  address: String!
  name: String!
  # ... remaining fields
}
```

`address` is the Stellar public key of the issuer and is globally unique.

#### `MultisigProposal` — primary key: `id`

```graphql
# future subgraph annotation (not active today)
type MultisigProposal @key(fields: "id") {
  id: String!
  # ... remaining fields
}
```

### 4. Recommended subgraph split topology (future)

When the decision is made to federate, the recommended initial split is:

| Subgraph | Types | Rationale |
|---|---|---|
| `attestation-subgraph` | `Attestation`, `AuditEntry`, `AttestationRequest` | Highest read volume; benefits most from independent horizontal scaling |
| `issuer-subgraph` | `Issuer`, `IssuerStats` | Low-write, cache-friendly; issuer data changes infrequently |
| `governance-subgraph` | `MultisigProposal`, `CouncilAction`, `Delegation` | Governance operations have different SLA/availability requirements |
| `webhook-subgraph` | (internal, not exposed in public API) | Operational concern; can be hidden behind gateway |

The `Query.attestations`, `Query.issuer`, and `Subscription.*` entry points
remain at the gateway layer. The gateway composes responses from the subgraphs
transparently.

### 5. Schema change governance

| Change type | Required action |
|---|---|
| Add optional field | Bump patch version (`1.0.0` → `1.0.1`) |
| Add new type or query | Bump minor version (`1.0.0` → `1.1.0`) |
| Remove or rename field / change argument type | Bump major version (`1.0.0` → `2.0.0`), maintain old field deprecated for ≥ 2 releases |
| Add `@key` or federation directive | Minor bump; no client-visible change |

---

## Consequences

**Positive:**
- Future federation split is additive (add directives, deploy gateway) rather
  than breaking (rename fields, rebuild clients).
- `schemaVersion` gives integrators a reliable signal for API compatibility.
- `@key` candidates are documented and reviewed before clients depend on the
  current layout.

**Negative:**
- Minor boilerplate: `schemaVersion` must be maintained in `graphql.ts`.
- Teams must follow the change governance table or the version signal becomes
  meaningless.

**Neutral:**
- No performance impact. Federation directives are metadata; they have no
  runtime effect until a gateway is introduced.
- No dependency on `@apollo/subgraph` or any federation library until the
  gateway is actually deployed.

---

## Alternatives Considered

### A. Do nothing until federation is needed

Rejected. Retrofitting `@key` after hundreds of clients depend on the schema
requires a coordinated multi-team migration. The cost of adding metadata now is
effectively zero.

### B. Use schema stitching instead of Apollo Federation

Schema stitching predates Federation and has significantly worse tooling
support in 2026. Federation v2 is the industry standard. Rejected.

### C. Use Hasura or a different BaaS layer

Out of scope. TrustLink's indexer has event-driven ingestion logic
(Soroban RPC polling, dead-letter handling, webhook dispatch) that does not map
cleanly to a BaaS. Rejected.

---

## References

- [Apollo Federation v2 documentation](https://www.apollographql.com/docs/federation/)
- [GraphQL Schema Versioning best practices](https://graphql.org/learn/best-practices/)
- [ADR-004: Dual indexes](./ADR-004-dual-indexes.md) — context for Issuer and Subject as first-class entities
- [Issue #976](https://github.com/afurious/TrustLink/issues/976)
