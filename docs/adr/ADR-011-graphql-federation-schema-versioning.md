# ADR-011: GraphQL Schema Versioning and Federation Strategy

**Status:** Accepted  
**Date:** 2026-07-29  
**Issue:** [#976](https://github.com/Idaonoli/TrustLink/issues/976)

---

## Context

The TrustLink indexer currently exposes a single, monolithic GraphQL schema
(`indexer/src/schema.graphql`) that covers attestations, issuers, multi-sig
proposals, audit logs, webhooks, delegations, templates, whitelists, and the
admin council. As the feature surface grows this creates a maintenance burden
and makes it increasingly painful to:

1. **Deploy subsets of the API independently** — all functionality is coupled
   into one process.
2. **Scale hot resources separately** — e.g. the `attestations` query has very
   different throughput requirements from `auditLog`.
3. **Add a federation gateway later without breaking existing clients** —
   retrofitting federation key directives after clients have hardened
   expectations on the monolithic schema is disruptive.

---

## Decision

### 1. Schema versioning via a `schemaVersion` sentinel

A `schemaVersion` string constant is maintained in `schema.graphql` as a
doc-comment header and in the `healthCheck` response. This lets clients detect
schema changes without needing out-of-band version negotiation.

Current version: **`2`** (2026-07-29 — adds consistent subscription filters,
`subject`/`claimType` fields on `AttestationRevoked`, federation key comments).

Version history:
| Version | Date       | Changes                                        |
|---------|------------|------------------------------------------------|
| 1       | 2026-01-01 | Initial schema                                 |
| 2       | 2026-07-29 | Consistent subscription args (#974), federation key annotations (#976) |

### 2. Federation key candidates — annotated, not yet split

Rather than immediately adopting Apollo Federation (which requires a gateway
and changes the deployment topology), this ADR takes a **progressive
federation** approach:

- Identify the fields that *would* become `@key` directives in a future split.
- Document them as doc-comments on the affected types now, so a future split
  is additive and non-breaking.
- No gateway, no `@link` SDL, no subgraph stitching — yet.

This means the current schema remains a single monolith but is structured to
make a future split straightforward.

### 3. Federation key field decisions

| Type               | Key field  | Rationale                                                                                    |
|--------------------|------------|----------------------------------------------------------------------------------------------|
| `Attestation`      | `id`       | Globally unique deterministic hash from the contract; stable forever.                        |
| `Issuer`           | `address`  | Stellar address; globally unique; already the Prisma primary key.                           |
| `MultisigProposal` | `id`       | Deterministic proposal ID from the contract.                                                 |

No key candidates for `AuditEntry`, `AttestationRequest`, `Webhook`, etc.
because those types are always accessed through a parent entity relation and
do not need independent resolution.

### 4. Evaluation of federation libraries

| Option                        | Pros                                                     | Cons                                                    | Decision |
|-------------------------------|----------------------------------------------------------|---------------------------------------------------------|----------|
| **Apollo Federation v2**      | Mature, best gateway (Router), OTel support              | Requires gateway process, SDL `@link` pragma, subgraph SDL changes | **Deferred** — adopt when first split is needed |
| **GraphQL Modules**           | Lightweight code-splitting within a single process       | No gateway, no cross-service federation                 | Not suitable for multi-service splits |
| **Schema stitching (manual)** | No external dependency                                   | Manual maintenance of merged schemas; error-prone       | Avoid |
| **Hive Gateway / Cosmo**      | Open source, Apollo-compatible                           | Less mature than Apollo Router                          | Acceptable alternative to Apollo Router if licence matters |

**Chosen path:** Continue with the monolith. When the first subgraph split
is warranted (most likely separating the `Attestation` type from the issuer
management types), adopt **Apollo Federation v2** with the `@apollo/subgraph`
package and the open-source Apollo Router.

### 5. Subscription filter consistency (covered by #974, recorded here)

All `Subscription` fields now accept the same optional filter arguments:
`subject`, `issuer`, and `claimType`. This is enforced at the schema level so
future subscription additions automatically surface the gap during review.

---

## Consequences

### Positive

- Existing clients see no breaking change — the schema is backward-compatible.
- A future engineer splitting `Attestation` into its own subgraph has a clear,
  documented migration path: add `@link(url: "...", import: ["@key"])` and
  replace the doc-comment annotation with the actual directive.
- Schema version tracking lets clients detect incompatible changes without
  polling a separate metadata endpoint.

### Negative / Trade-offs

- The `schemaVersion` sentinel is doc-comment–only. There is no runtime query
  that returns the schema version today (the `healthCheck` type could be
  extended to include it in a future iteration).
- Deferring federation means the split, when it comes, will still require
  some migration work (adding the SDL pragma, splitting resolvers, deploying a
  gateway). The annotations make this easier but do not eliminate the effort.

---

## Migration path to full federation (future)

When the split is triggered:

1. Install `@apollo/subgraph` in the indexer package.
2. Add to `schema.graphql`:
   ```graphql
   extend schema
     @link(url: "https://specs.apollo.dev/federation/v2.0",
           import: ["@key", "@shareable"])

   type Attestation @key(fields: "id") { ... }
   type Issuer      @key(fields: "address") { ... }
   ```
3. Implement `__resolveReference` for each entity type in the resolvers.
4. Deploy a second subgraph for whichever slice of the schema is being
   extracted.
5. Stand up an Apollo Router instance pointing at both subgraphs.
6. Update client `GRAPHQL_URL` to the gateway endpoint.

No client query changes are required if the split is done correctly — clients
do not observe subgraph boundaries.
