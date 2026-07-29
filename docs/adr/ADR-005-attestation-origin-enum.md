# ADR-005: Replace `imported`/`bridged` Booleans with `AttestationOrigin` Enum

**Status:** Accepted  
**Date:** 2026-04-24  
**Issue:** [#291](https://github.com/marvs8/TrustLink/issues/291)

---

## Context

The `Attestation` struct previously carried two boolean fields:

```rust
pub imported: bool,
pub bridged: bool,
```

These fields are mutually exclusive — an attestation can only be native, imported,
or bridged, never two at once. The implicit "native" state (both `false`) was
undocumented and easy to misread. Any future origin type would require yet another
boolean, making the struct harder to reason about.

## Decision

Replace both booleans with a single `AttestationOrigin` enum:

```rust
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttestationOrigin {
    Native,    // created directly by a registered issuer
    Imported,  // migrated from an external source by the admin
    Bridged,   // mirrored from another chain by a trusted bridge contract
}
```

The `Attestation` struct now has:

```rust
pub origin: AttestationOrigin,
```

instead of `imported: bool` and `bridged: bool`.

## Consequences

### Positive
- All three states are explicit and exhaustive — no implicit "both false" case.
- Adding a new origin in the future is a single enum variant, not a new boolean.
- Pattern matching on `origin` is exhaustive; the compiler enforces handling every case.
- Cleaner API surface for consumers querying attestation records.

### Negative / Breaking Change
- **This is a breaking storage change.** Existing on-chain `Attestation` records
  serialized with the old layout (`imported: bool, bridged: bool`) are not
  forward-compatible with the new layout (`origin: AttestationOrigin`).

---

## Migration Plan

Because Soroban uses XDR-based serialization for `#[contracttype]` structs, the
on-chain binary layout changes when fields are added, removed, or reordered.
Existing attestation records stored under the old schema will fail to deserialize
after the upgrade.

### Step 1 — Deploy a migration contract (recommended for production)

Write a one-shot migration contract (or a migration entry-point on the main
contract) that:

1. Iterates every known attestation ID (from the `SubjectAttestations` and
   `IssuerAttestations` indexes).
2. Reads each record using the **old** schema (a temporary `AttestationV1` struct
   with `imported: bool, bridged: bool`).
3. Converts to the new schema:
   - `imported == true` → `origin: AttestationOrigin::Imported`
   - `bridged == true`  → `origin: AttestationOrigin::Bridged`
   - both `false`       → `origin: AttestationOrigin::Native`
4. Writes the record back using the **new** `Attestation` struct.

### Step 2 — Testnet dry-run

Run the migration against a testnet fork with a snapshot of production state.
Verify that:
- All attestation IDs resolve correctly after migration.
- `has_valid_claim` and related queries return the same results as before.
- No records are lost or corrupted.

### Step 3 — Coordinated mainnet upgrade

1. Pause new attestation writes (optional, via an admin-controlled circuit breaker).
2. Deploy the upgraded contract WASM.
3. Invoke the migration entry-point.
4. Resume normal operation.

### Step 4 — Remove migration code

After confirming all records are migrated, remove the migration entry-point in a
follow-up deployment to reduce attack surface.

---

## Alternatives Considered

**Keep the booleans, add documentation** — rejected because it doesn't prevent
invalid states (`imported == true && bridged == true`) and doesn't scale to
future origin types.

**Use a `u8` tag** — rejected because an enum is self-documenting and
compiler-checked, while a raw integer is not.
