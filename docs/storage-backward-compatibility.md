# Storage Layout Backward-Compatibility Guide

## Overview

Contract upgrades must not break existing on-chain data. This guide documents how TrustLink ensures storage migrations are safe and provides a test strategy to catch breaking changes before merge.

## Storage Compatibility Guarantees

### Current Version: 1.0

The contract currently stores the following major entities:

```
Attestation(id) -> Attestation struct
SubjectAttestations(address) -> Vec<String> (attestation IDs)
IssuerAttestations(address) -> Vec<String> (attestation IDs)
Issuer(address) -> bool
Bridge(address) -> bool
Admin -> Address
FeeConfig -> FeeConfig
AdminCouncil -> Vec<Address>
```

### Safe Changes (Non-Breaking)

✅ **Adding new optional fields**
```rust
// Old
pub struct Attestation {
    id: String,
    issuer: Address,
}

// New (safe)
pub struct Attestation {
    id: String,
    issuer: Address,
    new_field: Option<String>,  // Defaults to None if missing
}
```

✅ **Adding new storage keys**
```rust
pub enum StorageKey {
    // ... existing keys ...
    NewFeature(String),  // New key, doesn't affect old data
}
```

✅ **Widening field types**
```rust
// Old
pub struct Config {
    limit: u32,
}

// New (safe)
pub struct Config {
    limit: u64,  // Can deserialize u32 as u64
}
```

### Unsafe Changes (Breaking)

❌ **Removing fields**
```rust
// Old
pub struct Attestation {
    id: String,
    issuer: Address,
    deprecated_field: String,  // ❌ Removing this breaks deserialization
}
```

❌ **Renaming storage keys** (if not migrated)
```rust
pub enum StorageKey {
    // ❌ Changing variant names breaks key lookup
    // OldKey(String),
    NewKey(String),  // This is a different key; old data is orphaned
}
```

❌ **Changing field types unsafely**
```rust
// Old
pub struct Config {
    enabled: bool,
}

// New (breaks deserialization)
pub struct Config {
    enabled: u8,  // ❌ Can't deserialize bool as u8
}
```

## Migration Pattern

When a breaking change is unavoidable:

### 1. Add a New Storage Key

```rust
pub enum StorageKey {
    AttestationV1(String),    // Old key, keep for reference
    AttestationV2(String),    // New key, safer structure
}
```

### 2. Implement Migration Function

```rust
pub fn migrate_attestation_v1_to_v2(env: &Env, id: &String) {
    if let Ok(old) = storage::get_attestation_v1(env, id) {
        let new = Attestation::from_legacy(old);
        storage::store_attestation_v2(env, &new);
        // Optionally delete old key to save space
        // storage::delete_attestation_v1(env, id);
    }
}
```

### 3. Call Migration in `initialize()`

```rust
pub fn initialize(env: Env, admin: Address, ttl_days: Option<u32>) -> Result<(), Error> {
    // ... existing init code ...
    
    // Migrate legacy storage if upgrading from previous version
    if Storage::has_admin(env) {
        migrate_storage_v1_to_v2(&env)?;
    }
    
    Ok(())
}
```

### 4. Update Tests

Add a test to verify migration works:

```rust
#[test]
fn test_migrate_attestation_v1_to_v2() {
    // Create v1 attestation
    // Call migration
    // Verify v2 attestation exists and is identical
}
```

## Backward-Compatibility Test Suite

### Location

```
tests/storage_layout_compatibility.rs
test_fixtures/storage_compat/
```

### Fixture Format

Store versioned snapshots of serialized storage:

```json
// test_fixtures/storage_compat/v0.1.0_snapshot.json
{
  "version": "0.1.0",
  "entries": [
    {
      "key_description": "Attestation(att_123)",
      "key_bytes": "...",
      "value_bytes": "..."
    },
    {
      "key_description": "SubjectAttestations(issuer_addr)",
      "key_bytes": "...",
      "value_bytes": "..."
    }
  ]
}
```

### Test Execution

```bash
# Run backward-compatibility tests
cargo test storage_layout_compatibility

# Verify no breaking changes are introduced
cargo test --test storage_layout_compatibility -- --nocapture
```

### CI Integration

Add to `.github/workflows/ci.yml`:

```yaml
- name: Storage Backward-Compatibility Check
  run: cargo test --test storage_layout_compatibility --release
```

## Checklist for Breaking Changes

Before introducing any storage structure change:

- [ ] Is the change truly necessary?
- [ ] Can it be made with `Option<T>` instead?
- [ ] Does migration path exist and is it tested?
- [ ] Are old keys preserved for at least one release?
- [ ] Is migration explicitly called during upgrade?
- [ ] Do all existing tests pass with new storage?
- [ ] Is new storage backward-compatible with old records?
- [ ] Does documentation explain the migration?

## Common Pitfalls

### Pitfall 1: Silent Data Orphaning

```rust
// ❌ Bad: Old data is lost
pub enum StorageKey {
    Attestation(String),    // Changed meaning without migration
}

// ✅ Good: Keep old key, add new one
pub enum StorageKey {
    AttestationV1(String),
    AttestationV2(String),
}
```

### Pitfall 2: Deserializing Old Records

```rust
// ❌ Bad: Doesn't check schema version
let attestation: Attestation = env.storage().get(key)?;

// ✅ Good: Handles both old and new formats
let attestation = match env.storage().get::<StorageKey, Attestation>(key) {
    Ok(att) => att,
    Err(_) => Attestation::default_for_missing_fields()
};
```

### Pitfall 3: Missing TTL on New Entries

```rust
// ✅ Always set TTL on new storage entries during migration
env.storage().instance().set(
    &key,
    &new_value,
);
env.storage().instance().bump(&key, DEFAULT_INSTANCE_LIFETIME);
```

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_attestation_deserializes_from_old_format() {
    // Create old Attestation bytes
    // Deserialize as new Attestation
    // Verify new fields have defaults
}
```

### Integration Tests

```rust
#[test]
fn test_contract_works_with_mixed_storage_versions() {
    // Create storage with v1 attestations
    // Upgrade to v2 code
    // Verify queries work on both versions
}
```

### Fixture-Based Tests

```rust
#[test]
fn test_load_mainnet_snapshot_and_deserialize() {
    // Load snapshot from past mainnet state
    // Verify current code can deserialize
    // Fail loudly if not
}
```

## References

- [Stellar Contract Upgrade Guide](https://developers.stellar.org/docs/learn/smart-contracts/upgrade-contract)
- [Soroban Storage Best Practices](https://developers.stellar.org/docs/learn/smart-contracts/storing-data)
- [Protocol Buffers Evolution Guide](https://developers.google.com/protocol-buffers/docs/overview#evolution)
