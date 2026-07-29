# TrustLink Mainnet Deployment Runbook

Step-by-step guide for deploying TrustLink to Stellar mainnet. Complete the
[mainnet-checklist.md](./mainnet-checklist.md) before starting.

---

## 1. Pre-Deployment Checklist

Run through these checks immediately before executing the deployment. Each must
pass or be formally waived.

```bash
# 1. All tests green
cargo test

# 2. Build the optimized WASM
make optimize

# 3. Record the WASM size — must be < 100 KB
ls -lh target/wasm32-unknown-unknown/release/trustlink.optimized.wasm

# 4. Record the SHA-256 hash for post-deploy verification
sha256sum target/wasm32-unknown-unknown/release/trustlink.optimized.wasm
```

| Check | Expected |
|---|---|
| `cargo test` exit code | `0` |
| WASM artifact exists | `trustlink.optimized.wasm` present |
| WASM size | < 100 KB |
| SHA-256 recorded | noted for step 4 |
| Admin key on hardware wallet | confirmed |
| Testnet smoke-test passed | confirmed |
| Security audit sign-off | confirmed |

---

## 2. Deployment Commands

Set environment variables once; all commands below reference them.

```bash
export ADMIN_SECRET=SXXX...          # hardware wallet or secure store — never commit
export ADMIN_PUBLIC=GXXX...          # corresponding public key
```

### Step 1 — Upload the WASM

```bash
stellar contract upload \
  --source "$ADMIN_SECRET" \
  --network mainnet \
  --wasm target/wasm32-unknown-unknown/release/trustlink.optimized.wasm
```

Expected output:

```
<WASM_HASH>   # 64-character hex string — save this
```

Verify the hash matches the SHA-256 recorded in the pre-deployment checklist.

### Step 2 — Deploy the contract

```bash
stellar contract deploy \
  --wasm-hash <WASM_HASH> \
  --source "$ADMIN_SECRET" \
  --network mainnet
```

Expected output:

```
<CONTRACT_ID>   # C... address — save this
```

```bash
export CONTRACT_ID=<CONTRACT_ID>
```

### Step 3 — Initialize

```bash
stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source "$ADMIN_SECRET" \
  --network mainnet \
  -- initialize \
  --admin "$ADMIN_PUBLIC" \
  --ttl_days null
```

Expected output: `null` (no return value on success).

### Step 4 — Register production issuers

Repeat for each trusted issuer:

```bash
stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source "$ADMIN_SECRET" \
  --network mainnet \
  -- register_issuer \
  --admin "$ADMIN_PUBLIC" \
  --issuer <ISSUER_PUBLIC_KEY>
```

Expected output: `null`.

---

## 3. Post-Deployment Verification

Run these checks immediately after deployment to confirm the contract is live
and correctly initialized.

### 3.1 Confirm admin

```bash
stellar contract invoke \
  --id "$CONTRACT_ID" \
  --network mainnet \
  -- get_admin
```

Expected: `"<ADMIN_PUBLIC>"` — must match `$ADMIN_PUBLIC`.

### 3.2 Health check

```bash
stellar contract invoke \
  --id "$CONTRACT_ID" \
  --network mainnet \
  -- health_check
```

Expected:

```json
{ "initialized": true, "admin_set": true, "issuer_count": <N>, "total_attestations": 0 }
```

### 3.3 Confirm issuers registered

```bash
stellar contract invoke \
  --id "$CONTRACT_ID" \
  --network mainnet \
  -- is_issuer \
  --issuer <ISSUER_PUBLIC_KEY>
```

Expected: `true` for each issuer registered in step 4.

### 3.4 End-to-end smoke test

Use the verification script (creates and revokes a test attestation, then
cleans up):

```bash
./scripts/verify_deployment.sh \
  --contract "$CONTRACT_ID" \
  --source "$ADMIN_SECRET" \
  --network mainnet
```

Expected exit code: `0`.

### 3.5 Record deployment details

| Field | Value |
|---|---|
| Contract ID | |
| WASM hash | |
| Deploy transaction | |
| Init transaction | |
| Deployed by | |
| Date (UTC) | |

Commit these values to `DEPLOYMENT.md`.

---

## 4. Rollback Procedure

TrustLink does not support deleting a deployed contract — Soroban storage is
immutable. "Rollback" means one of two things depending on the situation:

### Option A — Upgrade back to the previous WASM (preferred)

Use this when a new version was deployed but the previous version was stable.

```bash
# 1. Re-upload the previous optimized WASM
stellar contract upload \
  --source "$ADMIN_SECRET" \
  --network mainnet \
  --wasm <PATH_TO_PREVIOUS_WASM>
# Outputs: <PREVIOUS_WASM_HASH>

# 2. Invoke upgrade on the live contract
stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source "$ADMIN_SECRET" \
  --network mainnet \
  -- upgrade \
  --admin "$ADMIN_PUBLIC" \
  --new_wasm_hash <PREVIOUS_WASM_HASH>

# 3. Verify admin and state are intact
stellar contract invoke \
  --id "$CONTRACT_ID" \
  --network mainnet \
  -- get_admin
```

All storage (issuers, attestations) is preserved — only the executable is
replaced.

### Option B — Pause and redirect (when the contract is broken)

If the contract cannot be upgraded (e.g. the `upgrade` function itself is
broken), pause activity and deploy a fresh instance:

1. Communicate the incident to all issuers and integrators immediately.
2. Deploy a new contract instance following section 2.
3. Re-register all issuers on the new instance.
4. Update all integrators with the new `CONTRACT_ID`.
5. Mark the old contract as deprecated in `DEPLOYMENT.md`.

### Decision criteria

| Situation | Action |
|---|---|
| Bug in new version, previous version stable | Option A — upgrade back |
| Critical security vulnerability, contract must stop | Option B — pause and redeploy |
| Data corruption or storage inconsistency | Option B — redeploy; assess migration |

### Rollback ownership

The admin key holder is responsible for executing the rollback. A second
authorized team member must confirm the decision before execution on mainnet.

---

*Related: [mainnet-checklist.md](./mainnet-checklist.md) · [DEPLOYMENT.md](../DEPLOYMENT.md) · [docs/security.md](./security.md) · [docs/monitoring.md](./monitoring.md)*
