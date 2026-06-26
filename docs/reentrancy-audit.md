# Reentrancy Audit — TrustLink

## Scope

This document covers the reentrancy analysis for every external cross-contract
call site in TrustLink. An external call is any invocation of a client generated
from a foreign contract interface (`TokenClient`, `ExpirationCallbackClient`).

Three call sites were identified:

| # | Call | Location | Caller function(s) |
|---|------|----------|--------------------|
| 1 | `TokenClient::transfer` | `attestation.rs` → `charge_attestation_fee()` | `create_attestation_internal()` |
| 2 | `TokenClient::try_balance` | `admin.rs` → `validate_fee_config()` | `set_fee()` |
| 3 | `ExpirationCallbackClient::try_notify_expiring` | `attestation.rs` → `maybe_trigger_expiration_hook()` | `has_valid_claim()`, `has_valid_claim_from_issuer()` |

---

## Soroban Reentrancy Model

Soroban's WASM execution model does **not** provide automatic reentrancy locks.
A cross-contract call within a single transaction can re-enter the calling
contract before the original invocation returns. The
**Checks-Effects-Interactions (CEI)** pattern is therefore the primary defense:
all state mutations must be committed before any external call is made.

---

## Call Site 1 — `TokenClient::transfer` in `charge_attestation_fee()`

### Location

`src/attestation.rs` — `charge_attestation_fee()`, called at the end of
`create_attestation_internal()`.

### State read/written before the call

All state mutations happen in the EFFECTS phase, before `charge_attestation_fee`
is reached:

```
create_attestation_internal()
  CHECKS  — require_auth, require_not_paused, require_issuer,
             validate_claim_type, validate_metadata, validate_jurisdiction,
             validate_tags, validate_native_expiration, validate_valid_from,
             issuer != subject, whitelist check, check_rate_limit,
             limit checks, duplicate-ID guard (has_attestation)
  EFFECTS — store_attestation()        ← attestation record + indexes + stats
             append_audit_entry()      ← audit log entry
             set_last_issuance_time()  ← rate-limit timestamp
  INTERACTION — charge_attestation_fee() → TokenClient::transfer()
  EVENT   — Events::attestation_created()
```

By the time `TokenClient::transfer` is invoked, the following storage keys are
already written:

- `Attestation(id)` — full attestation record
- `SubjectAttestations(subject)` — subject index updated
- `IssuerAttestations(issuer)` — issuer index updated
- `IssuerStats(issuer).total_issued` — incremented
- `GlobalStats.total_attestations` — incremented
- `AuditLog(id)` — Created entry appended
- `LastIssuanceTime(issuer)` — timestamp recorded

### Is reentrancy possible?

A malicious token contract could call back into `create_attestation_internal`
during `transfer`. However, every re-entrant attempt is blocked:

- **Same inputs** → `has_attestation` returns `true` → `Error::DuplicateAttestation`.
- **Different `claim_type`** → a new ID is generated, but `check_rate_limit`
  rejects the call because `set_last_issuance_time` was already written in the
  EFFECTS phase of the outer call.
- **Different `subject`** → passes the duplicate and rate-limit guards, but the
  subject-count limit check may block it; and the outer call's fee transfer has
  not yet completed, so the issuer's token balance may be insufficient.

### Mitigation

**CEI ordering** is the primary mitigation. All state is committed before the
external call. The duplicate-ID guard and rate-limit timestamp provide
defense-in-depth.

### Verdict

✅ **No reentrancy risk.** CEI is correctly followed.

---

## Call Site 2 — `TokenClient::try_balance` in `validate_fee_config()`

### Location

`src/admin.rs` — `validate_fee_config()`, called from `set_fee()`.

```rust
fn validate_fee_config(env: &Env, fee: i128, fee_token: &Option<Address>) -> Result<(), Error> {
    // ...
    if let Some(token_addr) = fee_token {
        let token = TokenClient::new(env, token_addr);
        token
            .try_balance(&env.current_contract_address())
            .map_err(|_| Error::InvalidFeeToken)?;
    }
    Ok(())
}
```

### State read/written before the call

`validate_fee_config` is a pure validation helper. It is called **before**
`Storage::set_fee_config` writes the new fee configuration:

```
set_fee()
  CHECKS  — require_auth, require_admin, validate_fee_config (→ try_balance),
             admin != collector
  EFFECTS — Storage::set_fee_config()
```

No contract state is written before or after `try_balance` within
`validate_fee_config` itself.

### Is reentrancy possible?

`try_balance` is a **read-only** call — it queries the token contract's balance
for the TrustLink contract address. It does not transfer tokens or trigger any
callback. A malicious token contract could theoretically re-enter `set_fee`, but:

- `set_fee` requires admin authentication (`require_auth` + `require_admin`).
  A token contract cannot forge admin credentials.
- Even if re-entry occurred, `Storage::set_fee_config` has not yet been called,
  so the re-entrant call would overwrite nothing meaningful.

### Mitigation

The call is read-only and gated behind admin auth. No state mutation occurs
before or after the call within the validation helper.

### Verdict

✅ **No reentrancy risk.** Read-only probe behind admin auth; no state written
around the call.

---

## Call Site 3 — `ExpirationCallbackClient::try_notify_expiring` in `maybe_trigger_expiration_hook()`

### Location

`src/attestation.rs` — `maybe_trigger_expiration_hook()`, called from
`query::has_valid_claim()` and `query::has_valid_claim_from_issuer()`.

```rust
pub fn maybe_trigger_expiration_hook(
    env: &Env,
    subject: &Address,
    attestation_id: &String,
    expiration: u64,
    current_time: u64,
) {
    let hook = match Storage::get_expiration_hook(env, subject) {
        Some(h) => h,
        None => return,
    };
    let notify_window = (hook.notify_days_before as u64) * SECS_PER_DAY;
    let notify_from = expiration.saturating_sub(notify_window);
    if current_time >= notify_from && current_time < expiration {
        Events::expiration_hook_triggered(env, subject, attestation_id, expiration);
        let client = ExpirationCallbackClient::new(env, &hook.callback_contract);
        let _ = client.try_notify_expiring(subject, attestation_id, &expiration);
    }
}
```

### State read/written before the call

`has_valid_claim` and `has_valid_claim_from_issuer` are **read-only query
functions**. They write no contract state. `maybe_trigger_expiration_hook`
itself writes no state — it only emits an event and makes the external call.

```
has_valid_claim()
  READ    — Storage::get_subject_attestations, Storage::get_attestation
  (no state written)
  CALL    — maybe_trigger_expiration_hook() → try_notify_expiring()
  RETURN  — bool (no state written after the call)
```

### Is reentrancy possible?

A malicious callback contract could re-enter any TrustLink function during
`try_notify_expiring`. However:

- **Re-entering `has_valid_claim`** — harmless; it is read-only and returns a
  bool. No state is mutated.
- **Re-entering a mutating function** (e.g. `create_attestation`) — the
  callback contract is registered by the **subject** (`register_expiration_hook`
  requires subject auth). The subject is not an issuer, so `require_issuer`
  would reject any attestation creation attempt. Even if the subject were also
  an issuer, the re-entrant call would be a normal, independent invocation
  subject to all standard guards.
- **`try_` variant** — `try_notify_expiring` uses the non-panicking `try_`
  form. Any panic or error in the callback is silently swallowed; it cannot
  revert the outer call or corrupt TrustLink state.

### Mitigation

1. **No state written before or after the external call** in the query path.
2. **`try_` variant** — callback failures are isolated and cannot affect the
   caller.
3. **Subject-registered hook** — the callback address is controlled by the
   subject, not an arbitrary party; and subject auth is required to change it.

### Verdict

✅ **No reentrancy risk.** The call site is inside a read-only query function;
no state is written around the call, and the `try_` variant isolates failures.

---

## Summary

| Call site | Function | Reentrancy possible? | Mitigation |
|-----------|----------|----------------------|------------|
| `TokenClient::transfer` | `charge_attestation_fee()` → `create_attestation_internal()` | No | CEI: all state committed before call; duplicate-ID guard and rate-limit timestamp block re-entrant attempts |
| `TokenClient::try_balance` | `validate_fee_config()` → `set_fee()` | No | Read-only probe; admin auth required; no state written around the call |
| `ExpirationCallbackClient::try_notify_expiring` | `maybe_trigger_expiration_hook()` → `has_valid_claim()` / `has_valid_claim_from_issuer()` | No | Called from read-only query functions; no state written before or after; `try_` variant isolates failures |

### Overall Verdict

✅ **TrustLink has no reentrancy vulnerabilities.** All three external call sites
are safe: the fee transfer follows CEI, the balance probe is read-only, and the
expiration callback is called from a stateless query path using the non-panicking
`try_` variant.

---

## Appendix — CEI Invariants for `create_attestation_internal`

| Invariant | Guaranteed by |
|-----------|---------------|
| No duplicate attestation IDs | `has_attestation` check before `store_attestation` |
| `IssuerStats.total_issued` matches actual count | incremented inside `store_attestation` (EFFECTS) |
| Audit log entry exists for every stored attestation | `append_audit_entry` called in EFFECTS |
| Rate-limit timestamp always recorded before external call | `set_last_issuance_time` called in EFFECTS |
| Fee charged only after attestation is fully persisted | `charge_attestation_fee` called last (INTERACTION) |
