# Reentrancy Audit — TrustLink

## Scope

This document covers the reentrancy analysis for `create_attestation` and its
interaction with the external token contract used for fee collection.

## The Risk

`create_attestation` calls an external token contract (`TokenClient::transfer`)
to collect the attestation fee. A malicious token contract could re-enter
`create_attestation` during that call, potentially causing:

- Double-issuance of the same attestation
- Inconsistent per-issuer / per-subject counters
- Bypassed rate-limit or storage-limit checks

### Attack Scenario (before fix)

```
create_attestation(issuer, subject, claim_type, ...)
  └─> charge_attestation_fee()
        └─> malicious_token.transfer()
              └─> re-enters create_attestation(issuer, subject, claim_type, ...)
                    └─> state not yet stored → duplicate passes all guards
  └─> store_attestation()   ← too late, duplicate already stored
```

## Mitigation: Checks-Effects-Interactions (CEI)

The contract follows the **Checks-Effects-Interactions** pattern:

```
create_attestation_internal()
  1. CHECKS  — validate inputs, auth, rate limit, duplicate guard
  2. EFFECTS — store_attestation(), append_audit_entry(),
               set_last_issuance_time()   ← all state committed
  3. INTERACTION — charge_attestation_fee() → TokenClient::transfer()
  4. EVENT   — Events::attestation_created()
```

By the time the external token contract is called, the attestation record is
fully persisted. A re-entrant call will hit the duplicate-ID guard
(`Error::DuplicateAttestation`) and be rejected.

### Rate Limiting as Defense-in-Depth

`set_last_issuance_time` is written in the EFFECTS phase. If a re-entrant call
somehow bypasses the duplicate guard (e.g. with a different `claim_type`), the
rate-limit check will block it because the timestamp was already recorded.

## Invariants

| Invariant | Guaranteed by |
|-----------|---------------|
| No duplicate attestation IDs | `has_attestation` check before store |
| `IssuerStats.total_issued` matches actual count | incremented in EFFECTS |
| Audit log entry exists for every stored attestation | both written in EFFECTS |
| Rate-limit timestamp always recorded before external call | written in EFFECTS |

## Soroban Context

Soroban's WASM execution model does not provide automatic reentrancy locks.
Cross-contract calls within a single transaction can re-enter the same contract.
The CEI pattern is therefore the primary defense.

The `ExpirationCallbackClient` (expiration hooks) is called from `has_valid_claim`
via `try_notify_expiring`, which uses `try_` (non-panicking). Failures are
silently swallowed so the main flow is never interrupted and no state is mutated
after the external call.

## Conclusion

TrustLink is protected against reentrancy in `create_attestation` through:

1. **CEI ordering** — all state written before `TokenClient::transfer`
2. **Duplicate-ID guard** — re-entrant calls with the same inputs are rejected
3. **Rate limiting** — timestamp recorded before external call blocks rapid re-entry
4. **`try_` calls for hooks** — expiration callbacks cannot panic the contract
