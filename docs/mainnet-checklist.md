# Stellar Mainnet Deployment Checklist

> Reviewer: ________________  Date: ________________
>
> Each item must be marked **PASS**, **FAIL**, or **N/A** before deployment proceeds.
> A single FAIL blocks deployment unless explicitly waived with justification.

---

## 1. Security Audit

| # | Item | Status | Notes |
|---|------|--------|-------|
| 1.1 | External security audit completed by a qualified third party | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 1.2 | All critical and high findings resolved or formally accepted | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 1.3 | Medium/low findings triaged and tracked | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 1.4 | Reentrancy audit reviewed (`docs/reentrancy-audit.md`) | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 1.5 | Security review document signed off (`docs/security-review.md`) | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 1.6 | No hardcoded secrets, private keys, or sensitive values in source | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 1.7 | Dependency audit run — no known critical CVEs | [ ] PASS / [ ] FAIL / [ ] N/A | |

---

## 2. Testing

| # | Item | Status | Notes |
|---|------|--------|-------|
| 2.1 | All unit tests pass (`cargo test`) | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 2.2 | All integration tests pass (`tests/integration_test.rs`) | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 2.3 | Authorization tests pass (`tests/authorization.rs`) | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 2.4 | Fuzz/property tests pass (`tests/id_generation_fuzz.rs`) | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 2.5 | All test snapshots up to date and committed | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 2.6 | Tests run against the exact WASM artifact being deployed | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 2.7 | Edge cases tested: empty inputs, max-length fields, boundary timestamps | [ ] PASS / [ ] FAIL / [ ] N/A | |

---

## 3. WASM Build

| # | Item | Status | Notes |
|---|------|--------|-------|
| 3.1 | WASM built with `--release` profile | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 3.2 | WASM optimized (e.g. `wasm-opt -Oz` or `stellar contract optimize`) | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 3.3 | Optimized WASM size recorded and within acceptable limits (<100KB recommended) | [ ] PASS / [ ] FAIL / [ ] N/A | Size: _______ KB |
| 3.4 | WASM hash/checksum recorded for deployment verification | [ ] PASS / [ ] FAIL / [ ] N/A | SHA256: _______ |
| 3.5 | WASM artifact stored in a versioned, immutable location | [ ] PASS / [ ] FAIL / [ ] N/A | |

---

## 4. Admin Key Security

| # | Item | Status | Notes |
|---|------|--------|-------|
| 4.1 | Admin key stored on a hardware wallet (Ledger or equivalent) — strongly recommended | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 4.2 | Admin key is not a hot wallet or stored in plaintext anywhere | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 4.3 | Admin key access is restricted to authorized personnel only | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 4.4 | Backup/recovery procedure for admin key documented and tested | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 4.5 | Multisig or threshold policy configured if applicable | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 4.6 | Testnet deployment verified with the same key management setup | [ ] PASS / [ ] FAIL / [ ] N/A | |

---

## 5. Post-Deployment Verification

After the contract is deployed and initialized on mainnet, complete the
verification checklist below. Each item must be marked **PASS**, **FAIL**, or
**N/A** (only if not applicable).

| # | Step | Expected result | Status | Notes |
|---|-------|------------------|--------|-------|
| 5.1 | Run verification script | `./scripts/verify_deployment.sh --contract "$CONTRACT_ID" --source "$ADMIN_SECRET_ALIAS" --network mainnet` exits `0` | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 5.2 | Confirm `get_admin` returns correct address | `get_admin == $ADMIN_PUBLIC` | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 5.3 | Confirm `get_version` returns expected version | `get_version == <expected_semver>` | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 5.4 | Confirm `health_check` indicates initialized state | `health_check.initialized == true` | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 5.5 | Verify indexer is processing events from the new contract | Indexer `last_processed` / lag improves and events from `$CONTRACT_ID` appear in indexing (no sustained silence) | [ ] PASS / [ ] FAIL / [ ] N/A | |

Notes / commands:

- `make verify` (if available in your operator workflow) should run the same
  checks as the script above.
- `stellar contract invoke --id "$CONTRACT_ID" --network mainnet -- get_admin`
  should match the `ADMIN_PUBLIC` recorded during initialization.
- `stellar contract invoke --id "$CONTRACT_ID" --network mainnet -- get_version`
  should match the version you intended to deploy.
- `stellar contract invoke --id "$CONTRACT_ID" --network mainnet -- health_check`
  should report `initialized: true`.
- Indexer validation can be done via:
  - Prometheus metrics exported by the indexer (see `docs/monitoring.md`), and/or
  - Observing that new events emitted by the freshly deployed contract are reflected in your API/DB within the expected indexing window.

---

## 6. Monitoring

| # | Item | Status | Notes |
|---|------|--------|-------|
| 6.1 | Indexer deployed and syncing (`indexer/`) | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 6.2 | Alerting configured for contract errors and unexpected state changes | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 6.3 | Dashboard or observability tooling live (see `docs/monitoring.md`) | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 6.4 | On-call rotation or responsible party identified for alerts | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 6.5 | Fee collection and admin operations are being logged/tracked | [ ] PASS / [ ] FAIL / [ ] N/A | |

---

## 7. Incident Response Plan

| # | Item | Status | Notes |
|---|------|--------|-------|
| 7.1 | Incident response runbook written and accessible to the team | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 7.2 | Escalation path defined (who to contact, in what order) | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 7.3 | Communication plan ready (users, partners, public disclosure) | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 7.4 | Emergency pause or freeze mechanism identified (if available) | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 7.5 | Post-incident review process defined | [ ] PASS / [ ] FAIL / [ ] N/A | |

---

## 8. Rollback Plan

| # | Item | Status | Notes |
|---|------|--------|-------|
| 8.1 | Previous stable WASM artifact retained and accessible | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 8.2 | Rollback procedure documented and tested on testnet | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 8.3 | Data migration or state compatibility assessed for rollback scenario | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 8.4 | Decision criteria defined: what triggers a rollback vs. a hotfix | [ ] PASS / [ ] FAIL / [ ] N/A | |
| 8.5 | Rollback ownership assigned (who executes it) | [ ] PASS / [ ] FAIL / [ ] N/A | |

---

## 9. Final Sign-off

| Role | Name | Signature | Date |
|------|------|-----------|------|
| Author / Lead Engineer | | | |
| Reviewer | | | |
| Security Lead (if applicable) | | | |

**Deployment approved:** [ ] YES — all items PASS or formally waived  
**Waived items (list any FAILs accepted with justification):**

```
Item #:   Justification:
```

---

*Reference docs: `docs/security.md`, `docs/security-review.md`, `docs/reentrancy-audit.md`, `docs/monitoring.md`, `DEPLOYMENT.md`*

