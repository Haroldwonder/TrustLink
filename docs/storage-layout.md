# TrustLink — On-Chain Storage Layout

This document describes every storage key used by the TrustLink contract, the
data each key holds, which storage tier it lives in, the TTL policy applied to
it, and the serialization format. It is intended for developers building
indexers, analytics tools, or off-chain integrations that read contract state
directly via RPC.

---

## Storage tiers

Soroban provides two persistent storage tiers. TrustLink uses both:

| Tier           | Used for                        | TTL behaviour                                      |
|----------------|---------------------------------|----------------------------------------------------|
| **Instance**   | `Admin`, `Version`, `FeeConfig` | Single shared TTL; refreshed to 30 days on every admin write |
| **Persistent** | All other keys (see table below)| Per-key TTL; refreshed to 30 days on every write of that key |

30 days is calculated as `17 280 ledgers/day × 30 = 518 400 ledgers`
(`DAY_IN_LEDGERS = 17_280`, `INSTANCE_LIFETIME = 518_400`).

A key that is never written again will be evicted from the ledger once its TTL
reaches zero. Any contract call that writes a key resets that key's TTL to the
full 30-day window.

---

## Serialization format

All keys and values are encoded using **Soroban's XDR `contracttype` codec**.
Every Rust type annotated with `#[contracttype]` is automatically serialized to
`ScVal` XDR when stored and deserialized back when read. There is no custom
serialization logic in TrustLink — the SDK handles it entirely.

The `StorageKey` enum itself is also `#[contracttype]`, so each variant
serializes to a distinct `ScVal` discriminant that Soroban uses as the raw
storage key on-chain.

---

## Storage key reference

### 1. `Admin`

| Property      | Value                          |
|---------------|--------------------------------|
| Tier          | Instance                       |
| TTL           | Shared instance TTL, 30 days, refreshed on every `set_admin` call |
| Value type    | `Address`                      |
| Written by    | `initialize`                   |
| Read by       | `get_admin`, `Validation::require_admin` |

Stores the single contract administrator address set during `initialize`. There
is exactly one `Admin` entry per contract instance. The key is a unit variant
(`StorageKey::Admin`) with no parameters.

**Rust type:**
```rust
Address
```

---

### 2. `Version`

| Property      | Value                          |
|---------------|--------------------------------|
| Tier          | Instance                       |
| TTL           | Shared instance TTL, 30 days, refreshed on every `set_admin` call |
| Value type    | `String`                       |
| Written by    | `initialize`                   |
| Read by       | `get_version`, `get_contract_metadata` |

Stores the semver version string set at initialization (currently `"1.0.0"`).
Lives in instance storage alongside `Admin` and shares the same TTL entry.

**Rust type:**
```rust
String   // e.g. "1.0.0"
```

---

### 3. `FeeConfig`

| Property      | Value                          |
|---------------|--------------------------------|
| Tier          | Instance                       |
| TTL           | Shared instance TTL, 30 days, refreshed on every `set_fee_config` call |
| Value type    | `FeeConfig`                    |
| Written by    | `initialize`, `set_fee`        |
| Read by       | `get_fee_config`, `create_attestation` |

Stores the global fee policy for native attestation creation. The fee is
disabled by default by storing `attestation_fee = 0`, `fee_collector = admin`,
and `fee_token = None` during `initialize`.

When `attestation_fee > 0`, `create_attestation` transfers that amount of the
configured `fee_token` from the issuer to `fee_collector` before persisting the
attestation.

**Rust type:**
```rust
pub struct FeeConfig {
    pub attestation_fee: i128,      // amount charged on create_attestation
    pub fee_collector: Address,     // recipient of collected fees
    pub fee_token: Option<Address>, // token contract used for collection
}
```

---

### 4. `Issuer(Address)`

| Property      | Value                                              |
|---------------|----------------------------------------------------|
| Tier          | Persistent                                         |
| TTL           | Per-key, 30 days, refreshed on `add_issuer`        |
| Value type    | `bool` (always `true` when present)                |
| Written by    | `register_issuer`                                  |
| Deleted by    | `remove_issuer`                                    |
| Read by       | `is_issuer`, `Validation::require_issuer`          |

One entry exists per registered issuer. The key embeds the issuer's `Address`
as a parameter. Presence of the key means the address is authorized; absence
means it is not. The stored value is always `true` — the key acts as a set
membership flag.

**Rust type:**
```rust
bool   // always true
```

---

### 5. `Bridge(Address)`

| Property      | Value                                              |
|---------------|----------------------------------------------------|
| Tier          | Persistent                                         |
| TTL           | Per-key, 30 days, refreshed on `add_bridge`        |
| Value type    | `bool` (always `true` when present)                |
| Written by    | `register_bridge`                                  |
| Read by       | `is_bridge`, `Validation::require_bridge`          |

One entry exists per trusted bridge contract. The key embeds the bridge
contract `Address` as a parameter. Presence of the key means the contract is
allowed to create bridged attestations; absence means it is not.

**Rust type:**
```rust
bool   // always true
```

---

### 6. `Attestation(String)`

| Property      | Value                                                    |
|---------------|----------------------------------------------------------|
| Tier          | Persistent                                               |
| TTL           | Per-key, 30 days, refreshed on every `set_attestation`   |
| Value type    | `Attestation` struct                                     |
| Written by    | `create_attestation`, `import_attestation`, `bridge_attestation`, `revoke_attestation`, `renew_attestation`, `update_expiration`, `revoke_attestations_batch` |
| Read by       | `get_attestation`, `get_attestation_status`, `has_valid_claim`, `has_any_claim`, `has_all_claims`, `get_valid_claims`, `get_attestation_by_type` |

The primary attestation record. The key parameter is the 32-character hex
attestation ID (a SHA-256-derived string). Attestations are never deleted —
revocation sets `revoked = true` in place.

**Rust type:**
```rust
pub struct Attestation {
    pub id:          String,          // 32-char hex ID
    pub issuer:      Address,         // issuer who created it
    pub subject:     Address,         // address being attested about
    pub claim_type:  String,          // e.g. "KYC_PASSED"
    pub timestamp:   u64,             // ledger timestamp at creation (seconds)
    pub expiration:  Option<u64>,     // optional expiry (seconds); None = no expiry
    pub revoked:     bool,            // true once revoke_attestation is called
    pub metadata:    Option<String>,  // optional issuer-supplied metadata
    pub valid_from:  Option<u64>,     // optional future activation time (seconds)
    pub imported:    bool,            // true when imported from an external source
    pub bridged:     bool,            // true when created by a bridge contract
    pub source_chain: Option<String>, // original chain for bridged attestations
    pub source_tx:   Option<String>,  // original transaction/reference for bridged attestations
}
```

**Status derivation** (computed at query time, not stored):

| Condition                                  | Status    |
|--------------------------------------------|-----------|
| `valid_from` is set and `now < valid_from` | `Pending` |
| `revoked == true`                          | `Revoked` |
| `expiration` is set and `now >= expiration`| `Expired` |
| None of the above                          | `Valid`   |

Priority order: `Pending` > `Revoked` > `Expired` > `Valid`.

---

### 7. `SubjectAttestations(Address)`

| Property      | Value                                                        |
|---------------|--------------------------------------------------------------|
| Tier          | Persistent                                                   |
| TTL           | Per-key, 30 days, refreshed on every `add_subject_attestation` |
| Value type    | `Vec<String>` — ordered list of attestation IDs             |
| Written by    | `create_attestation`, `import_attestation`, `bridge_attestation` |
| Read by       | `get_subject_attestations`, `has_valid_claim`, `has_any_claim`, `has_all_claims`, `get_valid_claims`, `get_attestation_by_type` |

An append-only index mapping a subject address to all attestation IDs ever
created for that subject (including revoked and expired ones). Used for
pagination (`get_subject_attestations`) and for scanning all claims during
verification queries. IDs appear in insertion order.

**Rust type:**
```rust
Vec<String>   // ordered list of 32-char hex attestation IDs
```

---

### 8. `IssuerAttestations(Address)`

| Property      | Value                                                       |
|---------------|-------------------------------------------------------------|
| Tier          | Persistent                                                  |
| TTL           | Per-key, 30 days, refreshed on every `add_issuer_attestation` |
| Value type    | `Vec<String>` — ordered list of attestation IDs            |
| Written by    | `create_attestation`, `import_attestation`, `bridge_attestation` |
| Read by       | `get_issuer_attestations`                                   |

An append-only index mapping an attestation creator address to all attestation
IDs that address has ever created. For native/imported attestations this is the
issuer address; for bridged attestations this is the bridge contract address.
Used for pagination via `get_issuer_attestations`. IDs appear in insertion
order.

**Rust type:**
```rust
Vec<String>   // ordered list of 32-char hex attestation IDs
```

---

### 9. `IssuerMetadata(Address)`

| Property      | Value                                                    |
|---------------|----------------------------------------------------------|
| Tier          | Persistent                                               |
| TTL           | Per-key, 30 days, refreshed on every `set_issuer_metadata` |
| Value type    | `IssuerMetadata` struct                                  |
| Written by    | `set_issuer_metadata`                                    |
| Read by       | `get_issuer_metadata`                                    |

Optional public profile that a registered issuer can attach to their address.
The key is absent until the issuer calls `set_issuer_metadata` for the first
time. Subsequent calls overwrite the existing record.

**Rust type:**
```rust
pub struct IssuerMetadata {
    pub name:        String,   // human-readable issuer name
    pub url:         String,   // issuer's website or documentation URL
    pub description: String,   // short description of the issuer's role
}
```

---

### 10. `ClaimType(String)`

| Property      | Value                                                  |
|---------------|--------------------------------------------------------|
| Tier          | Persistent                                             |
| TTL           | Per-key, 30 days, refreshed on every `set_claim_type`  |
| Value type    | `ClaimTypeInfo` struct                                 |
| Written by    | `register_claim_type`                                  |
| Read by       | `get_claim_type_description`                           |

One entry per registered claim type. The key parameter is the claim type
identifier string (e.g. `"KYC_PASSED"`). Re-registering an existing claim type
overwrites the description in place without adding a duplicate to
`ClaimTypeList`.

**Rust type:**
```rust
pub struct ClaimTypeInfo {
    pub claim_type:  String,   // identifier, e.g. "KYC_PASSED"
    pub description: String,   // human-readable description
}
```

---

### 11. `ClaimTypeList`

| Property      | Value                                                      |
|---------------|------------------------------------------------------------|
| Tier          | Persistent                                                 |
| TTL           | Per-key, 30 days, refreshed whenever a new claim type is registered |
| Value type    | `Vec<String>` — ordered list of claim type identifiers     |
| Written by    | `register_claim_type` (only when a new type is added)      |
| Read by       | `list_claim_types`                                         |

A global ordered list of all registered claim type identifier strings. New
identifiers are appended on first registration; re-registering an existing type
does **not** append a duplicate. Used to support paginated listing via
`list_claim_types(start, limit)`.

**Rust type:**
```rust
Vec<String>   // ordered list of claim type identifier strings
```

---

## Summary table

| Key                          | Tier       | Value type          | TTL window | Refreshed on write? |
|------------------------------|------------|---------------------|------------|---------------------|
| `Admin`                      | Instance   | `Address`           | 30 days    | Yes (shared)        |
| `Version`                    | Instance   | `String`            | 30 days    | Yes (shared)        |
| `FeeConfig`                  | Instance   | `FeeConfig`         | 30 days    | Yes (shared)        |
| `Issuer(Address)`            | Persistent | `bool`              | 30 days    | Yes (per-key)       |
| `Bridge(Address)`            | Persistent | `bool`              | 30 days    | Yes (per-key)       |
| `Attestation(String)`        | Persistent | `Attestation`       | 30 days    | Yes (per-key)       |
| `SubjectAttestations(Address)`| Persistent | `Vec<String>`      | 30 days    | Yes (per-key)       |
| `IssuerAttestations(Address)`| Persistent | `Vec<String>`       | 30 days    | Yes (per-key)       |
| `IssuerMetadata(Address)`    | Persistent | `IssuerMetadata`    | 30 days    | Yes (per-key)       |
| `ClaimType(String)`          | Persistent | `ClaimTypeInfo`     | 30 days    | Yes (per-key)       |
| `ClaimTypeList`              | Persistent | `Vec<String>`       | 30 days    | Yes (on new entry)  |

---

## Reading storage via RPC

The following example shows how to read an `Attestation` record directly from
a Soroban RPC node without invoking the contract. This is useful for indexers
and analytics tools that need raw state access.

### Prerequisites

- A Soroban-compatible RPC endpoint (e.g. Testnet: `https://soroban-testnet.stellar.org`)
- The contract ID
- The attestation ID (32-char hex string returned by `create_attestation`)

### Step 1 — Encode the storage key as XDR

The storage key for an attestation is `StorageKey::Attestation(id)`. In XDR
`ScVal` terms this is a `SCV_VEC` containing two elements:

1. The enum discriminant symbol `"Attestation"` as `SCV_SYMBOL`
2. The attestation ID string as `SCV_STRING`

Using the JavaScript Stellar SDK:

```js
import { xdr, Contract, SorobanRpc } from "@stellar/stellar-sdk";

const server = new SorobanRpc.Server("https://soroban-testnet.stellar.org");
const contractId = "C..."; // your deployed contract ID
const attestationId = "a3f1..."; // 32-char hex ID from create_attestation

// Build the StorageKey::Attestation(id) ScVal
const key = xdr.ScVal.scvVec([
  xdr.ScVal.scvSymbol("Attestation"),
  xdr.ScVal.scvString(attestationId),
]);

const ledgerKey = xdr.LedgerKey.contractData(
  new xdr.LedgerKeyContractData({
    contract: new Contract(contractId).address().toScAddress(),
    key,
    durability: xdr.ContractDataDurability.persistent(),
  })
);

const response = await server.getLedgerEntries(ledgerKey);
const entry = response.entries[0];

// Decode the value back to a JS object
const val = entry.val.contractData().val();
console.log(val.value()); // raw ScVal — use scValToNative() for a plain object
```

### Step 2 — Decode the result

The returned `ScVal` is an `SCV_MAP` whose fields correspond to the `Attestation`
struct in declaration order:

| Field        | ScVal type    | Notes                              |
|--------------|---------------|------------------------------------|
| `id`         | `SCV_STRING`  | 32-char hex                        |
| `issuer`     | `SCV_ADDRESS` | Stellar strkey (G… or C…)          |
| `subject`    | `SCV_ADDRESS` | Stellar strkey                     |
| `claim_type` | `SCV_STRING`  | e.g. `"KYC_PASSED"`               |
| `timestamp`  | `SCV_U64`     | Ledger timestamp at creation       |
| `expiration` | `SCV_VEC` or `SCV_VOID` | `Some(u64)` or `None`  |
| `revoked`    | `SCV_BOOL`    |                                    |
| `valid_from` | `SCV_VEC` or `SCV_VOID` | `Some(u64)` or `None`  |

Using `scValToNative` from `@stellar/stellar-sdk` will convert the map to a
plain JavaScript object automatically.

### Reading instance storage (Admin / Version)

Instance storage keys use `ContractDataDurability.instance()` instead of
`persistent()`, and the key is a plain symbol with no parameters:

```js
const adminKey = xdr.LedgerKey.contractData(
  new xdr.LedgerKeyContractData({
    contract: new Contract(contractId).address().toScAddress(),
    key: xdr.ScVal.scvSymbol("Admin"),
    durability: xdr.ContractDataDurability.instance(),
  })
);
```

---

## Notes for indexer developers

- **Attestations are never deleted.** An attestation with `revoked: true` stays
  in storage indefinitely (subject to TTL). Index both active and revoked
  records if you need a complete history.
- **TTL eviction.** A key that is not touched for 30 days will be evicted.
  Indexers should snapshot state proactively rather than relying on keys always
  being present.
- **Subject and issuer indexes are maintained for pagination.** `SubjectAttestations`
  and `IssuerAttestations` store ordered attestation IDs used by listing queries.
  When an attestation is revoked, its ID is removed from both indexes so
  pagination counts shrink; the attestation record itself remains in storage
  (with `revoked = true`) until TTL eviction.
- **`ClaimTypeList` is insertion-ordered.** The order reflects the sequence in
  which `register_claim_type` was first called for each type.
- **Status is computed, not stored.** `AttestationStatus` (`Valid`, `Expired`,
  `Revoked`, `Pending`) is derived at query time from the stored fields and the
  current ledger timestamp. Indexers must replicate this logic locally.
