# TrustLink Troubleshooting & FAQ

Common integration failures, their causes, and how to fix them.  
For the full error enum, see [`src/errors.rs`](../src/errors.rs).

---

## Quick-reference: error codes

| Code | Enum variant | One-line cause |
|------|-------------|----------------|
| `#1`  | `AlreadyInitialized`      | `initialize()` called twice |
| `#2`  | `NotInitialized`          | Contract not yet initialized |
| `#3`  | `Unauthorized`            | Caller is not admin / issuer / subject |
| `#4`  | `NotFound`                | Attestation ID does not exist |
| `#5`  | `DuplicateAttestation`    | Same (issuer, subject, claim_type) already exists |
| `#6`  | `AlreadyRevoked`          | Attestation was already revoked |
| `#7`  | `Expired`                 | Attestation expiration timestamp has passed |
| `#8`  | `InvalidValidFrom`        | `valid_from` is after `expiration` |
| `#9`  | `InvalidExpiration`       | Expiration is in the past |
| `#10` | `MetadataTooLong`         | Metadata string exceeds 1024 bytes |
| `#11` | `InvalidTimestamp`        | Timestamp is zero or implausibly far in future |
| `#12` | `InvalidFee`              | Fee amount is negative |
| `#13` | `FeeTokenRequired`        | Fee is configured but no token address was provided |
| `#14` | `TooManyTags`             | More than 10 tags supplied |
| `#15` | `TagTooLong`              | A tag string exceeds 64 bytes |
| `#16` | `InvalidThreshold`        | Multi-sig threshold < 1 or > signers count |
| `#17` | `NotRequiredSigner`       | Co-signer not in proposal's required-signers list |
| `#18` | `AlreadySigned`           | This signer already co-signed the proposal |
| `#19` | `ProposalFinalized`       | Proposal already activated |
| `#20` | `ProposalExpired`         | 7-day co-signing window elapsed |
| `#21` | `ReasonTooLong`           | Revocation reason exceeds 128 characters |
| `#22` | `CannotEndorseOwn`        | Endorser tried to endorse their own attestation |
| `#23` | `AlreadyEndorsed`         | Endorser already endorsed this attestation |
| `#24` | `ContractPaused`          | Contract is paused; write operations blocked |
| `#25` | `SubjectNotWhitelisted`   | Issuer has whitelist mode on and subject is not in it |
| `#26` | `DelegationNotFound`      | No delegation exists for caller + issuer + claim type |
| `#27` | `DelegationExpired`       | Delegation has passed its expiration |
| `#28` | `CannotDelegateToSelf`    | Issuer tried to delegate to their own address |
| `#29` | `LastAdminCannotBeRemoved`| Removing the only remaining admin |
| `#30` | `RateLimited`             | Issuer must wait before creating another attestation |
| `#31` | `InvalidClaimType`        | Claim type string is empty or contains illegal characters |
| `#32` | `InvalidJurisdiction`     | Jurisdiction string is empty |
| `#33` | `LimitExceeded`           | Issuer or subject attestation limit reached |
| `#34` | `BatchTooLarge`           | Batch exceeds the configured maximum (default 100) |
| `#35` | `ClaimTypeNotRegistered`  | Claim type not in registry (when enforcement is on) |
| `#36` | `InvalidFeeToken`         | Supplied fee token does not match configured token |
| `#37` | `DuplicateRequest`        | An attestation request with this ID already exists |
| `#38` | `RequestAlreadyProcessed` | Request was already fulfilled or rejected |
| `#39` | `RequestExpired`          | Attestation request has expired |
| `#40` | `AlreadyApproved`         | Council vote already cast |
| `#41` | `CouncilProposalExists`   | Active council proposal already exists for this action |
| `#42` | `CouncilProposalExecuted` | Proposal was already executed |
| `#43` | `CouncilProposalExpired`  | Council proposal voting window elapsed |
| `#44` | `InvalidSourceReference`  | Source reference string is missing or empty |

---

## Common integration failures

### 1. Wrong network passphrase

**Symptom**  
Transaction submission fails immediately with an authentication or hash-mismatch error from the Stellar RPC, not from the contract itself. In the TypeScript SDK the error typically reads:

```
Error: Transaction could not be deserialized: bad sequence
```

or the transaction simply isn't accepted by the node.

**Cause**  
The `networkPassphrase` used when building the transaction does not match the network the RPC endpoint belongs to.

| Network | Correct passphrase |
|---------|-------------------|
| Testnet | `Test SDF Network ; September 2015` |
| Mainnet | `Public Global Stellar Network ; September 2015` |

Note the trailing space before the semicolons — this is intentional and must be exact.

**Fix**  
Use `Networks.TESTNET` or `Networks.PUBLIC` from `@stellar/stellar-sdk` instead of a hand-written string:

```typescript
import { Networks } from "@stellar/stellar-sdk";

const tx = new TransactionBuilder(account, {
  fee: "100",
  networkPassphrase: Networks.TESTNET, // not a raw string
}).addOperation(operation).setTimeout(30).build();
```

For Rust / Soroban CLI:
```bash
soroban contract invoke --network testnet ...
# or pass the passphrase explicitly:
soroban contract invoke \
  --network-passphrase "Test SDF Network ; September 2015" ...
```

---

### 2. Expired attestation treated as a contract error (`#7 Expired`)

**Symptom**  
A call to `has_valid_claim` returns `false` even though you can see the attestation exists. Alternatively, calling `get_attestation` succeeds but a downstream contract call fails with `Error(Contract, #7)`.

**Cause**  
`has_valid_claim` silently excludes expired attestations (returns `false`). If you call `get_attestation_status` or pass the attestation directly to another contract that checks validity, you receive `Expired` (#7) as a hard error.

`Expired` is **not retryable**. The attestation must be renewed or a new one created.

**Fix**  
Check status explicitly before acting:

```typescript
async function ensureValidAttestation(attestationId: string): Promise<void> {
  const status = await getAttestationStatus(attestationId);
  if (status === "Expired") {
    throw new Error("Attestation has expired — please renew verification.");
  }
}
```

In Rust:
```rust
match trustlink.get_attestation_status(&attestation_id) {
    Ok(AttestationStatus::Valid)   => { /* proceed */ }
    Ok(AttestationStatus::Expired) => return Err(Error::AttestationExpired),
    Ok(AttestationStatus::Revoked) => return Err(Error::AttestationRevoked),
    Err(_)                         => return Err(Error::NotFound),
}
```

To renew, the original issuer calls:
```bash
soroban contract invoke --id <CONTRACT> --source ISSUER_SECRET \
  -- renew_attestation \
  --issuer ISSUER_PUBKEY \
  --attestation_id <ID> \
  --new_expiration 1893456000  # Unix timestamp
```

---

### 3. Rate-limit errors (`#30 RateLimited`)

**Symptom**  
`create_attestation` fails with `Error(Contract, #30)` intermittently, particularly under high volume from the same issuer.

**Cause**  
The admin has configured a minimum issuance interval via `set_rate_limit`. An issuer must wait at least `min_issuance_interval` seconds between attestations. The contract enforces this per-issuer, and the error is returned as soon as the interval is violated.

**Fix**

1. **Check the current rate limit** before bulk operations:

```typescript
const config = await getRateLimit(callerPublicKey);
if (config) {
  console.log(`Min interval: ${config.min_issuance_interval}s`);
}
```

2. **Space your requests** when creating attestations in bulk. Add a delay between submissions:

```typescript
async function createAttestationsThrottled(
  issuer: Keypair,
  subjects: string[],
  claimType: string,
  intervalMs: number
): Promise<void> {
  for (const subject of subjects) {
    await createAttestation(issuer, subject, claimType);
    await new Promise((r) => setTimeout(r, intervalMs));
  }
}
```

3. **Use `create_attestations_batch`** instead of individual calls. The batch function counts as a single issuance event against the rate limit, making it the most efficient option for bulk operations:

```typescript
// One rate-limit event for all 50 subjects
await createAttestationsBatch(issuerKeypair, subjects, "KYC_PASSED");
```

4. **Handle the error with retry logic**:

```typescript
async function createWithRetry(
  issuer: Keypair,
  subject: string,
  claimType: string,
  maxRetries = 3
): Promise<void> {
  for (let i = 0; i < maxRetries; i++) {
    try {
      await createAttestation(issuer, subject, claimType);
      return;
    } catch (err) {
      const tlErr = parseTrustLinkError(err);
      if (tlErr?.code === 30) {
        // RateLimited — wait and retry
        await new Promise((r) => setTimeout(r, (i + 1) * 2000));
        continue;
      }
      throw err;
    }
  }
  throw new Error("Rate-limit retries exhausted");
}
```

---

### 4. Fee-token mismatch (`#13 FeeTokenRequired`, `#36 InvalidFeeToken`)

**Symptom**  
`create_attestation` fails with `Error(Contract, #13)` or `Error(Contract, #36)`.

**Cause**  
- `#13 FeeTokenRequired`: The contract has an attestation fee configured, but the caller did not approve a token transfer to the fee collector before calling `create_attestation`. The contract requires a prior `token.approve(...)` call.
- `#36 InvalidFeeToken`: The caller approved a different token than the one the admin configured.

**Fix**

1. **Query the fee config first**:

```typescript
const fee = await getAttestationFee(issuerPublicKey);
if (fee?.feeEnabled) {
  console.log(`Fee token:    ${fee.tokenAddress}`);
  console.log(`Fee amount:   ${fee.amount}`);
  console.log(`Fee collector: ${fee.collectorAddress}`);
}
```

2. **Approve the correct token** before calling `create_attestation`:

```typescript
import { Contract, xdr, nativeToScVal } from "@stellar/stellar-sdk";

async function approveAndAttest(
  issuerKeypair: Keypair,
  subject: string,
  claimType: string
): Promise<void> {
  const fee = await getAttestationFee(issuerKeypair.publicKey());

  if (fee?.feeEnabled && fee.tokenAddress) {
    // Step 1: approve the fee token
    const tokenContract = new Contract(fee.tokenAddress);
    const approveOp = tokenContract.call(
      "approve",
      nativeToScVal(issuerKeypair.publicKey(), { type: "address" }), // from
      nativeToScVal(fee.collectorAddress, { type: "address" }),       // spender
      nativeToScVal(fee.amount, { type: "i128" }),                    // amount
      nativeToScVal(200, { type: "u32" })                             // expiration_ledger
    );
    await submitTransaction(issuerKeypair, [approveOp]);
  }

  // Step 2: create the attestation
  await createAttestation(issuerKeypair, subject, claimType);
}
```

3. **Verify the token address exactly**. The contract stores a single configured token address; using a different stablecoin or wrapped asset will produce `#36 InvalidFeeToken`.

---

### 5. Duplicate attestation (`#5 DuplicateAttestation`)

**Symptom**  
`create_attestation` returns `Error(Contract, #5)` on a second call for the same subject.

**Cause**  
Attestation IDs are deterministic: they are a hash of `(issuer, subject, claim_type)`. If an attestation with that combination already exists and is not revoked, creating another one fails.

**Fix**  
Check first with `has_valid_claim` or `get_attestation_by_type`, then only create if absent:

```typescript
const existing = await getAttestationByType(subject, claimType);
if (!existing) {
  await createAttestation(issuerKeypair, subject, claimType);
} else {
  console.log("Attestation already exists:", existing);
}
```

---

### 6. Contract paused (`#24 ContractPaused`)

**Symptom**  
All write operations (`create_attestation`, `revoke_attestation`, etc.) fail with `Error(Contract, #24)`.

**Cause**  
An admin called `pause()`. All state-mutating functions are disabled while paused. Read-only functions (`has_valid_claim`, `get_attestation`, etc.) still work.

**Fix**  
Check the pause status and inform users:

```typescript
const paused = await isPaused(callerPublicKey);
if (paused) {
  throw new Error("TrustLink is under maintenance. Read queries still work. Please try again later.");
}
```

Only the admin can call `unpause()` to resume normal operation.

---

### 7. Subject not whitelisted (`#25 SubjectNotWhitelisted`)

**Symptom**  
`create_attestation` fails with `Error(Contract, #25)` for a specific issuer.

**Cause**  
The issuer has whitelist mode enabled (`enable_whitelist_mode`). Only subjects explicitly added via `add_to_whitelist` are accepted.

**Fix**  
The issuer must whitelist the subject first:

```bash
soroban contract invoke --id <CONTRACT> --source ISSUER_SECRET \
  -- add_to_whitelist \
  --issuer ISSUER_PUBKEY \
  --subject SUBJECT_PUBKEY
```

Or check whitelist mode before attempting:

```typescript
const whitelistEnabled = await isWhitelistEnabled(issuerAddress);
if (whitelistEnabled) {
  const listed = await isWhitelisted(issuerAddress, subjectAddress);
  if (!listed) throw new Error("Subject must be whitelisted by issuer first.");
}
```

---

## Diagnosing simulation vs. on-chain failures

| Error surface | Meaning | Action |
|--------------|---------|--------|
| `simulateTransaction` fails | Bad arguments, contract logic error, auth issue | Fix before submitting |
| `sendTransaction` returns `ERROR` | Ledger-level rejection (sequence number, auth, resource limits) | Check `errorResult` field |
| `getTransaction` returns `FAILED` | Transaction submitted but reverted on-chain | Inspect `resultMetaXdr` for contract error code |

Use this snippet to decode the error code from a failed `getTransaction` response:

```typescript
function decodeResultMeta(resultMetaXdr: string): string {
  const meta = xdr.TransactionMeta.fromXDR(resultMetaXdr, "base64");
  return JSON.stringify(meta.toXDR("base64")); // inspect in Stellar Laboratory
}
```

---

## Further reading

- [Integration Guide](./integration-guide.md) — step-by-step setup and code examples
- [Architecture Overview](./architecture.md) — how components fit together
- [Performance Reference](./performance.md) — compute unit budgets
- [`src/errors.rs`](../src/errors.rs) — canonical error enum source of truth
