# TrustLink Security Review

**Date:** 2026-04-24
**Reviewer:** Pre-mainnet authorization audit
**Scope:** All public functions in `src/lib.rs` — `require_auth()` placement, state reads before auth, TOCTOU, admin/issuer check correctness.

---

## Methodology

Every public entry point was reviewed for:

1. `require_auth()` placement — must be the first meaningful call.
2. State reads before authorization — any storage read before auth can leak info or enable TOCTOU.
3. TOCTOU (time-of-check-time-of-use) — auth check and the guarded action must be atomic.
4. Admin check correctness — must compare against stored value, never trust the parameter alone.
5. Issuer check bypass — whether the issuer registry check can be circumvented.

---

## Findings

### FINDING-001 — `initialize`: State read before `require_auth` [MEDIUM]

**Location:** `src/lib.rs` — `initialize()` (line 275)

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

**Issue:** `Storage::has_admin()` is called before `admin.require_auth()`. An unauthenticated caller can probe whether the contract is initialized without providing a valid signature.

**Risk:** Low data sensitivity (boolean only), but violates "auth before state reads."

**Recommendation:** Move `require_auth()` to the first line.

**Status:** Open

---

### FINDING-002 — `revoke_attestation`: Missing `require_issuer` check [HIGH]

**Location:** `src/lib.rs` — `revoke_attestation()` (line 897)

**Issue:** Unlike `revoke_attestations_batch`, `revoke_attestation` does not call `Validation::require_issuer()`. A de-registered issuer can still revoke attestations they originally issued.

**Recommendation:** Add `Validation::require_issuer(&env, &issuer)?;` after `require_auth()`.

**Status:** Open

---

### FINDING-003 — `update_expiration`: Missing `require_issuer` check [HIGH]

**Location:** `src/lib.rs` — `update_expiration()` (line 1025)

**Issue:** `update_expiration` has no `Validation::require_issuer()` call, inconsistent with `renew_attestation`. A de-registered issuer can extend expiration on their attestations.

**Recommendation:** Add `Validation::require_issuer(&env, &issuer)?;` after `require_auth()`.

**Status:** Open

---

### FINDING-004 — `revoke_attestation` / `update_expiration`: State read before ownership check [LOW]

**Location:** `src/lib.rs` — `revoke_attestation()`, `update_expiration()`

**Issue:** `Storage::get_attestation()` is called before the `attestation.issuer != issuer` ownership check. Any authenticated caller can force a storage read for an arbitrary attestation ID.

**Status:** Accepted risk — mitigated once FINDING-002 and FINDING-003 are resolved.

---

### FINDING-005 — `initialize`: Auth on parameter, not stored value [INFO / BY DESIGN]

**Location:** `src/lib.rs` — `initialize()` (line 275)

**Issue:** During initialization there is no stored admin yet, so `require_auth()` is called on the `admin` parameter. This is the only correct pattern for a bootstrap function.

**Status:** Accepted — by design for bootstrap only.

---

### FINDING-006 — `get_admin` exposes admin address publicly [INFO]

**Location:** `src/lib.rs` — `get_admin()` (line 1427)

**Issue:** Admin address is publicly readable with no authentication. Standard on-chain transparency pattern.

**Status:** Accepted risk — standard transparency pattern.

---

### FINDING-007 — `cosign_attestation`: Proposal read before expiry/finalization checks [LOW]

**Location:** `src/lib.rs` — `cosign_attestation()` (line 1529)

**Issue:** Proposal is loaded from storage before checking `finalized` and `expires_at`. Any registered issuer can force a storage read on any proposal ID. Proposal data is not sensitive.

**Status:** Accepted risk — registry check provides adequate gating.

---

### FINDING-008 — Duplicate `pause`/`unpause`/`is_paused` definitions [MEDIUM]

**Location:** `src/lib.rs` — lines 545–568 and lines 1733–1751

**Issue:** `pause`, `unpause`, and `is_paused` are defined twice in the same `impl` block. The second definitions (lines 1733–1751) call `Events::contract_paused(&env, &admin)` without the `timestamp` argument, which will cause a compile error. The first definitions (lines 545–568) are correct. The duplicate definitions must be removed.

**Recommendation:** Remove the duplicate `pause`, `unpause`, and `is_paused` definitions at lines 1733–1751.

**Status:** Open

---

## Summary Table

| ID | Function | Severity | Issue | Status |
|----|----------|----------|-------|--------|
| FINDING-001 | `initialize` | Medium | State read (`has_admin`) before `require_auth` | Open |
| FINDING-002 | `revoke_attestation` | High | Missing `require_issuer` check | Open |
| FINDING-003 | `update_expiration` | High | Missing `require_issuer` check | Open |
| FINDING-004 | `revoke_attestation`, `update_expiration` | Low | Storage read before ownership check | Accepted |
| FINDING-005 | `initialize` | Info | Auth on parameter during bootstrap | Accepted |
| FINDING-006 | `get_admin` | Info | Admin address publicly readable | Accepted |
| FINDING-007 | `cosign_attestation` | Low | Proposal read before expiry/finalization checks | Accepted |
| FINDING-008 | `pause`, `unpause`, `is_paused` | Medium | Duplicate definitions with incorrect call signature | Open |

---

## Full Function Audit

### Admin / Initialization

| Function | Line | Auth Pattern | require_auth First? | State Read Before Auth? | Result |
|----------|------|-------------|---------------------|------------------------|--------|
| `initialize` | 275 | `require_auth` on param (bootstrap) | ❌ `has_admin` read first | Yes — `has_admin` | **FAIL** (FINDING-001) |
| `transfer_admin` | 299 | `require_auth` → `require_admin` (storage) | ✅ | No | **PASS** |
| `add_admin` | 313 | `require_auth` → `require_admin` (storage) | ✅ | No | **PASS** |
| `remove_admin` | 330 | `require_auth` → `require_admin` (storage) | ✅ | No | **PASS** |
| `get_admin` | 1427 | None (read-only, public) | N/A | N/A | **PASS** (info: public) |

### Issuer Management

| Function | Line | Auth Pattern | require_auth First? | State Read Before Auth? | Result |
|----------|------|-------------|---------------------|------------------------|--------|
| `register_issuer` | 350 | `require_auth` → `require_admin` | ✅ | No | **PASS** |
| `remove_issuer` | 359 | `require_auth` → `require_admin` | ✅ | No | **PASS** |
| `update_issuer_tier` | 418 | `require_auth` → `require_admin` → `require_issuer` | ✅ | No | **PASS** |
| `get_issuer_tier` | 433 | None (read-only) | N/A | N/A | **PASS** |
| `get_issuer_stats` | 1404 | None (read-only) | N/A | N/A | **PASS** |
| `set_issuer_metadata` | 1412 | `require_auth` → `require_issuer` | ✅ | No | **PASS** |
| `get_issuer_metadata` | 1423 | None (read-only) | N/A | N/A | **PASS** |
| `is_issuer` | 1400 | None (read-only) | N/A | N/A | **PASS** |

### Whitelist Management

| Function | Line | Auth Pattern | require_auth First? | State Read Before Auth? | Result |
|----------|------|-------------|---------------------|------------------------|--------|
| `set_whitelist_enabled` | 375 | `require_auth` → `require_issuer` | ✅ | No | **PASS** |
| `add_to_whitelist` | 386 | `require_auth` → `require_issuer` | ✅ | No | **PASS** |
| `remove_from_whitelist` | 397 | `require_auth` → `require_issuer` | ✅ | No | **PASS** |
| `is_whitelisted` | 405 | None (read-only) | N/A | N/A | **PASS** |
| `is_whitelist_enabled` | 410 | None (read-only) | N/A | N/A | **PASS** |

### Bridge Management

| Function | Line | Auth Pattern | require_auth First? | State Read Before Auth? | Result |
|----------|------|-------------|---------------------|------------------------|--------|
| `register_bridge` | 468 | `require_auth` → `require_admin` | ✅ | No | **PASS** |
| `is_bridge` | 1408 | None (read-only) | N/A | N/A | **PASS** |

### Fee & Rate Limit Configuration

| Function | Line | Auth Pattern | require_auth First? | State Read Before Auth? | Result |
|----------|------|-------------|---------------------|------------------------|--------|
| `set_fee` | 479 | `require_auth` → `require_admin` → `validate_fee_config` | ✅ | No | **PASS** |
| `get_fee_config` | 1431 | None (read-only) | N/A | N/A | **PASS** |
| `set_rate_limit` | 514 | `require_auth` → `require_admin` | ✅ | No | **PASS** |
| `get_rate_limit` | 533 | None (read-only) | N/A | N/A | **PASS** |

### Pause / Unpause

| Function | Line | Auth Pattern | require_auth First? | State Read Before Auth? | Result |
|----------|------|-------------|---------------------|------------------------|--------|
| `pause` (first) | 545 | `require_auth` → `require_admin` | ✅ | No | **PASS** |
| `unpause` (first) | 557 | `require_auth` → `require_admin` | ✅ | No | **PASS** |
| `is_paused` | 566 | None (read-only) | N/A | N/A | **PASS** |
| `pause` (duplicate) | 1733 | `require_auth` → `require_admin` | ✅ | No | **FAIL** (FINDING-008: duplicate, wrong call) |
| `unpause` (duplicate) | 1741 | `require_auth` → `require_admin` | ✅ | No | **FAIL** (FINDING-008: duplicate, wrong call) |

### Attestation Creation

| Function | Line | Auth Pattern | require_auth First? | State Read Before Auth? | Result |
|----------|------|-------------|---------------------|------------------------|--------|
| `create_attestation` | 660 | `require_auth` → `require_issuer` → validations | ✅ | No | **PASS** |
| `create_attestation_jurisdiction` | 681 | `require_auth` → `require_issuer` → validations | ✅ | No | **PASS** |
| `import_attestation` | 703 | `require_auth` → `require_admin` → `require_issuer` | ✅ | No | **PASS** |
| `bridge_attestation` | 759 | `require_auth` → `require_bridge` | ✅ | No | **PASS** |
| `create_attestations_batch` | 820 | `require_auth` → `require_issuer` | ✅ | No | **PASS** |

### Attestation Mutation

| Function | Line | Auth Pattern | require_auth First? | State Read Before Auth? | Result |
|----------|------|-------------|---------------------|------------------------|--------|
| `revoke_attestation` | 897 | `require_auth` only (no `require_issuer`) | ✅ | No | **FAIL** (FINDING-002) |
| `revoke_attestations_batch` | 942 | `require_auth` → `require_issuer` | ✅ | No | **PASS** |
| `renew_attestation` | 991 | `require_auth` → `require_issuer` | ✅ | No | **PASS** |
| `update_expiration` | 1025 | `require_auth` only (no `require_issuer`) | ✅ | No | **FAIL** (FINDING-003) |
| `request_deletion` | 1194 | `require_auth` → ownership check on loaded attestation | ✅ | Yes — `get_attestation` after auth | **PASS** (auth first, read after) |
| `transfer_attestation` | 1796 | `require_auth` → `require_admin` → `require_issuer` (new_issuer) | ✅ | Yes — `get_attestation` after auth | **PASS** (auth first, read after) |

### Attestation Queries (Read-Only)

| Function | Line | Auth Pattern | Result |
|----------|------|-------------|--------|
| `has_valid_claim` | 1064 | None (read-only) | **PASS** |
| `has_valid_claim_from_issuer` | 1099 | None (read-only) | **PASS** |
| `has_valid_claim_from_tier` | 439 | None (read-only) | **PASS** |
| `has_any_claim` | 1128 | None (read-only) | **PASS** |
| `has_all_claims` | 1152 | None (read-only) | **PASS** |
| `get_attestation` | 1178 | None (read-only) | **PASS** |
| `get_attestation_status` | 1225 | None (read-only) | **PASS** |
| `get_attestation_by_type` | 1375 | None (read-only) | **PASS** |
| `get_audit_log` | 1221 | None (read-only) | **PASS** |
| `get_subject_attestations` | 1239 | None (read-only) | **PASS** |
| `get_attestations_in_range` | 1253 | None (read-only) | **PASS** |
| `get_attestations_by_tag` | 1287 | None (read-only) | **PASS** |
| `get_attestations_by_jurisdiction` | 1310 | None (read-only) | **PASS** |
| `get_issuer_attestations` | 1334 | None (read-only) | **PASS** |
| `get_valid_claims` | 1348 | None (read-only) | **PASS** |

### Claim Type Registry

| Function | Line | Auth Pattern | require_auth First? | State Read Before Auth? | Result |
|----------|------|-------------|---------------------|------------------------|--------|
| `register_claim_type` | 1435 | `require_auth` → `require_admin` | ✅ | No | **PASS** |
| `get_claim_type_description` | 1454 | None (read-only) | N/A | N/A | **PASS** |
| `list_claim_types` | 1458 | None (read-only) | N/A | N/A | **PASS** |

### Multi-Sig

| Function | Line | Auth Pattern | require_auth First? | State Read Before Auth? | Result |
|----------|------|-------------|---------------------|------------------------|--------|
| `propose_attestation` | 1471 | `require_auth` → `require_issuer` | ✅ | No | **PASS** |
| `cosign_attestation` | 1529 | `require_auth` → `require_issuer` | ✅ | Yes — proposal read after auth | **PASS** (auth first; see FINDING-007) |
| `get_multisig_proposal` | 1612 | None (read-only) | N/A | N/A | **PASS** |

### Endorsements

| Function | Line | Auth Pattern | require_auth First? | State Read Before Auth? | Result |
|----------|------|-------------|---------------------|------------------------|--------|
| `endorse_attestation` | 1628 | `require_auth` → `require_issuer` | ✅ | Yes — `get_attestation` after auth | **PASS** (auth first, read after) |

### Storage Limits

| Function | Line | Auth Pattern | require_auth First? | State Read Before Auth? | Result |
|----------|------|-------------|---------------------|------------------------|--------|
| `set_limits` | 1679 | `require_auth` → `require_admin` | ✅ | No | **PASS** |
| `get_limits` | 1699 | None (read-only) | N/A | N/A | **PASS** |

### Contract Metadata / Config

| Function | Line | Auth Pattern | Result |
|----------|------|-------------|--------|
| `get_version` | 1707 | None (read-only) | **PASS** |
| `get_global_stats` | 1714 | None (read-only) | **PASS** |
| `health_check` | 1722 | None (read-only) | **PASS** |
| `get_contract_metadata` | 1753 | None (read-only) | **PASS** |
| `get_config` | 1765 | None (read-only) | **PASS** |

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

Four actionable findings remain open:

1. **FINDING-001** — Move `require_auth` before `has_admin` check in `initialize`.
2. **FINDING-002** — Add `Validation::require_issuer` to `revoke_attestation`.
3. **FINDING-003** — Add `Validation::require_issuer` to `update_expiration`.
4. **FINDING-008** — Remove duplicate `pause`/`unpause`/`is_paused` definitions (lines 1733–1751) that call `Events::contract_paused`/`contract_unpaused` with wrong arity.

Run the full test suite to confirm no regressions: `cargo test`
