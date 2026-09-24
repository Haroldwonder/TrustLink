# TrustLink — On-Chain Storage Layout

This document describes every storage key used by the TrustLink contract, the
data each key holds, which storage tier it lives in, the TTL policy applied to
it, and the serialization format. It is intended for developers building
indexers, analytics tools, or off-chain integrations that read contract state
directly via RPC.

---

## Storage tiers

Soroban provides two persistent storage tiers. TrustLink uses both:

| Tier           | Used for                                          | TTL behaviour                                      |
|----------------|---------------------------------------------------|----------------------------------------------------|
| **Instance**   | Singleton contract state (admin council, config, global stats, paused flag, etc.) | Single shared TTL; refreshed to 30 days on every admin write |
| **Persistent** | Per-address and per-ID records (attestations, indexes, proposals, etc.) | Per-key TTL; refreshed to 30 days on every write of that key |

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

> **Coverage:** all 47 `StorageKey` variants defined in `src/storage.rs` plus
> the `ClaimTypeIssuanceKey` composite key. Entries are grouped by storage tier
> and then by functional area.

---

### Instance storage keys

All instance keys share one TTL entry. Writing **any** instance key refreshes
the TTL for **all** of them simultaneously.

---

#### 1. `AdminCouncil`

| Property      | Value                          |
|---------------|--------------------------------|
| Tier          | Instance                       |
| TTL           | Shared instance TTL, refreshed on every `set_admin_council` call |
| Value type    | `AdminCouncil` (`Vec<Address>`) |
| Written by    | `initialize`, `add_admin`, `remove_admin`, `accept_admin_transfer` |
| Read by       | `get_admin`, `get_admin_council`, `is_admin`, `Validation::require_admin` |

The ordered list of all current administrators. The first element is treated as
the primary admin for backwards-compatible single-admin reads. Presence of at
least one entry is used by `has_admin` to determine whether the contract has
been initialized.

**Rust type:**
```rust
Vec<Address>   // AdminCouncil alias
```

---

#### 2. `Version`

| Property      | Value                          |
|---------------|--------------------------------|
| Tier          | Instance                       |
| TTL           | Shared instance TTL            |
| Value type    | `String`                       |
| Written by    | `initialize`                   |
| Read by       | `get_version`, `get_contract_metadata` |

Stores the semver version string set at initialization (e.g. `"1.0.0"`).

**Rust type:**
```rust
String   // e.g. "1.0.0"
```

---

#### 3. `FeeConfig`

| Property      | Value                          |
|---------------|--------------------------------|
| Tier          | Instance                       |
| TTL           | Shared instance TTL            |
| Value type    | `FeeConfig`                    |
| Written by    | `initialize`, `set_fee`        |
| Read by       | `get_fee_config`, `create_attestation` |

Global fee policy for attestation creation. Fee is disabled by default
(`attestation_fee = 0`).

**Rust type:**
```rust
pub struct FeeConfig {
    pub attestation_fee: i128,
    pub fee_collector:   Address,
    pub fee_token:       Option<Address>,
}
```

---

#### 4. `TtlConfig`

| Property      | Value                          |
|---------------|--------------------------------|
| Tier          | Instance                       |
| TTL           | Shared instance TTL            |
| Value type    | `TtlConfig`                    |
| Written by    | `initialize`, `set_ttl_config` |
| Read by       | `get_ttl_config`, `get_ttl_lifetime` (internal) |

Operator-configurable TTL window in days. When absent, `get_ttl_lifetime` falls
back to `DEFAULT_INSTANCE_LIFETIME` (518 400 ledgers / 30 days).

**Rust type:**
```rust
pub struct TtlConfig {
    pub ttl_days: u32,
}
```

---

#### 5. `ContractConfig`

| Property      | Value                          |
|---------------|--------------------------------|
| Tier          | Instance                       |
| TTL           | Shared instance TTL            |
| Value type    | `ContractConfig`               |
| Written by    | `initialize`, `set_require_registered_claim_type`, `set_metadata_hash_only`, `set_max_attestations_per_subject`, `set_chunk_size` |
| Read by       | `get_require_registered_claim_type`, `get_metadata_hash_only`, `get_max_attestations_per_subject`, `get_chunk_size`, `ChunkedIndex` (internal) |

Miscellaneous contract-wide behavioural flags and limits stored as a single
struct to minimise instance storage entries.

**Rust type:**
```rust
pub struct ContractConfig {
    pub require_registered_claim_type:   bool,
    pub metadata_hash_only:              bool,
    pub max_attestations_per_subject:    Option<u32>,
    pub chunk_size:                      u32,
}
```

---

#### 6. `GlobalStats`

| Property      | Value                          |
|---------------|--------------------------------|
| Tier          | Instance                       |
| TTL           | Shared instance TTL            |
| Value type    | `GlobalStats`                  |
| Written by    | Internal helpers `increment_total_attestations`, `increment_total_revocations`, `increment_total_issuers`, `decrement_total_issuers` |
| Read by       | `get_global_stats` |

Contract-wide counters incremented on every relevant mutation.

**Rust type:**
```rust
pub struct GlobalStats {
    pub total_attestations: u64,
    pub total_revocations:  u64,
    pub total_issuers:      u64,
}
```

---

#### 7. `Paused`

| Property      | Value                          |
|---------------|--------------------------------|
| Tier          | Instance                       |
| TTL           | Shared instance TTL (refreshed via `DEFAULT_INSTANCE_LIFETIME`) |
| Value type    | `bool`                         |
| Written by    | `pause`, `unpause`             |
| Read by       | `is_paused`, `Validation::require_not_paused` |

Contract-wide pause flag. When `true`, all state-mutating calls are rejected.
Absent means not paused (treated as `false`).

**Rust type:**
```rust
bool
```

---

#### 8. `ClaimTypeRateLimit(String)`

| Property      | Value                          |
|---------------|--------------------------------|
| Tier          | Instance                       |
| TTL           | Shared instance TTL            |
| Value type    | `u64`                          |
| Key param     | Claim type identifier string   |
| Written by    | `set_rate_limit_for_claim_type`|
| Read by       | `get_rate_limit_for_claim_type`|

Per-claim-type minimum issuance interval in seconds. Overrides the global rate
limit (`MiscConfig.rate_limit_config`) for the specific claim type. Absent
means no per-type override applies.

**Rust type:**
```rust
u64   // minimum seconds between successive issuances of this claim type
```

---

#### 9. `ProposalCounter`

| Property      | Value                          |
|---------------|--------------------------------|
| Tier          | Instance                       |
| TTL           | Shared instance TTL            |
| Value type    | `u32`                          |
| Written by    | `next_proposal_id` (internal, called by council proposal creation) |
| Read by       | `next_proposal_id`             |

Monotonically increasing counter used to generate unique `CouncilProposal` IDs.
Starts at 0; each call to `next_proposal_id` increments it and returns the new
value.

**Rust type:**
```rust
u32
```

---

#### 10. `PendingAdminTransfer`

| Property      | Value                          |
|---------------|--------------------------------|
| Tier          | Instance                       |
| TTL           | Shared instance TTL            |
| Value type    | `PendingAdminTransfer`         |
| Written by    | `propose_admin_transfer`       |
| Deleted by    | `accept_admin_transfer`, `cancel_admin_transfer` |
| Read by       | `get_pending_admin_transfer`   |

Holds the proposed new admin address during a two-step admin handover. Absent
when no transfer is in progress.

**Rust type:**
```rust
pub struct PendingAdminTransfer {
    pub new_admin: Address,
}
```

---

#### 11. `MiscConfig`

| Property      | Value                          |
|---------------|--------------------------------|
| Tier          | Instance                       |
| TTL           | Shared instance TTL            |
| Value type    | `MiscConfig`                   |
| Written by    | `set_rate_limit`, `set_limits`, `set_decay_config`, `set_multisig_ttl`, `set_council_timelock_delay` |
| Read by       | `get_rate_limit_config`, `get_limits`, `get_decay_config`, `get_multisig_ttl`, `get_council_timelock_delay` |

A consolidated singleton bundle for infrequently-changed contract-wide settings.
Multiple logical config values are packed into one key to stay within the
`#[contracttype]` enum's 50-variant limit.

**Rust type:**
```rust
pub struct MiscConfig {
    pub multisig_ttl_days:        u32,
    pub rate_limit_config_set:    bool,
    pub rate_limit_config:        RateLimitConfig,
    pub decay_config_set:         bool,
    pub decay_config:             DecayConfig,
    pub council_timelock_delay:   u64,
    pub storage_limits:           StorageLimits,
}
```

---

### Persistent storage keys

Each persistent key has its own independent TTL, refreshed only when that key
is written.

---

#### 12. `Issuer(Address)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `add_issuer`            |
| Value type    | `bool` (always `true`)                        |
| Key param     | Issuer `Address`                              |
| Written by    | `register_issuer`                             |
| Deleted by    | `remove_issuer`                               |
| Read by       | `is_issuer`, `Validation::require_issuer`     |

Membership flag. Presence means the address is a registered issuer; absence
means it is not.

---

#### 13. `IssuerList`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `add_issuer` / `remove_issuer` |
| Value type    | `Vec<Address>`                                |
| Written by    | `register_issuer`, `remove_issuer`            |
| Read by       | `get_issuer_list`                             |

Ordered list of all registered issuer addresses, maintained in sync with
`Issuer(Address)` membership flags. Used for paginated `get_issuer_list`
queries.

**Rust type:**
```rust
Vec<Address>
```

---

#### 14. `IssuerMetadata(Address)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `set_issuer_metadata`   |
| Value type    | `IssuerMetadata`                              |
| Key param     | Issuer `Address`                              |
| Written by    | `set_issuer_metadata`                         |
| Read by       | `get_issuer_metadata`                         |

Optional public profile attached to a registered issuer. Absent until the
issuer calls `set_issuer_metadata` for the first time.

**Rust type:**
```rust
pub struct IssuerMetadata {
    pub name:        String,
    pub url:         String,
    pub description: String,
}
```

---

#### 15. `IssuerStats(Address)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `set_issuer_stats`      |
| Value type    | `IssuerStats`                                 |
| Key param     | Issuer `Address`                              |
| Written by    | `increment_issuer_stats` (internal)           |
| Read by       | `get_issuer_stats`                            |

Per-issuer cumulative attestation counter.

**Rust type:**
```rust
pub struct IssuerStats {
    pub total_issued: u64,
}
```

---

#### 16. `IssuerTier(Address)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `set_issuer_tier`       |
| Value type    | `IssuerTier`                                  |
| Key param     | Issuer `Address`                              |
| Written by    | `set_issuer_tier`                             |
| Read by       | `get_issuer_tier`                             |

Optional trust tier assigned to an issuer by an admin. Absent until explicitly
set.

**Rust type:**
```rust
pub enum IssuerTier {
    Basic,
    Verified,
    Premium,
}
```

---

#### 17. `IssuerRevocations(Address)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `increment_issuer_revocations` |
| Value type    | `u64`                                         |
| Key param     | Issuer `Address`                              |
| Written by    | `increment_issuer_revocations` (internal, called by `revoke_attestation`) |
| Read by       | `get_issuer_revocations`, confidence scoring logic |

Cumulative count of attestations revoked by this issuer. Used in the confidence
score decay calculation.

**Rust type:**
```rust
u64
```

---

#### 18. `Bridge(Address)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `add_bridge`            |
| Value type    | `bool` (always `true`)                        |
| Key param     | Bridge contract `Address`                     |
| Written by    | `register_bridge`                             |
| Read by       | `is_bridge`, `Validation::require_bridge`     |

Membership flag for trusted bridge contracts. Presence means the contract is
allowed to create bridged attestations.

---

#### 19. `BridgeList`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `add_bridge`            |
| Value type    | `Vec<Address>`                                |
| Written by    | `register_bridge`                             |
| Read by       | `get_bridge_list`                             |

Ordered list of all registered bridge contract addresses.

**Rust type:**
```rust
Vec<Address>
```

---

#### 20. `Attestation(String)`

| Property      | Value                                                    |
|---------------|----------------------------------------------------------|
| Tier          | Persistent                                               |
| TTL           | Per-key, refreshed on every `set_attestation`; extended to cover full `expiration` duration if that exceeds the default window |
| Value type    | `Attestation`                                            |
| Key param     | 32-char hex attestation ID                               |
| Written by    | `create_attestation`, `import_attestation`, `bridge_attestation`, `revoke_attestation`, `renew_attestation`, `update_expiration`, `revoke_attestations_batch`, `cosign_attestation` (on threshold) |
| Read by       | `get_attestation`, `get_attestation_status`, `has_valid_claim`, `has_any_claim`, `has_all_claims`, `get_valid_claims`, `get_attestation_by_type` |

Primary attestation record. Never deleted — revocation sets `revoked = true`
in place.

**Rust type:**
```rust
pub struct Attestation {
    pub id:           String,
    pub issuer:       Address,
    pub subject:      Address,
    pub claim_type:   String,
    pub timestamp:    u64,
    pub expiration:   Option<u64>,
    pub revoked:      bool,
    pub metadata:     Option<String>,
    pub valid_from:   Option<u64>,
    pub imported:     bool,
    pub bridged:      bool,
    pub source_chain: Option<String>,
    pub source_tx:    Option<String>,
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

#### 21. `AttestationHistory(String)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `push_attestation_version` |
| Value type    | `Vec<AttestationVersionSnapshot>`             |
| Key param     | Attestation ID string                         |
| Written by    | `push_attestation_version` (internal, called on attestation updates) |
| Read by       | `get_attestation_history`, `get_attestation_version_count` |

Ordered log of every previous version of an attestation record, enabling
full audit history. Each entry captures the state at the time of a mutation.

**Rust type:**
```rust
Vec<AttestationVersionSnapshot>
```

---

#### 22. `SubjectAttestations(Address)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `add_subject_attestation` |
| Value type    | `Vec<String>`                                 |
| Key param     | Subject `Address`                             |
| Written by    | `create_attestation`, `import_attestation`, `bridge_attestation` |
| Read by       | `get_subject_attestations`, claim verification functions |

> **Note:** `SubjectAttestations` and `IssuerAttestations` store a flat
> `Vec<String>` only when the chunked index has not been activated. Once
> `ChunkedIndex` is in use (the default), the individual chunk keys below
> replace this key. See `SubjectAttestationChunk` and `IssuerAttestationChunk`.

Ordered list of all attestation IDs ever created for a subject (including
revoked and expired). IDs appear in insertion order.

**Rust type:**
```rust
Vec<String>   // ordered list of 32-char hex attestation IDs
```

---

#### 23. `SubjectAttestationChunk(Address, u32)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on every chunk write       |
| Value type    | `Vec<String>`                                 |
| Key params    | Subject `Address`, chunk index `u32`          |
| Written by    | `ChunkedIndex::add_subject`, `ChunkedIndex::remove_subject` |
| Read by       | `ChunkedIndex::get_subject_page`, `ChunkedIndex::get_subject_all` |

One chunk of the subject's attestation ID list. Chunk size is configurable via
`ContractConfig.chunk_size` (default 50). Chunk indices start at 0 and are
contiguous; the first missing chunk signals end of list. Old trailing chunks
are deleted when the list shrinks.

**Rust type:**
```rust
Vec<String>
```

---

#### 24. `IssuerAttestations(Address)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on every write             |
| Value type    | `Vec<String>`                                 |
| Key param     | Issuer `Address`                              |
| Written by    | `create_attestation`, `import_attestation`, `bridge_attestation`, `create_attestations_batch` |
| Read by       | `get_issuer_attestations`                     |

Flat issuer attestation index, used alongside `IssuerAttestationChunk` (see
note on `SubjectAttestations` above).

**Rust type:**
```rust
Vec<String>
```

---

#### 25. `IssuerAttestationChunk(Address, u32)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on every chunk write       |
| Value type    | `Vec<String>`                                 |
| Key params    | Issuer `Address`, chunk index `u32`           |
| Written by    | `ChunkedIndex::add_issuer`, `ChunkedIndex::add_issuer_bulk`, `ChunkedIndex::remove_issuer` |
| Read by       | `ChunkedIndex::get_issuer_page`, `ChunkedIndex::get_issuer_all` |

One chunk of the issuer's attestation ID list. Same chunking behaviour as
`SubjectAttestationChunk`.

**Rust type:**
```rust
Vec<String>
```

---

#### 26. `ValidAttestations(Address)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on write                   |
| Value type    | `Vec<String>`                                 |
| Key param     | Subject `Address`                             |
| Written by    | `add_valid_attestation` (internal)            |
| Read by       | `get_valid_attestations`                      |

Optimised lookup index containing only currently-valid attestation IDs for a
subject. Entries are added on attestation creation and removed on revocation or
expiry, avoiding full-scan of `SubjectAttestations` for common validity checks.

**Rust type:**
```rust
Vec<String>
```

---

#### 27. `AuditLog(String)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `append_audit_entry`    |
| Value type    | `Vec<AuditEntry>`                             |
| Key param     | Attestation ID string                         |
| Written by    | `append_audit_entry` (internal, called on every state-changing attestation operation) |
| Read by       | `get_audit_log`                               |

Ordered log of all administrative actions performed on an attestation (create,
revoke, renew, transfer, etc.).

**Rust type:**
```rust
Vec<AuditEntry>
```

---

#### 28. `Endorsements(String)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `add_endorsement`       |
| Value type    | `Vec<Endorsement>`                            |
| Key param     | Attestation ID string                         |
| Written by    | `add_endorsement` (internal)                  |
| Read by       | `get_endorsements`                            |

All endorsements recorded against a specific attestation, in insertion order.

**Rust type:**
```rust
Vec<Endorsement>
```

---

#### 29. `EndorserIndex(Address)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `add_endorsement`       |
| Value type    | `Vec<Endorsement>`                            |
| Key param     | Endorser `Address`                            |
| Written by    | `add_endorsement` (internal)                  |
| Read by       | `get_endorsements_by_endorser`                |

Reverse index: all endorsements made by a given endorser across all
attestations.

**Rust type:**
```rust
Vec<Endorsement>
```

---

#### 30. `ExpirationHook(Address)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `set_expiration_hook`   |
| Value type    | `ExpirationHook`                              |
| Key param     | Subject `Address`                             |
| Written by    | `set_expiration_hook`                         |
| Deleted by    | `remove_expiration_hook`                      |
| Read by       | `get_expiration_hook`                         |

Optional callback configuration registered by a subject. When an attestation
for that subject expires, the contract invokes the specified hook contract.
Absent when no hook has been registered.

**Rust type:**
```rust
pub struct ExpirationHook {
    pub hook_contract: Address,
    pub hook_fn:       String,
}
```

---

#### 31. `IssuerWhitelistMode(Address)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `set_whitelist_mode`    |
| Value type    | `bool`                                        |
| Key param     | Issuer `Address`                              |
| Written by    | `set_whitelist_enabled`, `enable_whitelist_mode` |
| Read by       | `is_whitelist_enabled`, `Validation::require_whitelisted` |

Flag controlling whether this issuer enforces subject whitelisting. When
`true`, only addresses present in `IssuerWhitelist(issuer, subject)` may
receive attestations from this issuer.

**Rust type:**
```rust
bool
```

---

#### 32. `IssuerWhitelist(Address, Address)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `add_to_whitelist`      |
| Value type    | `bool` (always `true`)                        |
| Key params    | Issuer `Address`, Subject `Address`           |
| Written by    | `add_to_whitelist`, `bulk_add_to_whitelist`   |
| Deleted by    | `remove_from_whitelist`                       |
| Read by       | `is_whitelisted`                              |

Per-pair membership flag. Presence means the subject is on the issuer's
whitelist.

**Rust type:**
```rust
bool   // always true
```

---

#### 33. `ClaimType(String)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `set_claim_type`        |
| Value type    | `ClaimTypeInfo`                               |
| Key param     | Claim type identifier string                  |
| Written by    | `register_claim_type`                         |
| Read by       | `get_claim_type_description`, `Validation::require_registered_claim_type` |

Registered claim type descriptor. Re-registering an existing type overwrites
the description in place without duplicating the `ClaimTypeList` entry.

**Rust type:**
```rust
pub struct ClaimTypeInfo {
    pub claim_type:  String,
    pub description: String,
}
```

---

#### 34. `ClaimTypeList`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed whenever a new claim type is first registered |
| Value type    | `Vec<String>`                                 |
| Written by    | `register_claim_type` (only on first registration of a type) |
| Read by       | `list_claim_types`                            |

Global insertion-ordered list of all registered claim type identifier strings.

**Rust type:**
```rust
Vec<String>
```

---

#### 35. `ClaimTypeConstraints(String)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `set_claim_type_constraints` |
| Value type    | `ClaimTypeConstraints`                        |
| Key param     | Claim type identifier string                  |
| Written by    | `set_claim_type_constraints`                  |
| Read by       | `get_claim_type_constraints`, `Validation::validate_claim_type_constraints` |

Optional validation rules applied to new attestations of this claim type
(e.g. required metadata fields, allowed subjects).

**Rust type:**
```rust
pub struct ClaimTypeConstraints {
    pub required_metadata_fields: Vec<String>,
    pub max_duration_days:        Option<u32>,
    pub allowed_issuers:          Option<Vec<Address>>,
}
```

---

#### 36. `ClaimTypeCount(String)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `increment_claim_type_count` / `decrement_claim_type_count` |
| Value type    | `u64`                                         |
| Key param     | Claim type identifier string                  |
| Written by    | `increment_claim_type_count`, `decrement_claim_type_count` (internal) |
| Read by       | `get_claim_type_count`                        |

Running total of live (non-revoked) attestations of this claim type across all
issuers. Incremented on creation, decremented on revocation.

**Rust type:**
```rust
u64
```

---

#### 37. `LastIssuanceTime(Address)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `set_last_issuance_time` |
| Value type    | `u64`                                         |
| Key param     | Issuer `Address`                              |
| Written by    | `set_last_issuance_time` (internal, called on every new attestation) |
| Read by       | `get_last_issuance_time`, rate-limit enforcement logic |

Timestamp (seconds) of the issuer's most recent attestation creation, across
all claim types. Used to enforce the global per-issuer rate limit.

**Rust type:**
```rust
u64   // ledger timestamp in seconds
```

---

#### 38. `ClaimTypeIssuanceKey { issuer, claim_type }` *(composite key)*

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `set_last_issuance_time_by_claim_type` |
| Value type    | `u64`                                         |
| Key params    | Issuer `Address`, claim type `String`         |
| Written by    | `set_last_issuance_time_by_claim_type` (internal) |
| Read by       | `get_last_issuance_time_by_claim_type`, per-claim-type rate-limit enforcement |

This key is **not** a `StorageKey` enum variant. It is a separate
`#[contracttype]` struct used to avoid consuming an enum slot. It stores the
timestamp of the issuer's last attestation creation for a specific claim type,
enabling per-claim-type rate limiting that is independent of the global
`LastIssuanceTime` value.

**Rust type:**
```rust
#[contracttype]
pub struct ClaimTypeIssuanceKey {
    pub issuer:      Address,
    pub claim_type:  String,
}
// stored value: u64 (timestamp in seconds)
```

---

#### 39. `MultiSigProposal(String)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `set_multisig_proposal` |
| Value type    | `MultiSigProposal`                            |
| Key param     | Proposal ID string                            |
| Written by    | `set_multisig_proposal`                       |
| Read by       | `get_multisig_proposal`                       |

A multi-signature proposal record. The proposal accumulates cosigner approvals
until the threshold is reached, at which point the proposed action is executed.

**Rust type:**
```rust
pub struct MultiSigProposal {
    pub id:         String,
    pub action:     ProposalAction,
    pub signers:    Vec<Address>,
    pub threshold:  u32,
    pub approvals:  Vec<Address>,
    pub expires_at: u64,
    pub executed:   bool,
}
```

---

#### 40. `CouncilProposal(u32)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `set_proposal`          |
| Value type    | `CouncilProposal`                             |
| Key param     | Proposal ID `u32` (from `ProposalCounter`)    |
| Written by    | `set_proposal`                                |
| Read by       | `get_proposal`                                |

An admin council governance proposal. Council members vote on the proposal; if
quorum is reached before the timelock delay expires, the action is executed.

**Rust type:**
```rust
pub struct CouncilProposal {
    pub id:         u32,
    pub action:     CouncilAction,
    pub votes:      Vec<Address>,
    pub quorum:     u32,
    pub created_at: u64,
    pub executed:   bool,
}
```

---

#### 41. `ProposalIndex(Address)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `add_to_proposal_index` |
| Value type    | `Vec<String>`                                 |
| Key param     | Subject `Address`                             |
| Written by    | `add_to_proposal_index` (internal)            |
| Read by       | `get_proposal_index`, `list_open_proposals`   |

Ordered list of proposal IDs associated with a given subject address. Used to
support `list_open_proposals` without a full scan of all `CouncilProposal`
entries.

**Rust type:**
```rust
Vec<String>
```

---

#### 42. `AttestationRequest(String)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `set_attestation_request` |
| Value type    | `AttestationRequest`                          |
| Key param     | Request ID string                             |
| Written by    | `set_attestation_request`, `set_request`      |
| Read by       | `get_attestation_request`, `get_request`      |

A pending request submitted by a subject asking an issuer to issue a specific
attestation. Expires at `expires_at`; stale entries are cleaned up by
`cleanup_expired_requests`.

**Rust type:**
```rust
pub struct AttestationRequest {
    pub id:         String,
    pub issuer:     Address,
    pub subject:    Address,
    pub claim_type: String,
    pub expires_at: u64,
    pub metadata:   Option<String>,
}
```

---

#### 43. `IssuerPendingRequests(Address)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on add/remove operations   |
| Value type    | `Vec<String>`                                 |
| Key param     | Issuer `Address`                              |
| Written by    | `add_issuer_pending_request`, `remove_issuer_pending_request`, `cleanup_expired_requests` |
| Read by       | `get_issuer_pending_requests`, `get_pending_request_ids` |

Ordered list of open request IDs pending action by this issuer.

**Rust type:**
```rust
Vec<String>
```

---

#### 44. `AttestationBundle(String)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `set_bundle`            |
| Value type    | `AttestationBundle`                           |
| Key param     | Bundle ID string                              |
| Written by    | `set_bundle`                                  |
| Read by       | `get_bundle`, `has_bundle`                    |

Metadata record for an attestation bundle — a named collection of attestation
IDs issued together as a logical unit.

**Rust type:**
```rust
pub struct AttestationBundle {
    pub id:              String,
    pub issuer:          Address,
    pub subject:         Address,
    pub attestation_ids: Vec<String>,
    pub timestamp:       u64,
    pub metadata:        Option<String>,
}
```

---

#### 45. `IssuerBundles(Address)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `add_issuer_bundle`     |
| Value type    | `Vec<String>`                                 |
| Key param     | Issuer `Address`                              |
| Written by    | `add_issuer_bundle`                           |
| Read by       | `get_issuer_bundles`                          |

Ordered list of bundle IDs created by this issuer.

**Rust type:**
```rust
Vec<String>
```

---

#### 46. `SubjectBundles(Address)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `add_subject_bundle`    |
| Value type    | `Vec<String>`                                 |
| Key param     | Subject `Address`                             |
| Written by    | `add_subject_bundle`                          |
| Read by       | `get_subject_bundles`                         |

Ordered list of bundle IDs issued to this subject.

**Rust type:**
```rust
Vec<String>
```

---

#### 47. `AttestationTemplate(Address, String)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `set_template`          |
| Value type    | `AttestationTemplate`                         |
| Key params    | Issuer `Address`, template ID `String`        |
| Written by    | `set_template`                                |
| Read by       | `get_template`                                |

A reusable attestation template owned by an issuer. Templates pre-populate
claim type, default metadata, and optional validity duration so issuers can
create consistent attestations without repeating field values.

**Rust type:**
```rust
pub struct AttestationTemplate {
    pub template_id:      String,
    pub claim_type:       String,
    pub default_metadata: Option<String>,
    pub duration_days:    Option<u32>,
}
```

---

#### 48. `AttestationTemplateList(Address)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `add_to_template_registry` |
| Value type    | `Vec<String>`                                 |
| Key param     | Issuer `Address`                              |
| Written by    | `add_to_template_registry`                    |
| Read by       | `get_template_registry`                       |

Ordered list of template IDs registered by this issuer.

**Rust type:**
```rust
Vec<String>
```

---

#### 49. `Delegation(Address, Address, String)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `set_delegation`        |
| Value type    | `Delegation`                                  |
| Key params    | Delegator `Address`, Delegate `Address`, Claim type `String` |
| Written by    | `set_delegation`                              |
| Deleted by    | `remove_delegation`                           |
| Read by       | `get_delegation`                              |

A grant from a delegator allowing a delegate to issue attestations of a
specific claim type on the delegator's behalf.

**Rust type:**
```rust
pub struct Delegation {
    pub delegator:   Address,
    pub delegate:    Address,
    pub claim_type:  String,
    pub expires_at:  Option<u64>,
}
```

---

#### 50. `DelegatorIndex(Address)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `set_delegation`        |
| Value type    | `Vec<(Address, String)>`                      |
| Key param     | Delegator `Address`                           |
| Written by    | `set_delegation`                              |
| Read by       | `get_delegator_index`                         |

Ordered list of `(delegate, claim_type)` pairs representing all active
delegations granted by this delegator. Maintained as an index to support
listing a delegator's grants without a full-scan.

**Rust type:**
```rust
Vec<(Address, String)>
```

---

#### 51. `Dispute(String)`

| Property      | Value                                         |
|---------------|-----------------------------------------------|
| Tier          | Persistent                                    |
| TTL           | Per-key, refreshed on `set_dispute`           |
| Value type    | `DisputeRecord`                               |
| Key param     | Attestation ID string                         |
| Written by    | `set_dispute`                                 |
| Deleted by    | `remove_dispute`                              |
| Read by       | `get_dispute`                                 |

Active dispute record filed against an attestation. Absent when no dispute is
open for that attestation.

**Rust type:**
```rust
pub struct DisputeRecord {
    pub attestation_id: String,
    pub disputant:      Address,
    pub reason:         String,
    pub filed_at:       u64,
    pub resolved:       bool,
}
```

---

## Summary table

| # | Key | Tier | Value type | Key params |
|---|-----|------|------------|------------|
| 1 | `AdminCouncil` | Instance | `Vec<Address>` | — |
| 2 | `Version` | Instance | `String` | — |
| 3 | `FeeConfig` | Instance | `FeeConfig` | — |
| 4 | `TtlConfig` | Instance | `TtlConfig` | — |
| 5 | `ContractConfig` | Instance | `ContractConfig` | — |
| 6 | `GlobalStats` | Instance | `GlobalStats` | — |
| 7 | `Paused` | Instance | `bool` | — |
| 8 | `ClaimTypeRateLimit(String)` | Instance | `u64` | claim type |
| 9 | `ProposalCounter` | Instance | `u32` | — |
| 10 | `PendingAdminTransfer` | Instance | `PendingAdminTransfer` | — |
| 11 | `MiscConfig` | Instance | `MiscConfig` | — |
| 12 | `Issuer(Address)` | Persistent | `bool` | issuer |
| 13 | `IssuerList` | Persistent | `Vec<Address>` | — |
| 14 | `IssuerMetadata(Address)` | Persistent | `IssuerMetadata` | issuer |
| 15 | `IssuerStats(Address)` | Persistent | `IssuerStats` | issuer |
| 16 | `IssuerTier(Address)` | Persistent | `IssuerTier` | issuer |
| 17 | `IssuerRevocations(Address)` | Persistent | `u64` | issuer |
| 18 | `Bridge(Address)` | Persistent | `bool` | bridge contract |
| 19 | `BridgeList` | Persistent | `Vec<Address>` | — |
| 20 | `Attestation(String)` | Persistent | `Attestation` | attestation ID |
| 21 | `AttestationHistory(String)` | Persistent | `Vec<AttestationVersionSnapshot>` | attestation ID |
| 22 | `SubjectAttestations(Address)` | Persistent | `Vec<String>` | subject |
| 23 | `SubjectAttestationChunk(Address, u32)` | Persistent | `Vec<String>` | subject, chunk index |
| 24 | `IssuerAttestations(Address)` | Persistent | `Vec<String>` | issuer |
| 25 | `IssuerAttestationChunk(Address, u32)` | Persistent | `Vec<String>` | issuer, chunk index |
| 26 | `ValidAttestations(Address)` | Persistent | `Vec<String>` | subject |
| 27 | `AuditLog(String)` | Persistent | `Vec<AuditEntry>` | attestation ID |
| 28 | `Endorsements(String)` | Persistent | `Vec<Endorsement>` | attestation ID |
| 29 | `EndorserIndex(Address)` | Persistent | `Vec<Endorsement>` | endorser |
| 30 | `ExpirationHook(Address)` | Persistent | `ExpirationHook` | subject |
| 31 | `IssuerWhitelistMode(Address)` | Persistent | `bool` | issuer |
| 32 | `IssuerWhitelist(Address, Address)` | Persistent | `bool` | issuer, subject |
| 33 | `ClaimType(String)` | Persistent | `ClaimTypeInfo` | claim type |
| 34 | `ClaimTypeList` | Persistent | `Vec<String>` | — |
| 35 | `ClaimTypeConstraints(String)` | Persistent | `ClaimTypeConstraints` | claim type |
| 36 | `ClaimTypeCount(String)` | Persistent | `u64` | claim type |
| 37 | `LastIssuanceTime(Address)` | Persistent | `u64` | issuer |
| 38 | `ClaimTypeIssuanceKey` *(composite)* | Persistent | `u64` | issuer + claim type |
| 39 | `MultiSigProposal(String)` | Persistent | `MultiSigProposal` | proposal ID |
| 40 | `CouncilProposal(u32)` | Persistent | `CouncilProposal` | proposal ID |
| 41 | `ProposalIndex(Address)` | Persistent | `Vec<String>` | subject |
| 42 | `AttestationRequest(String)` | Persistent | `AttestationRequest` | request ID |
| 43 | `IssuerPendingRequests(Address)` | Persistent | `Vec<String>` | issuer |
| 44 | `AttestationBundle(String)` | Persistent | `AttestationBundle` | bundle ID |
| 45 | `IssuerBundles(Address)` | Persistent | `Vec<String>` | issuer |
| 46 | `SubjectBundles(Address)` | Persistent | `Vec<String>` | subject |
| 47 | `AttestationTemplate(Address, String)` | Persistent | `AttestationTemplate` | issuer, template ID |
| 48 | `AttestationTemplateList(Address)` | Persistent | `Vec<String>` | issuer |
| 49 | `Delegation(Address, Address, String)` | Persistent | `Delegation` | delegator, delegate, claim type |
| 50 | `DelegatorIndex(Address)` | Persistent | `Vec<(Address, String)>` | delegator |
| 51 | `Dispute(String)` | Persistent | `DisputeRecord` | attestation ID |

---

## TTL Extension Triggers

Every `extend_ttl` call in TrustLink is made inside a storage write helper in
`src/storage.rs`. There are no read-path TTL extensions in the current
implementation — a key's TTL is only refreshed when that key is written.

### TTL window

The extension target is determined at call time by `get_ttl_lifetime()`:

```rust
fn get_ttl_lifetime(env: &Env) -> u32 {
    if let Some(config) = env.storage().instance().get::<StorageKey, TtlConfig>(&StorageKey::TtlConfig) {
        DAY_IN_LEDGERS * config.ttl_days   // operator-configured value
    } else {
        DEFAULT_INSTANCE_LIFETIME          // 30 × 17 280 = 518 400 ledgers
    }
}
```

Both the `min_ledgers_to_live` and `extend_to` arguments passed to
`extend_ttl` are set to this same value, so every write unconditionally resets
the TTL to the full window regardless of how much time remains.

### `MIN_TTL_THRESHOLD` / `MIN_TTL_THRESHOLD_LEDGERS`

Two constants define a 7-day threshold (120 960 ledgers):

| Constant | Defined in | Value |
|---|---|---|
| `MIN_TTL_THRESHOLD` | `src/constants.rs` | `7 × DAY_IN_LEDGERS = 120 960` |
| `MIN_TTL_THRESHOLD_LEDGERS` | `src/types.rs` | `7 × DAY_IN_LEDGERS = 120 960` |

These constants are **reserved for a future lazy-extend pattern** — a
read-path guard that would call `extend_ttl` only when the remaining TTL drops
below the threshold, avoiding unnecessary ledger writes on every read. Neither
constant is wired into any live code path today; all TTL extensions are
currently triggered exclusively by writes.

### Instance storage triggers

Instance storage holds a single shared TTL for all instance keys. Any of the
following writes refreshes the entire instance TTL to the current TTL window:

| Contract function | Storage write helper | Keys covered |
|---|---|---|
| `initialize` | `set_admin_council` | `AdminCouncil`, `Version` |
| `initialize` / `set_fee` | `set_fee_config` | `FeeConfig` |
| `initialize` / `set_ttl_config` | `set_ttl_config` | `TtlConfig` |
| `initialize` / admin config calls | `set_contract_config` | `ContractConfig` |
| `pause` / `unpause` | `set_paused` | `Paused` |
| Any admin-council mutation | `set_admin_council` | `AdminCouncil` (and all other instance keys) |
| `set_global_stats` (internal) | `set_global_stats` | `GlobalStats` |
| `set_rate_limit`, `set_limits`, `set_decay_config`, `set_multisig_ttl`, `set_council_timelock_delay` | `set_misc_config` | `MiscConfig` |
| `propose_admin_transfer` | `set_pending_admin_transfer` | `PendingAdminTransfer` |
| `set_rate_limit_for_claim_type` | direct instance write | `ClaimTypeRateLimit(claim_type)` |
| `next_proposal_id` | direct instance write | `ProposalCounter` |

> Because all instance keys share one TTL entry, writing **any** instance key
> refreshes the TTL for **all** of them simultaneously.

### Persistent storage triggers

Each persistent key has its own independent TTL. The table below lists every
contract function that causes a persistent `extend_ttl` call and which key(s)
it refreshes.

| Contract function | Storage write helper | Key(s) refreshed |
|---|---|---|
| `register_issuer` | `add_issuer` | `Issuer(issuer)`, `IssuerList` |
| `remove_issuer` | `remove_issuer` | *(key deleted — no TTL extension)* |
| `register_bridge` | `add_bridge` | `Bridge(bridge)`, `BridgeList` |
| `create_attestation` | `set_attestation` | `Attestation(id)` |
| | `add_subject_attestation` / `ChunkedIndex::add_subject` | `SubjectAttestations(subject)` / `SubjectAttestationChunk(subject, n)` |
| | `add_issuer_attestation` / `ChunkedIndex::add_issuer` | `IssuerAttestations(issuer)` / `IssuerAttestationChunk(issuer, n)` |
| | `add_valid_attestation` | `ValidAttestations(subject)` |
| | `increment_claim_type_count` | `ClaimTypeCount(claim_type)` |
| | `set_last_issuance_time` | `LastIssuanceTime(issuer)` |
| | `set_last_issuance_time_by_claim_type` | `ClaimTypeIssuanceKey{issuer, claim_type}` |
| | `increment_issuer_stats` | `IssuerStats(issuer)` |
| | `increment_total_attestations` | `GlobalStats` (instance) |
| | `append_audit_entry` | `AuditLog(id)` |
| `import_attestation` | same helpers as `create_attestation` | same keys as above |
| `bridge_attestation` | same helpers as `create_attestation` | same keys as above |
| `create_attestations_batch` | `set_attestation` × N, `ChunkedIndex::add_issuer_bulk` | `Attestation(id)` × N, `IssuerAttestationChunk(issuer, n)` |
| `revoke_attestation` | `set_attestation` | `Attestation(id)` |
| | `remove_valid_attestation` | `ValidAttestations(subject)` |
| | `decrement_claim_type_count` | `ClaimTypeCount(claim_type)` |
| | `increment_issuer_revocations` | `IssuerRevocations(issuer)` |
| | `increment_total_revocations` | `GlobalStats` (instance) |
| | `append_audit_entry` | `AuditLog(id)` |
| | `push_attestation_version` | `AttestationHistory(id)` |
| `revoke_attestations_batch` | same per-attestation helpers as above × N | same keys × N |
| `renew_attestation` / `update_expiration` | `set_attestation` | `Attestation(id)` |
| | `push_attestation_version` | `AttestationHistory(id)` |
| | `append_audit_entry` | `AuditLog(id)` |
| `transfer_attestation` | `set_attestation` | `Attestation(id)` |
| | `ChunkedIndex::remove_issuer` / `add_issuer` | `IssuerAttestationChunk(old_issuer, n)`, `IssuerAttestationChunk(new_issuer, n)` |
| | `append_audit_entry` | `AuditLog(id)` |
| `cosign_attestation` (on threshold) | `set_attestation`, `add_subject_attestation`, `add_issuer_attestation` | `Attestation(id)`, `SubjectAttestations(subject)`, `IssuerAttestations(issuer)` |
| `set_issuer_metadata` | `set_issuer_metadata` | `IssuerMetadata(issuer)` |
| `set_issuer_tier` | `set_issuer_tier` | `IssuerTier(issuer)` |
| `register_claim_type` | `set_claim_type` | `ClaimType(claim_type)`, `ClaimTypeList` *(list only on first registration)* |
| `set_claim_type_constraints` | `set_claim_type_constraints` | `ClaimTypeConstraints(claim_type)` |
| `set_whitelist_mode` / `enable_whitelist_mode` | `set_whitelist_mode` | `IssuerWhitelistMode(issuer)` |
| `add_to_whitelist` / `bulk_add_to_whitelist` | `add_to_whitelist` | `IssuerWhitelist(issuer, subject)` |
| `remove_from_whitelist` | `remove_from_whitelist` | *(key deleted — no TTL extension)* |
| `set_expiration_hook` | `set_expiration_hook` | `ExpirationHook(subject)` |
| `set_multisig_proposal` | `set_multisig_proposal` | `MultiSigProposal(proposal_id)` |
| `set_proposal` (council) | `set_proposal` | `CouncilProposal(id)` |
| `add_to_proposal_index` | `add_to_proposal_index` | `ProposalIndex(subject)` |
| `set_attestation_request` | `set_attestation_request` | `AttestationRequest(request_id)` |
| `add_issuer_pending_request` | `add_issuer_pending_request` | `IssuerPendingRequests(issuer)` |
| `set_bundle` | `set_bundle` | `AttestationBundle(bundle_id)` |
| `add_issuer_bundle` | `add_issuer_bundle` | `IssuerBundles(issuer)` |
| `add_subject_bundle` | `add_subject_bundle` | `SubjectBundles(subject)` |
| `set_template` | `set_template` | `AttestationTemplate(issuer, template_id)` |
| `add_to_template_registry` | `add_to_template_registry` | `AttestationTemplateList(issuer)` |
| `set_delegation` | `set_delegation` | `Delegation(delegator, delegate, claim_type)`, `DelegatorIndex(delegator)` |
| `set_dispute` | `set_dispute` | `Dispute(attestation_id)` |
| `set_last_issuance_time` | `set_last_issuance_time` | `LastIssuanceTime(issuer)` |
| `set_last_issuance_time_by_claim_type` | `set_last_issuance_time_by_claim_type` | `ClaimTypeIssuanceKey{issuer, claim_type}` |

### Implications for archival node operators

- **A key that is never written will be evicted** once its TTL reaches zero.
  Infrequently-updated keys (e.g. `IssuerMetadata`, `ClaimType`, `ClaimTypeConstraints`) are at higher
  risk of eviction on low-activity contracts.
- **Reads never extend TTLs.** Calling `get_attestation`, `has_valid_claim`, or
  any other read-only function does not reset any TTL counter.
- **Batch operations extend each key individually.** `create_attestations_batch`
  calls `set_attestation` once per attestation, so each `Attestation(id)` key
  gets its own fresh TTL.
- **Chunk keys are contiguous.** `SubjectAttestationChunk(address, 0)`,
  `...1`, `...2`, etc. are written and deleted as a unit by `ChunkedIndex`.
  The first missing chunk index signals end of list. Deleted trailing chunks
  leave no gap — scan from index 0 and stop at the first absent key.
- **The shared instance TTL is a single point of failure.** If no admin
  operation is performed for the full TTL window (default 30 days), all
  instance keys (`AdminCouncil`, `FeeConfig`, `TtlConfig`, `ContractConfig`,
  `GlobalStats`, `MiscConfig`, etc.) expire together. Operators should schedule
  a periodic no-op admin write (e.g. re-applying the current `TtlConfig`) to
  keep instance storage alive.

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

### Reading instance storage (AdminCouncil / Version)

Instance storage keys use `ContractDataDurability.instance()` instead of
`persistent()`, and the key is a plain symbol with no parameters:

```js
const adminCouncilKey = xdr.LedgerKey.contractData(
  new xdr.LedgerKeyContractData({
    contract: new Contract(contractId).address().toScAddress(),
    key: xdr.ScVal.scvSymbol("AdminCouncil"),
    durability: xdr.ContractDataDurability.instance(),
  })
);
```

---

## Notes for indexer developers

- **Attestations are never deleted.** An attestation with `revoked: true` stays
  in storage indefinitely (subject to TTL). Index both active and revoked
  records if you need a complete history.
- **TTL eviction.** A key that is not touched for 30 days (or the configured
  `TtlConfig.ttl_days`) will be evicted. Indexers should snapshot state
  proactively rather than relying on keys always being present.
- **Subject and issuer indexes use chunked storage.** `SubjectAttestationChunk`
  and `IssuerAttestationChunk` replace the flat `SubjectAttestations` /
  `IssuerAttestations` keys once `ChunkedIndex` is active. Scan consecutive
  chunk indices (starting at 0) until a key is absent to reconstruct the full
  list.
- **`ValidAttestations(Address)` is a pre-filtered index.** It contains only
  currently-valid attestation IDs for a subject. This is faster for validity
  checks but incomplete for historical queries — use `SubjectAttestations` or
  the chunked equivalents for full history.
- **`ClaimTypeList` is insertion-ordered.** The order reflects the sequence in
  which `register_claim_type` was first called for each type.
- **Status is computed, not stored.** `AttestationStatus` (`Valid`, `Expired`,
  `Revoked`, `Pending`) is derived at query time from the stored fields and the
  current ledger timestamp. Indexers must replicate this logic locally.
- **`MiscConfig` bundles several logical config values.** Rate limit config,
  decay config, storage limits, multisig TTL, and council timelock delay are all
  packed into the single `MiscConfig` instance key. Reading one field requires
  deserializing the full struct.
- **`ClaimTypeIssuanceKey` is a composite `#[contracttype]` struct, not a
  `StorageKey` variant.** Its on-chain XDR discriminant differs from the
  `StorageKey` enum's discriminants. Use `ContractDataDurability.persistent()`
  and encode the struct directly when fetching via RPC.
- **`AuditLog`, `AttestationHistory`, and `Endorsements` share the attestation
  ID as their key parameter.** These three keys grow monotonically as an
  attestation is acted on; their size is proportional to attestation age and
  activity.
- **`GlobalStats` is a singleton instance key.** It tracks totals across all
  issuers and subjects. The values are eventually consistent within a transaction
  (incremented by helpers called during mutations) but can be read at any time
  via `get_global_stats`.

---

## Storage migration guide

This section explains how Soroban handles storage across contract upgrades and
how to safely evolve the TrustLink storage schema.

### How Soroban handles storage across upgrades

When a new WASM is uploaded and installed via `stellar contract upload` followed
by `stellar contract upgrade`, Soroban replaces the contract's executable code
atomically. **All storage is preserved exactly as-is** — no keys are touched,
no values are rewritten. The new WASM starts reading the same raw XDR bytes
that the old WASM wrote.

This means:

- Adding a new storage key is always safe — the key simply doesn't exist yet.
- Removing a storage key from the code is safe — the old bytes remain on-chain
  until TTL eviction, but the new code ignores them.
- **Changing the shape of an existing value type is a breaking change.** If the
  new WASM tries to deserialize a stored `ScVal` into a struct with a different
  field layout, deserialization will fail at runtime.

A dedicated admin-only migration function (called once by the admin immediately
after the upgrade) is the standard pattern for rewriting stored values into the
new format.

---

### Stable vs. potentially changing keys

**Stable** — these keys hold simple scalar values or flat lists. Their shape is
unlikely to change across versions:

| Key | Reason stable |
|---|---|
| `AdminCouncil` | `Vec<Address>` — no fields to add |
| `Version` | Single `String` — updated in place |
| `Issuer(Address)` | `bool` flag — no fields to add |
| `IssuerList` | `Vec<Address>` — append-only, no struct fields |
| `Bridge(Address)` | `bool` flag — no fields to add |
| `BridgeList` | `Vec<Address>` — append-only, no struct fields |
| `SubjectAttestations(Address)` | `Vec<String>` — append-only, no struct fields |
| `IssuerAttestations(Address)` | `Vec<String>` — append-only, no struct fields |
| `SubjectAttestationChunk(Address, u32)` | `Vec<String>` — chunk of IDs |
| `IssuerAttestationChunk(Address, u32)` | `Vec<String>` — chunk of IDs |
| `ValidAttestations(Address)` | `Vec<String>` — append-only index |
| `ClaimTypeList` | `Vec<String>` — append-only, no struct fields |
| `IssuerBundles(Address)` | `Vec<String>` — append-only index |
| `SubjectBundles(Address)` | `Vec<String>` — append-only index |
| `AttestationTemplateList(Address)` | `Vec<String>` — append-only index |
| `IssuerPendingRequests(Address)` | `Vec<String>` — mutable list |
| `ProposalIndex(Address)` | `Vec<String>` — append-only index |
| `Paused` | Single `bool` — no fields to add |
| `ProposalCounter` | Single `u32` — no fields to add |
| `ClaimTypeCount(String)` | Single `u64` — no fields to add |
| `LastIssuanceTime(Address)` | Single `u64` — no fields to add |
| `ClaimTypeIssuanceKey` *(composite)* | Single `u64` — no fields to add |
| `IssuerRevocations(Address)` | Single `u64` — no fields to add |
| `IssuerWhitelistMode(Address)` | Single `bool` — no fields to add |
| `IssuerWhitelist(Address, Address)` | `bool` flag — no fields to add |

**May change** — these keys hold structs with multiple fields. New fields may
be added in future versions:

| Key | Why it may change |
|---|---|
| `Attestation(String)` | Core data struct; new fields have already been added across versions |
| `FeeConfig` | Fee policy may gain new fields (e.g. per-claim-type fees) |
| `TtlConfig` | TTL strategy may gain new configuration axes |
| `ContractConfig` | Behavioural flags and limits will likely grow |
| `MiscConfig` | Consolidated settings bundle; new settings are added here first |
| `GlobalStats` | New global counters may be added |
| `IssuerMetadata(Address)` | Issuer profile may gain new fields |
| `IssuerStats(Address)` | New per-issuer counters may be added |
| `ClaimType(String)` | Claim type info may gain metadata fields |
| `ClaimTypeConstraints(String)` | Validation rules are likely to grow |
| `AttestationHistory(String)` | Snapshot struct may gain new fields |
| `AuditLog(String)` | Audit entry struct may gain new fields |
| `Endorsements(String)` | Endorsement struct may gain new fields |
| `EndorserIndex(Address)` | Same struct as `Endorsements` |
| `ExpirationHook(Address)` | Hook config may gain new parameters |
| `MultiSigProposal(String)` | Proposal struct may gain new fields |
| `CouncilProposal(u32)` | Council proposal struct may gain new fields |
| `AttestationRequest(String)` | Request struct may gain new fields |
| `AttestationBundle(String)` | Bundle struct may gain new fields |
| `AttestationTemplate(Address, String)` | Template struct may gain new fields |
| `Delegation(Address, Address, String)` | Delegation struct may gain new fields |
| `DelegatorIndex(Address)` | Element type may gain new fields |
| `Dispute(String)` | Dispute record struct may gain new fields |
| `PendingAdminTransfer` | Transfer record may gain new fields |

---

### Migration pattern for adding new fields to existing structs

The safest approach is an **opt-in default**: define the new field as
`Option<T>`, read existing records without a `migrate` call, and treat `None`
as the default value. This requires zero migration work and is backward
compatible.

Use a `migrate` function only when you need a non-optional field or must
rewrite every record eagerly.

#### Option 1 — Optional field (no migration needed)

Add the new field as `Option<T>` with a sensible default. Existing stored
records deserialize successfully because Soroban's XDR codec maps missing map
entries to `None` for `Option` fields.

```rust
// Before (v1)
pub struct Attestation {
    pub id:         String,
    pub issuer:     Address,
    pub claim_type: String,
    // ...
}

// After (v2) — backward compatible, no migrate() needed
pub struct Attestation {
    pub id:         String,
    pub issuer:     Address,
    pub claim_type: String,
    // ...
    pub audit_log:  Option<Vec<AuditEntry>>,  // None for all pre-v2 records
}
```

Call sites treat `None` as an empty audit log:

```rust
let log = attestation.audit_log.unwrap_or_default();
```

#### Option 2 — Eager migration with a `migrate` function

Use this when the new field must be non-optional or when you want to backfill
all existing records in one transaction.

```rust
pub fn migrate(env: Env, admin: Address) {
    admin.require_auth();
    Validation::require_admin(&env, &admin);

    // Iterate every known attestation ID and rewrite with the new default
    let ids: Vec<String> = /* load from an index or a migration manifest */;
    for id in ids.iter() {
        let mut att: AttestationV1 = storage::get_attestation(&env, &id);
        let att_v2 = AttestationV2 {
            id:        att.id,
            issuer:    att.issuer,
            claim_type: att.claim_type,
            // ... copy all existing fields ...
            new_field: DefaultValue,   // backfill
        };
        storage::set_attestation(&env, &att_v2);
    }
}
```

Call the migration function immediately after the upgrade in the same deployment window:

```bash
# 1. Upload the new WASM and capture the hash
NEW_HASH=$(stellar contract upload \
  --source "$ADMIN_SECRET" \
  --network mainnet \
  --wasm target/wasm32-unknown-unknown/release/trustlink.wasm)

# 2. Upgrade the contract executable
stellar contract upgrade \
  --id "$CONTRACT_ID" \
  --source "$ADMIN_SECRET" \
  --network mainnet \
  --wasm-hash "$NEW_HASH"

# 3. Run migration (admin only, call once)
stellar contract invoke --id "$CONTRACT_ID" --source "$ADMIN_SECRET" \
  --network mainnet -- migrate \
  --admin "$ADMIN_PUBLIC"
```

**Important:** `migrate` must be idempotent — safe to call more than once in
case of a partial failure. Guard against re-migration by checking a version
flag in instance storage:

```rust
pub fn migrate(env: Env, admin: Address) {
    admin.require_auth();
    Validation::require_admin(&env, &admin);

    let current: String = storage::get_version(&env);
    if current == "2.0.0" {
        return; // already migrated
    }

    // ... rewrite records ...

    storage::set_version(&env, &String::from_str(&env, "2.0.0"));
}
```

#### Choosing between the two options

| Situation | Recommended approach |
|---|---|
| New field has a sensible `None` / empty default | Option 1 — optional field |
| New field must be non-optional | Option 2 — migrate function |
| Renaming or removing a field | Option 2 — migrate function |
| Changing a field's type | Option 2 — migrate function; use a new key name to avoid XDR conflicts |

---

### Testing migrations

Always test the migration on testnet against a contract that has real stored
data before running on mainnet:

1. Deploy the current (pre-upgrade) version and create representative records.
2. Upgrade to the new WASM.
3. Call `migrate` (if applicable).
4. Run `./scripts/verify_deployment.sh` to confirm all read paths work.
5. Manually read a pre-existing record and confirm the new field has the
   expected default value.
