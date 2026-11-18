# TrustLink Attestation Request Lifecycle Example

This example demonstrates the complete **pull-model** attestation workflow in TrustLink: subjects initiating on-chain attestation requests, issuers inspecting and resolving them (via fulfillment or rejection), subjects verifying resulting credentials, cancellation of pending requests, and lifecycle maintenance (expiration and cleanup).

---

## 1. Architecture: Push Model vs. Pull Model

TrustLink supports two core interaction models for attestation issuance:

* **Push Model (`create_attestation`)**: The issuer directly mints and stores the attestation for a subject (e.g., automated KYC onboards or enterprise integrations).
* **Pull Model (`request_attestation`)**: The subject initiates an on-chain request asking a specific issuer for a claim. The issuer reviews pending requests asynchronously, then either fulfills or rejects the request.

---

## 2. Request Lifecycle & State Transitions

```
                    ┌────────────────────────┐
                    │        Pending         │
                    └───────────┬────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        │                       │                       │
        ▼                       ▼                       ▼
┌───────────────┐       ┌───────────────┐       ┌───────────────┐
│   Fulfilled   │       │   Rejected    │       │   Cancelled   │
└───────────────┘       └───────────────┘       └───────────────┘
  (Issuer signs;          (Issuer signs;          (Subject signs;
  mints attestation)      records reason)         revokes intent)
                                │
                                ▼
                        ┌───────────────┐
                        │    Expired    │
                        └───────────────┘
                         (TTL: 7 days;
                         lazy / cleanup)
```

| State | Who Triggers? | Description |
|---|---|---|
| **`Pending`** | Subject | Created via `request_attestation(subject, issuer, claim_type)`. Placed in issuer's pending queue. |
| **`Fulfilled`** | Issuer | Created via `fulfill_request(issuer, request_id, expiration)`. Mints attestation on-chain and prunes pending queue. |
| **`Rejected`** | Issuer | Created via `reject_request(issuer, request_id, reason)`. Attaches rejection reason and removes from pending queue. |
| **`Cancelled`** | Subject | Created via `cancel_request(subject, request_id)`. Subject withdraws their unfulfilled request. |
| **`Expired`** | Time / Ledger | If unprocessed after `ATTESTATION_REQUEST_TTL_SECS` (7 days), request cannot be fulfilled and can be cleaned up. |

---

## 3. Step-by-Step Walkthrough

### Step 1: Subject Initiates Request
The subject signs and submits a request specifying the desired claim type and target issuer:

```typescript
const requestId = await client.requestAttestation(
  subjectKeypair,
  issuerAddress,
  "KYC_PASSED"
);
console.log("Created request:", requestId);
```

### Step 2: Issuer Queries Pending Requests
Issuers regularly inspect their inbound pending queue using pagination:

```typescript
const pendingRequests = await client.getPendingRequests(issuerAddress, 0, 10);
for (const req of pendingRequests) {
  console.log(`Reviewing ${req.id} for subject ${req.subject} (${req.claim_type})`);
}
```

### Step 3: Issuer Fulfills a Request
When the subject's criteria are satisfied, the issuer fulfills the request. This atomically:
1. Mints a new on-chain `Attestation`.
2. Sets `request.status = Fulfilled`.
3. Removes the request from the issuer's active pending list.
4. Emits `req_ok` and `created` contract events.

```typescript
const attestationId = await client.fulfillRequest(
  issuerKeypair,
  requestId,
  optionalExpirationTimestamp
);
console.log("Attestation minted:", attestationId);
```

### Step 4: Issuer Rejects a Request
If documentation is insufficient or invalid, the issuer rejects the request with an explanatory reason:

```typescript
await client.rejectRequest(
  issuerKeypair,
  requestId,
  "Incomplete tax filing documentation"
);
```

### Step 5: Subject / Verifier Validates Credentials
Third-party verifiers query the contract to confirm valid credentials:

```typescript
const hasKyc = await client.hasValidClaim(subjectAddress, "KYC_PASSED");
console.log("Subject KYC valid:", hasKyc); // true

const hasAccredited = await client.hasValidClaim(subjectAddress, "ACCREDITED_INVESTOR");
console.log("Subject Accredited valid:", hasAccredited); // false (rejected)
```

### Step 6: Subject Cancels Request
If a subject submitted a request in error or no longer requires it:

```typescript
await client.cancelRequest(subjectKeypair, requestId);
```

---

## 4. Expiration and Cleanup Mechanism

Attestation requests occupy on-chain ledger storage. TrustLink implements a two-tier expiration policy:

### A. 7-Day Request TTL
Every request has an expiration deadline:
$$\text{expires\_at} = \text{timestamp} + \text{ATTESTATION_REQUEST_TTL_SECS}$$
Where `ATTESTATION_REQUEST_TTL_SECS = 604,800` (7 days). Once passed:
* `fulfill_request` rejects with `Error::RequestExpired` (#10).
* `reject_request` rejects with `Error::RequestExpired` (#10).

### B. Lazy Expiration on Read
When `get_request(request_id)` is invoked, the contract evaluates the current ledger timestamp. If `status == Pending && current_time >= expires_at`, it automatically returns `RequestStatus::Expired` without requiring prior state migration.

### C. Storage Pruning via `cleanup_expired_requests`
To reclaim ledger storage and prevent unbounded growth of an issuer's pending vector, the registered issuer calls:

```typescript
await client.cleanupExpiredRequests(issuerKeypair);
```

This iterates the issuer's pending IDs, persists `RequestStatus::Expired`, removes stale IDs from the pending list, and emits `req_exp` events.

---

## 5. Contract Error Codes

| Error Code | Name | Description |
|---|---|---|
| `#3` | `Unauthorized` | Caller is not the authorized subject (for cancel) or registered issuer (for fulfill/reject). |
| `#4` | `NotFound` | The requested request ID does not exist in storage. |
| `#9` | `DuplicateRequest` | A pending, unexpired request for the same `(subject, issuer, claim_type)` already exists. |
| `#10` | `RequestExpired` | The 7-day TTL elapsed before the request was processed. |
| `#11` | `RequestAlreadyProcessed` | Attempted to fulfill, reject, or cancel a request that is already terminal. |

---

## 6. Running the Example & Tests

### Prerequisites
* Node.js ≥ 18
* `@stellar/stellar-sdk`

### Run Flow
```bash
node flow.ts
```

### Run Unit Tests
```bash
npm test
# Or directly with Node's built-in test runner:
node --test test/flow.test.mjs
```
