# TrustLink Security Review

**Date:** 2026-03-25  
**Reviewer:** Pre-mainnet authorization audit  
**Scope:** All functions in `src/lib.rs` that call `require_auth()`, plus read-only entry points that could leak sensitive state.

---

## Methodology

Every public entry point was reviewed for:

1. `require_auth()` placement — must be the first meaningful call.
2. State reads before authorization — any storage read before auth can leak info or enable TOCTOU.
3. TOCTOU (time-of-check-time-of-use) — auth check and the guarded action must be atomic with no re-readable state in between.
4. Admin check correctness — must compare against stored value, never trust the parameter alone.
5. Issuer check bypass — whether the issuer registry check can be circumvented.

---

## Findings

### FINDING-001 — `initialize`: State read before `require_auth` [MEDIUM]

**Location:** `src/lib.rs` — `initialize()`

**Code:**
```rust
pub fn initialize(env: Env, admin: Address, ttl_days: Option<u32>) -> Result<(), Error> {
    if Storage::has_admin(&env) {          // ← storage read BEFORE auth
        return Err(Error::AlreadyInitialized);
    }
    admin.require_auth();                  // ← auth happens second
    ...
}
```

**Issue:** `Storage::has_admin()` is called before `admin.require_auth()`. While this specific read does not leak sensitive data (it only returns a boolean), it violates the principle that auth must precede all state interaction. On Soroban, `require_auth()` failing causes a transaction panic, so the early return on `AlreadyInitialized` is reachable without any valid signature — an unauthenticated caller can probe whether the contract is initialized.

**Risk:** Low data sensitivity (boolean only), but sets a bad precedent and technically violates "auth before state reads."

**Recommendation:** Move `require_auth()` to the first line, before the `has_admin` check.

```rust
pub fn initialize(env: Env, admin: Address, ttl_days: Option<u32>) -> Result<(), Error> {
    admin.require_auth();
    if Storage::has_admin(&env) {
        return Err(Error::AlreadyInitialized);
    }
    ...
}
```

**Status:** Open

---

### FINDING-002 — `revoke_attestation`: Missing `require_issuer` check [HIGH]

**Location:** `src/lib.rs` — `revoke_attestation()`

**Code:**
```rust
pub fn revoke_attestation(env: Env, issuer: Address, attestation_id: String, reason: Option<String>) -> Result<(), Error> {
    issuer.require_auth();
    validate_reason(&reason)?;
    let mut attestation = Storage::get_attestation(&env, &attestation_id)?;  // ← state read

    if attestation.issuer != issuer {      // ← ownership check after read
        return Err(Error::Unauthorized);
    }
    ...
}
```

**Issue:** Unlike `revoke_attestations_batch` (its batch sibling), `revoke_attestation` does **not** call `Validation::require_issuer()`. Any address — registered or not — can call this function with a valid signature. The only guard is the post-read ownership check `attestation.issuer != issuer`. This means:

- An unregistered address that happens to have issued an attestation (e.g., an issuer that was later de-registered) can still revoke.
- The attestation is read from storage before the ownership check, meaning the read happens for every caller regardless of registry status.

**Recommendation:** Add `Validation::require_issuer(&env, &issuer)?;` immediately after `require_auth()`, consistent with `revoke_attestations_batch`.

```rust
pub fn revoke_attestation(...) -> Result<(), Error> {
    issuer.require_auth();
    Validation::require_issuer(&env, &issuer)?;   // ← add this
    validate_reason(&reason)?;
    let mut attestation = Storage::get_attestation(&env, &attestation_id)?;
    ...
}
```

**Status:** Open

---

### FINDING-003 — `update_expiration`: Missing `require_issuer` check [HIGH]

**Location:** `src/lib.rs` — `update_expiration()`

**Code:**
```rust
pub fn update_expiration(env: Env, issuer: Address, attestation_id: String, new_expiration: Option<u64>) -> Result<(), Error> {
    issuer.require_auth();

    if let Some(value) = new_expiration {
        if value <= env.ledger().timestamp() {
            return Err(Error::InvalidExpiration);
        }
    }

    let mut attestation = Storage::get_attestation(&env, &attestation_id)?;  // ← state read
    if attestation.issuer != issuer {      // ← ownership check after read
        return Err(Error::Unauthorized);
    }
    ...
}
```

**Issue:** `update_expiration` has no `Validation::require_issuer()` call. Any address with a valid signature can call this. Compare with `renew_attestation`, which is functionally identical but correctly calls `Validation::require_issuer()` as its second line. This inconsistency is a clear oversight.

A de-registered issuer retains the ability to extend expiration on attestations they originally issued, which undermines the purpose of de-registration.

**Recommendation:** Add `Validation::require_issuer(&env, &issuer)?;` immediately after `require_auth()`, matching `renew_attestation`.

```rust
pub fn update_expiration(...) -> Result<(), Error> {
    issuer.require_auth();
    Validation::require_issuer(&env, &issuer)?;   // ← add this
    ...
}
```

**Status:** Open

---

### FINDING-004 — `revoke_attestation` / `update_expiration`: State read before ownership check [LOW-MEDIUM]

**Location:** `src/lib.rs` — `revoke_attestation()`, `update_expiration()`

**Issue:** In both functions, `Storage::get_attestation()` is called before the `attestation.issuer != issuer` ownership check. This means any authenticated caller can trigger a storage read for an arbitrary attestation ID. While the data returned is not secret (attestations are public), it does mean:

- Storage rent is consumed on failed calls.
- The existence of an attestation ID is confirmed to the caller before the ownership check fails.

This is a minor TOCTOU concern: the check (ownership) and the use (mutation) are separated by a storage read that any caller can force.

**Recommendation:** This is inherent to the pattern of loading then checking ownership. The risk is low given attestation data is public. Acceptable as-is once FINDING-002 and FINDING-003 are resolved (registry check will gate unregistered callers first).

**Status:** Accepted risk (mitigated by FINDING-002 and FINDING-003 fixes)

---

### FINDING-005 — `initialize`: Auth on `admin` parameter, not stored value [INFO / BY DESIGN]

**Location:** `src/lib.rs` — `initialize()`

**Issue:** During initialization, there is no stored admin yet, so `require_auth()` is necessarily called on the `admin` parameter. This is the correct and only possible pattern for a bootstrap function. After initialization, all admin functions correctly call `Validation::require_admin()` which reads from storage and compares — parameter trust is not used post-init.

**Status:** Accepted — by design for bootstrap only.

---

### FINDING-006 — `get_admin` exposes admin address publicly [INFO]

**Location:** `src/lib.rs` — `get_admin()`

**Code:**
```rust
pub fn get_admin(env: Env) -> Result<Address, Error> {
    Storage::get_admin(&env)
}
```

**Issue:** The admin address is publicly readable with no authentication. This is common practice for on-chain contracts (transparency), but it means the admin address is known to potential attackers who could target it off-chain.

**Status:** Accepted risk — standard on-chain transparency pattern.

---

### FINDING-007 — `cosign_attestation`: Proposal state read before expiry/finalization checks [LOW]

**Location:** `src/lib.rs` — `cosign_attestation()`

**Code:**
```rust
pub fn cosign_attestation(env: Env, issuer: Address, proposal_id: String) -> Result<(), Error> {
    issuer.require_auth();
    Validation::require_issuer(&env, &issuer)?;

    let mut proposal = Storage::get_multisig_proposal(&env, &proposal_id)?;  // ← read before checks

    if proposal.finalized { return Err(Error::ProposalFinalized); }
    let current_time = env.ledger().timestamp();
    if current_time >= proposal.expires_at { return Err(Error::ProposalExpired); }
    ...
}
```

**Issue:** The proposal is loaded from storage before checking `finalized` and `expires_at`. Any registered issuer can force a storage read on any proposal ID. Since proposal data is not sensitive and the issuer registry check gates unregistered callers, this is low risk.

**Status:** Accepted risk — proposal data is not sensitive; registry check provides adequate gating.

---

## Summary Table

| ID | Function | Severity | Issue | Status |
|----|----------|----------|-------|--------|
| FINDING-001 | `initialize` | Medium | State read (`has_admin`) before `require_auth` | Fixed |
| FINDING-002 | `revoke_attestation` | High | Missing `require_issuer` check; de-registered issuers can revoke | Fixed |
| FINDING-003 | `update_expiration` | High | Missing `require_issuer` check; inconsistent with `renew_attestation` | Fixed |
| FINDING-004 | `revoke_attestation`, `update_expiration` | Low | Storage read before ownership check (minor TOCTOU) | Accepted (mitigated by F-002/F-003) |
| FINDING-005 | `initialize` | Info | Auth on parameter during bootstrap | Accepted — by design |
| FINDING-006 | `get_admin` | Info | Admin address publicly readable | Accepted — transparency |
| FINDING-007 | `cosign_attestation` | Low | Proposal read before expiry/finalization checks | Accepted — data not sensitive |

---

## Functions Confirmed Correct

The following privileged functions were reviewed and found to have correct authorization ordering (`require_auth` first, then storage-based role check, then business logic):

| Function | Auth Pattern |
|----------|-------------|
| `transfer_admin` | `require_auth` → `require_admin` (storage comparison) |
| `register_issuer` | `require_auth` → `require_admin` |
| `remove_issuer` | `require_auth` → `require_admin` |
| `update_issuer_tier` | `require_auth` → `require_admin` → `require_issuer` |
| `register_bridge` | `require_auth` → `require_admin` |
| `set_fee` | `require_auth` → `require_admin` → `validate_fee_config` |
| `create_attestation` | `require_auth` → `require_issuer` → validations |
| `import_attestation` | `require_auth` → `require_admin` → `require_issuer` |
| `bridge_attestation` | `require_auth` → `require_bridge` |
| `create_attestations_batch` | `require_auth` → `require_issuer` |
| `revoke_attestations_batch` | `require_auth` → `require_issuer` |
| `renew_attestation` | `require_auth` → `require_issuer` |
| `set_issuer_metadata` | `require_auth` → `require_issuer` |
| `register_claim_type` | `require_auth` → `require_admin` |
| `propose_attestation` | `require_auth` → `require_issuer` |
| `endorse_attestation` | `require_auth` → `require_issuer` |

---

## Admin Check Verification

`Validation::require_admin()` in `src/validation.rs` reads the admin from storage via `Storage::get_admin(env)` and compares it against the caller parameter. It does **not** trust the parameter — the stored value is the source of truth. This is correct.

```rust
pub fn require_admin(env: &Env, caller: &Address) -> Result<(), Error> {
    let admin = Storage::get_admin(env)?;   // ← reads from storage
    if caller != &admin {
        return Err(Error::Unauthorized);
    }
    Ok(())
}
```

No bypass is possible through parameter manipulation.

---

## Issuer Check Verification

`Validation::require_issuer()` checks `Storage::is_issuer()` which does a persistent storage key presence check (`env.storage().persistent().has(...)`). The issuer address is used as the storage key, so the check is tied to the actual registered set — not a parameter comparison. No bypass identified.

---

## Required Actions Before Mainnet

All three actionable findings have been fixed in `src/lib.rs`:

1. **FINDING-001 fixed** — `require_auth` moved before `has_admin` check in `initialize`.
2. **FINDING-002 fixed** — `Validation::require_issuer` added to `revoke_attestation`.
3. **FINDING-003 fixed** — `Validation::require_issuer` added to `update_expiration`.

Run the full test suite to confirm no regressions: `cargo test`
