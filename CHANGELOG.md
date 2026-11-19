# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

<!-- Add new changes here before they are released. Use the categories below:
### Added
### Changed
### Deprecated
### Removed
### Fixed
### Security
-->

## [0.1.0] - 2026-03-25

### Added

- `initialize(admin, ttl_days)` — deploy and set the contract administrator with configurable storage TTL.
- `register_issuer(admin, issuer)` — admin registers a trusted attestation issuer.
- `remove_issuer(admin, issuer)` — admin removes an issuer from the registry.
- `is_issuer(address)` — query whether an address is an authorized issuer.
- `get_admin()` — return the current admin address.
- `transfer_admin(current_admin, new_admin)` — transfer contract administration rights.
- `create_attestation(issuer, subject, claim_type, expiration, metadata)` — issuer creates a new attestation with optional expiration and metadata; returns a deterministic hash-based ID.
- `revoke_attestation(issuer, attestation_id)` — issuer marks an attestation as revoked.
- `get_attestation(attestation_id)` — fetch full attestation data by ID.
- `get_attestation_status(attestation_id)` — return `Valid`, `Expired`, or `Revoked`; emits an `expired` event when status is `Expired`.
- `has_valid_claim(subject, claim_type)` — returns `true` if the subject holds a non-expired, non-revoked attestation of the given type; emits an `expired` event for any expired attestation encountered.
- `has_valid_claim_from_issuer(subject, claim_type, issuer)` — constrain verification to a specific issuer.
- `has_any_claim(subject, claim_types)` and `has_all_claims(subject, claim_types)` — OR/AND claim verification across multiple claim types.
- `get_subject_attestations(subject, start, limit)` — paginated list of attestation IDs for a subject.
- `get_issuer_attestations(issuer, start, limit)` — paginated list of attestation IDs issued by an issuer.
- `get_subject_attestation_count(subject)`, `get_issuer_attestation_count(issuer)`, and `get_valid_claim_count(subject)` — aggregate query helpers.
- Claim type registry: `register_claim_type`, `update_claim_type`, `remove_claim_type`, `get_claim_type_description`, and `list_claim_types`.
- Historical import support: `import_attestation(admin, issuer, subject, claim_type, timestamp, expiration)` and `Attestation.imported`.
- Fee configuration: `set_fee(admin, fee, collector, fee_token)` and `get_fee_config()` with optional token-denominated attestation fees.
- Bridge support: `register_bridge`, `remove_bridge`, `is_bridge`, and `bridge_attestation` with source-chain metadata.
- Batch operations: `create_attestations_batch` and `revoke_attestations_batch`.
- Expiration hooks: `register_expiration_hook`, `get_expiration_hook`, and `remove_expiration_hook` for callback notifications.
- Multi-signature attestations: `propose_attestation`, `cosign_attestation`, and `get_multisig_proposal`.
- Global and per-issuer statistics: `get_global_stats`, `get_issuer_stats`, and issuer tier/metadata management.
- Comprehensive event set for creation, revocation, bridge/import, fee updates, claim-type administration, multi-sig lifecycle, and expiration hooks.
- Integration examples under `examples/` including KYC token and governance-gated voting patterns.

### Fixed

- Validation coverage for metadata, tag cardinality/length, and timestamp/expiration edge cases.
- Deterministic storage/index consistency for issuer and subject attestation lookups.
- Authorization checks across admin, issuer, bridge, and multisig signer flows.

[Unreleased]: https://github.com/Haroldwonder/TrustLink/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Haroldwonder/TrustLink/releases/tag/v0.1.0
Below is the README written directly in the chat, covering both requirements: rental review integrity and RPC/Horizon outage resilience. It includes implementation rules, database constraints, API examples, pseudocode, test cases, acceptance criteria, and deployment procedures.

### README.md — Rental Reviews Integrity & Chain Resilience

### 1\. Project overview

This project addresses two critical reliability and security requirements in the rental platform:

* Ensure reviews are submitted only by eligible participants of completed rentals, with no duplicate reviews or ratings manipulated by unrelated accounts.

* Ensure RPC and Horizon outages do not cause duplicate financial transactions or unnecessarily disable unrelated read-only product features.

The implementation must enforce business rules at the service layer and database boundary, provide observable dependency health, and reconcile ambiguous transaction outcomes before allowing retries.

### 2\. Objectives

* Validate booking completion using authoritative booking records.

* Verify the authenticated user participated in the booking.

* Enforce a clearly defined review window.

* Allow only one review per eligible perspective.

* Prevent concurrent duplicate submissions.

* Define eligibility for cancelled, refunded, modified, and disputed bookings.

* Track RPC and Horizon health independently.

* Implement circuit breakers for unhealthy dependencies.

* Serve safe cached reads when appropriate.

* Restrict unsafe mutations while chain status is uncertain.

* Track pending submissions using durable operation records.

* Reconcile ambiguous submissions before retrying.

* Expose accurate degradation status to users and operations teams.

* Keep unrelated features available whenever their dependencies remain healthy.

### 3\. Scope

### 3.1 Rental review functionality

The review system must validate the booking lifecycle, participant relationships, eligibility window, review perspective, and uniqueness of each submission.

### 3.2 Chain resilience functionality

The chain resilience system must classify application actions according to their dependency on live RPC or Horizon access, provide safe fallback behavior, and prevent duplicate broadcasts.

### 3.3 Shared reliability principles

Both systems must:

* Treat backend records as the authority.

* Enforce critical invariants at the database boundary.

* Use deterministic state transitions.

* Return stable API error codes.

* Record meaningful audit events.

* Handle concurrency safely.

* Avoid trusting client-supplied eligibility or transaction status.

* Fail safely when authoritative information is unavailable.

### 4\. Non-goals

The implementation must not:

* Accept client-provided booking completion as proof of eligibility.

* Allow arbitrary users to submit reviews for bookings they did not participate in.

* Depend solely on a pre-insert duplicate check.

* Treat a timeout as proof that a transaction failed.

* Automatically broadcast a second transaction after an ambiguous response.

* Disable the entire application because one provider is unavailable.

* Use stale balances or sequence information to authorize unsafe transactions.

* Queue financial operations without durable tracking and a proven idempotency strategy.

* Expose secrets, signing keys, or sensitive provider diagnostics.

* Silently alter existing review records during migration.

### 5\. High-level architecture

### 5.1 Review submission flow

* Receive the review request.

* Authenticate the user.

* Validate the request payload.

* Load the authoritative booking record.

* Load the booking's participant relationships.

* Verify the booking is completed.

* Verify the user is an eligible participant.

* Evaluate cancellation, refund, modification, and dispute policies.

* Verify the review window.

* Derive the reviewee and perspective from stored records.

* Insert the review within a database transaction.

* Let the database uniqueness constraint prevent duplicate submissions.

* Return the canonical review or a stable error response.

* Record the result for monitoring and auditing.

### 5.2 Chain-dependent action flow

* Receive an action request.

* Authenticate and authorize the user.

* Classify the action as read-only, live-state-dependent, or mutating.

* Check the health of the required dependency.

* Serve a safe cached result where policy permits.

* Reject or hold unsafe mutations if live state cannot be trusted.

* Persist the operation record before performing the external side effect.

* Apply idempotency protection.

* Submit the transaction through the controlled broadcast service.

* Record the provider response.

* Mark ambiguous results as `UNKNOWN`.

* Reconcile transaction status using authoritative evidence.

* Permit retries only when the previous operation is proven safe to retry.

* Update the user interface and operations dashboard.

### 6\. Core domain definitions

* Booking: The authoritative rental record.

* Participant: A user recorded as a renter, property owner, host, or other explicitly eligible party.

* Perspective: The permitted direction of a review, such as renter reviewing host.

* Review window: The allowed period for submitting a review.

* Review eligibility: The result of evaluating booking state, participant relationship, timing, and policy.

* RPC provider: A service used to query blockchain state or submit transactions.

* Horizon: An API service used to query Stellar network information.

* Circuit breaker: A mechanism that blocks calls to an unhealthy dependency.

* Idempotency key: A stable identifier that prevents retries from creating duplicate logical operations.

* Ambiguous submission: A transaction attempt whose outcome cannot yet be determined.

* Reconciliation: The process of checking authoritative state to determine the outcome of a pending operation.

### Part A — Rental review integrity

### 7\. Booking completion validation

A review may be submitted only when the authoritative booking record meets the product's completed-rental definition.

The service must:

* Load the booking from the database.

* Validate the stored lifecycle status.

* Verify the completion timestamp.

* Check required handover or settlement evidence if applicable.

* Reject incomplete bookings.

* Reject cancelled bookings unless an explicitly separate feedback policy applies.

* Ignore any booking status provided by the client.

* Use server-generated timestamps.

* Handle missing or contradictory completion evidence safely.

Example eligibility rule:

```typescript
function isBookingCompleted(booking: Booking): boolean {
  return (
    booking.status === "COMPLETED" &&
    booking.completedAt !== null
  );
}
```

This example must be adapted to the application's real booking model. If completion depends on a verified chain event, the backend must verify that event rather than trusting a client assertion.

### 8\. Participant relationship validation

Only eligible booking participants may submit reviews.

The service must:

* Obtain the caller's identity from authenticated server context.

* Load participant records using the booking identifier.

* Verify that the caller matches an eligible participant.

* Determine the reviewee from the authoritative relationship.

* Derive the review perspective on the server.

* Reject unrelated accounts.

* Prevent clients from choosing their own reviewer or reviewee identity.

* Enforce the same authorization policy on all review creation endpoints.

Example:

```typescript
function isEligibleParticipant(
  userId: string,
  booking: Booking
): boolean {
  return (
    booking.renterUserId === userId ||
    booking.hostUserId === userId
  );
}
```

If the system supports co-renters, organizations, or multiple hosts, replace this simplified example with the canonical participant relationship model.

### 9\. Review window validation

The application must define the review window as a product policy.

Recommended rules:

* The window opens at the verified completion time.

* The window closes after a configured duration.

* The server clock determines eligibility.

* Client device time must not be trusted.

* The API returns a stable error code for reviews submitted too early or too late.

* Changes to the review window must be documented and tested.

Example:

```typescript
interface ReviewWindowPolicy {
  durationDays: number;
}

function isWithinReviewWindow(
  completedAt: Date,
  now: Date,
  policy: ReviewWindowPolicy
): boolean {
  const opensAt = completedAt.getTime();

  const closesAt =
    opensAt +
    policy.durationDays * 24 * 60 * 60 * 1000;

  return (
    now.getTime() >= opensAt &&
    now.getTime() <= closesAt
  );
}
```

The exact closing-boundary behavior must be explicitly documented. The example uses an inclusive closing boundary.

### 10\. One-review-per-perspective rule

The application must define what constitutes a unique review.

For example:

* A renter may review the host once.

* A host may review the renter once.

* A second submission for the same permitted perspective must not create another row.

The uniqueness key should reflect the actual product policy.

If reviews are unique per booking, reviewer, and perspective, use:

```sql
CREATE UNIQUE INDEX
  reviews_one_per_booking_reviewer_perspective
ON reviews (
  booking_id,
  reviewer_user_id,
  perspective
);
```

If a participant may submit only one review total per booking regardless of perspective, use a different uniqueness key.

The final schema must match the agreed product rules.

### Why a database constraint is mandatory

A service-layer check alone is insufficient.

Two concurrent requests can both check for an existing review before either request inserts one. Both may then attempt to create a review.

The unique constraint prevents both inserts from succeeding.

The service should catch the relevant uniqueness violation and return a stable error such as:

```json
{
  "error": {
    "code": "REVIEW_ALREADY_EXISTS",
    "message": "A review has already been submitted for this booking perspective."
  }
}
```

Do not expose raw database errors to the client.

### 11\. Refund eligibility rules

Refunds must have explicit eligibility rules. A refund does not automatically prove that a rental did or did not occur.

Recommended policy matrix:

| Booking situation                                  | Recommended review eligibility                               |
| -------------------------------------------------- | ------------------------------------------------------------ |
| Fully refunded before handover                     | Ineligible                                                   |
| Partially refunded after completed rental          | Eligible if policy permits                                   |
| Refunded because of a disputed rental              | Follow dispute policy                                        |
| Refund pending                                     | Apply the documented pending-refund rule                     |
| Completed rental with a later compensation payment | Normally retain eligibility if the rental genuinely occurred |

Implementation requirements:

* Load refund status from authoritative payment records.

* Verify that the refund belongs to the booking.

* Store the refund reason and completion timestamp.

* Define what happens if a refund is finalized after a review is published.

* Prevent a refund status supplied by the client from changing eligibility.

* Ensure API and database enforcement remain consistent.

### 12\. Cancellation eligibility rules

Recommended default: cancelled bookings that never became completed rentals cannot receive rental-quality reviews.

The implementation must distinguish:

* Cancellation before the rental starts.

* Cancellation after payment but before handover.

* Cancellation after partial use.

* Cancellation after the rental was completed.

* Cancellation associated with a dispute.

If the platform wants users to rate cancellation experiences, implement a separate feedback category. Do not mix cancellation feedback with completed-rental ratings without an explicit product decision.

### 13\. Modified booking eligibility

Changes to booking dates, prices, properties, or participants must not silently reset review eligibility.

The service must:

* Read the current authoritative booking record.

* Maintain an audit trail for material modifications.

* Define which participants remain eligible after a participant change.

* Use the correct completion timestamp.

* Avoid creating a new booking identity merely to bypass review uniqueness.

* Use version checks or transaction locking when modifications can race with review submission.

* Document how a completed booking is handled if someone attempts to modify it.

### 14\. Disputed booking eligibility

A dispute may affect review eligibility, publication, or moderation. These are separate decisions and should be modeled separately.

Recommended implementation:

* Represent dispute state using structured fields or related records.

* Record when a dispute starts and ends.

* Record the resolution outcome.

* Decide whether pending disputes block new reviews.

* Decide whether published reviews remain visible during a dispute.

* Restrict administrative exceptions to authorized staff.

* Record who approved an exception and why.

* Do not let a dispute create review privileges for unrelated accounts.

### 15\. Suggested database schema

Adapt these examples to the existing database engine and migration conventions.

### 15.1 Bookings

Suggested fields:

```sql
CREATE TABLE bookings (
  id UUID PRIMARY KEY,
  renter_user_id UUID NOT NULL,
  host_user_id UUID NOT NULL,
  status VARCHAR(32) NOT NULL,
  completed_at TIMESTAMP NULL,
  cancelled_at TIMESTAMP NULL,
  refund_status VARCHAR(32) NOT NULL DEFAULT 'NONE',
  dispute_status VARCHAR(32) NOT NULL DEFAULT 'NONE',
  version INTEGER NOT NULL DEFAULT 1,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

This is illustrative, not a replacement for the existing schema. Add appropriate foreign keys, indexes, lifecycle constraints, and audit tables.

### 15.2 Reviews

```sql
CREATE TABLE reviews (
  id UUID PRIMARY KEY,
  booking_id UUID NOT NULL REFERENCES bookings(id),
  reviewer_user_id UUID NOT NULL,
  reviewee_user_id UUID NOT NULL,
  perspective VARCHAR(32) NOT NULL,
  rating INTEGER NOT NULL,
  body TEXT NOT NULL,
  eligibility_policy_version VARCHAR(32) NOT NULL,
  submitted_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT reviews_rating_range
    CHECK (rating BETWEEN 1 AND 5)
);
```

Add user foreign keys if the actual schema supports them. Choose the rating range according to the product's established rating scale.

Add the uniqueness constraint from Section 10.

### 16\. Review request validation

Example request:

```http
POST /api/bookings/{bookingId}/reviews
Authorization: Bearer <session-token>
Idempotency-Key: <stable-request-key>
Content-Type: application/json
```

Example body:

```json
{
  "rating": 5,
  "body": "The property was clean and the handover was smooth."
}
```

The request must not accept trusted authority fields such as:

* `reviewerId`

* `revieweeId`

* `bookingStatus`

* `completedAt`

* `submittedAt`

* `moderationStatus`

The server must derive those values from authentication, booking records, and server time.

### 17\. Review API error codes

Define consistent machine-readable errors.

| Error code                     | Meaning                                       |
| ------------------------------ | --------------------------------------------- |
| AUTHENTICATION_REQUIRED        | Caller is not authenticated                   |
| BOOKING_NOT_FOUND              | Booking does not exist or cannot be disclosed |
| REVIEW_BOOKING_NOT_COMPLETED   | Rental is not completed                       |
| REVIEWER_NOT_A_PARTICIPANT     | Caller is not an eligible participant         |
| REVIEW_PERSPECTIVE_NOT_ALLOWED | Requested perspective is not permitted        |
| REVIEW_WINDOW_NOT_OPEN         | Review window has not started                 |
| REVIEW_WINDOW_EXPIRED          | Review window has closed                      |
| REVIEW_BLOCKED_BY_DISPUTE      | Current dispute policy blocks submission      |
| REVIEW_INELIGIBLE_AFTER_REFUND | Refund policy blocks submission               |
| REVIEW_ALREADY_EXISTS          | Unique review already exists                  |
| REVIEW_VALIDATION_FAILED       | Payload is invalid                            |
| RETRYABLE_SERVICE_UNAVAILABLE  | A required service is temporarily unavailable |

Return appropriate HTTP status codes and avoid disclosing private booking details to unauthorized callers.

### 18\. Review service implementation

The following is framework-neutral TypeScript-style pseudocode. Adapt it to the project's actual ORM and transaction API.

```typescript
async function submitReview(
  authenticatedUser: AuthenticatedUser,
  bookingId: string,
  payload: ReviewPayload,
  idempotencyKey: string
): Promise<ReviewResponse> {
  validateReviewPayload(payload);

  if (!authenticatedUser?.id) {
    throw new ApiError("AUTHENTICATION_REQUIRED");
  }

  return database.transaction(async (tx) => {
    const booking = await tx.bookings.findForUpdate(bookingId);

    if (!booking) {
      throw new ApiError("BOOKING_NOT_FOUND");
    }

    const participants =
      await tx.participants.findForBooking(booking.id);

    const eligibility = evaluateReviewEligibility({
      booking,
      participants,
      reviewerId: authenticatedUser.id,
      now: new Date()
    });

    if (!eligibility.eligible) {
      throw new ApiError(eligibility.reasonCode);
    }

    const existing = await tx.reviews.findByIdempotencyKey({
      bookingId,
      reviewerId: authenticatedUser.id,
      idempotencyKey
    });

    if (existing) {
      return existing;
    }

    return tx.reviews.insert({
      bookingId: booking.id,
      reviewerUserId: authenticatedUser.id,
      revieweeUserId: eligibility.revieweeId,
      perspective: eligibility.perspective,
      rating: payload.rating,
      body: payload.body,
      eligibilityPolicyVersion: eligibility.policyVersion
    });
  });
}
```

Important implementation notes:

* `findForUpdate` is illustrative; use the locking mechanism supported by the actual database.

* `findByIdempotencyKey` must use a properly scoped key if idempotent replay is required.

* The unique review constraint remains mandatory even if the service checks for existing reviews.

* Catch and translate database uniqueness violations at the API boundary.

* Avoid using the application server's clock if a trusted database-time strategy is required by the existing architecture.

* Ensure a failed transaction never returns a false success response.

### 19\. Shared review eligibility function

Use one policy module rather than duplicating eligibility rules across API handlers.

Example result:

```typescript
interface ReviewEligibility {
  eligible: boolean;
  reasonCode: string | null;
  revieweeId: string | null;
  perspective: string | null;
  policyVersion: string;
}
```

The module must:

* Accept authoritative booking and participant records.

* Accept a controlled evaluation timestamp.

* Check completion.

* Check participant identity.

* Check review window.

* Apply cancellation policy.

* Apply refund policy.

* Apply modification policy.

* Apply dispute policy.

* Derive the reviewee and perspective.

* Return a stable reason code.

* Include the policy version.

* Avoid network calls where possible so the function can be unit-tested deterministically.

### 20\. Review migration plan

* Inspect the existing booking and review schema.

* Identify every code path that creates reviews.

* Audit existing duplicate reviews.

* Generate a report of conflicting rows.

* Agree on historical duplicate handling with product and support.

* Do not silently delete user-generated content.

* Backfill missing fields only when authoritative evidence exists.

* Deploy code that writes the required fields.

* Backfill existing records.

* Validate the backfill.

* Add the unique index using a production-safe migration strategy.

* Deploy code that relies on the constraint.

* Monitor duplicate conflicts and eligibility rejections.

* Document rollback or forward-fix procedures.

For large tables, determine whether concurrent index creation or an equivalent online migration is supported by the database engine.

### 21\. Review security controls

* Authenticate every review mutation.

* Derive user identity from trusted authentication context.

* Verify booking participation on the server.

* Enforce rating and text limits on the server.

* Prevent mass assignment of identity and lifecycle fields.

* Sanitize content when displayed.

* Apply rate limits to repeated submission attempts.

* Restrict moderation and override privileges.

* Protect booking lookup endpoints against unauthorized enumeration.

* Audit administrative actions.

* Protect cookie-authenticated mutation endpoints against CSRF where applicable.

* Avoid logging access tokens, signing keys, or unnecessary personal data.

* Do not rely on IP address alone to establish whether two accounts are related.

### 22\. Review observability

Emit structured events such as:

```text
review_submission_requested
review_eligibility_evaluated
review_submission_accepted
review_submission_rejected
review_duplicate_conflict
review_admin_override
```

Recommended fields:

* Request ID.

* Booking ID.

* Authenticated user ID in secured logs.

* Eligibility reason code.

* Policy version.

* Outcome.

* Duration.

* Database constraint name, when relevant.

* Application version.

Track metrics for:

* Accepted submissions.

* Rejections by reason.

* Duplicate conflicts.

* Database insertion failures.

* Eligibility evaluation latency.

* Historical duplicates awaiting resolution.

* Administrative overrides.

Never log credentials or private signing material.

### Part B — RPC and Horizon outage resilience

### 23\. Objectives

The application must remain reliable when blockchain infrastructure is slow, partially unavailable, or returning inconsistent information.

Required capabilities:

* Provider health monitoring.

* Circuit breakers.

* Safe cached reads.

* Feature-specific restrictions.

* Durable pending operation records.

* Idempotent submission handling.

* Transaction reconciliation.

* Recovery controls.

* User-facing degradation indicators.

* Operational monitoring and alerts.

### 24\. Dependency health states

Use explicit states for each provider and capability.

| State       | Meaning                                               |
| ----------- | ----------------------------------------------------- |
| HEALTHY     | Required operations are working within defined limits |
| DEGRADED    | Errors or latency exceed normal thresholds            |
| OPEN        | Circuit breaker is blocking ordinary requests         |
| HALF_OPEN   | Limited probes are testing recovery                   |
| UNKNOWN     | Health information is missing or stale                |
| MAINTENANCE | Provider is intentionally disabled                    |

Health records should contain:

* Provider identifier.

* Network or environment.

* Capability being monitored.

* Current health state.

* Last successful request.

* Last failed request.

* Consecutive failures.

* Recent latency measurements.

* Last error category.

* Circuit transition timestamp.

* Probe result.

* Configuration version.

A successful connection alone does not establish that a provider's required API operations are healthy.

### 25\. Provider health monitoring

Implement lightweight probes for RPC and Horizon separately.

The monitoring system must:

* Set strict probe timeouts.

* Validate returned responses.

* Measure latency.

* Classify failures.

* Detect stale health information.

* Avoid probes that submit financial transactions.

* Track read success separately from broadcast success.

* Monitor Horizon indexing lag where relevant.

* Record provider-specific failure rates.

* Expose state changes through metrics and logs.

* Avoid polling so aggressively that monitoring worsens the outage.

Health checks must test the capabilities the application actually depends on.

### 26\. Circuit breaker implementation

A circuit breaker protects the application from repeated calls to an unhealthy dependency.

### Closed state

Requests are allowed. Failures are recorded against the configured threshold.

### Open state

Ordinary requests are rejected quickly without repeatedly contacting the unhealthy provider.

### Half-open state

A limited number of probes are permitted to determine whether the provider has recovered.

Implementation requirements:

* Configure thresholds independently for each provider and capability.

* Define the measurement window and cooldown.

* Limit concurrent half-open probes.

* Close the circuit after meaningful successful probes.

* Exclude ordinary business validation errors from provider failure counts.

* Prevent one provider's outage from disabling unrelated services.

* Record all circuit transitions.

* Use jitter for scheduled retries.

* Combine circuit breakers with request deadlines and bounded retries.

Example interface:

```typescript
type DependencyHealth =
  | "HEALTHY"
  | "DEGRADED"
  | "OPEN"
  | "HALF_OPEN"
  | "UNKNOWN"
  | "MAINTENANCE";

interface CircuitBreaker {
  getState(): DependencyHealth;

  canExecute(): boolean;

  recordSuccess(): void;

  recordFailure(error: ProviderError): void;

  probeRecovery(): Promise<boolean>;
}
```

This is a design interface. Its implementation must use the project's actual concurrency and runtime model.

### 27\. Classify every application action

Create an explicit dependency policy for each feature.

### 27.1 Safe read-only actions

These may remain available during an RPC outage if their data source is healthy and their freshness requirements permit it:

* Public property descriptions.

* Static property images.

* Help pages and documentation.

* Public product information.

* Verified historical transaction records.

* Previously confirmed booking information, subject to authorization and freshness rules.

* Application preferences stored in a healthy application database.

### 27.2 Live-state-dependent reads

These require current chain information or an explicitly marked stale fallback:

* Current account balance.

* Current sequence or nonce.

* Current network fee information.

* Latest ledger state.

* Current transaction status when no reliable local state exists.

* Data required to validate a new transaction.

Do not present stale data as current.

### 27.3 High-risk mutations

These generally must be blocked when the application cannot establish safe execution:

* Payments and transfers.

* Signed transaction broadcasts.

* Asset issuance or modification.

* Ownership changes.

* Other financial or irreversible chain operations.

The exact policy depends on the chain protocol and the guarantees provided by the transaction submission mechanism.

### 28\. Feature-specific restriction matrix

Implement a configuration or policy table that describes each action.

| Action                             | Dependency unavailable | Expected behavior                                            |
| ---------------------------------- | ---------------------- | ------------------------------------------------------------ |
| Public property browsing           | RPC unavailable        | Remain available if local data source is healthy             |
| Review submission                  | RPC unavailable        | Continue if eligibility data and database are healthy        |
| Wallet balance display             | RPC unavailable        | Show an appropriately timestamped cached value if permitted  |
| Payment submission                 | RPC unavailable        | Block or retain a safely tracked pending operation           |
| Transaction history                | Horizon unavailable    | Serve verified local history or an explicitly stale result   |
| Profile preference update          | RPC unavailable        | Continue if its own dependencies are healthy                 |
| Chain-dependent booking settlement | RPC unavailable        | Wait for authoritative verification                          |
| Transaction status lookup          | Horizon unavailable    | Use a supported alternative or clearly report unknown status |

The policy must be based on actual dependencies, not broad assumptions about feature names.

### 29\. Safe cached reads

Caching is a fallback for approved read-only data, not a substitute for authoritative transaction validation.

Every cached result should have:

* Source identifier.

* Fetch timestamp.

* Data version or ledger reference where applicable.

* Maximum allowed age.

* Clear freshness status.

Requirements:

* Define cache TTLs per data category.

* Preserve user-specific access control.

* Prevent cache keys from crossing accounts or tenants.

* Invalidate or refresh entries after confirmed mutations.

* Mark stale responses in the API and UI.

* Do not use stale balances to authorize payments.

* Do not use cached booking records to bypass current review eligibility.

* Return a controlled error when stale data could mislead the user.

* Measure cache age and stale-response rates.

* Prevent cached errors from persisting longer than intended.

Example:

```typescript
interface CachedRead<T> {
  data: T;
  source: string;
  fetchedAt: string;
  stale: boolean;
  maxAgeSeconds: number;
}
```

The API must calculate freshness from a trusted clock and the configured maximum age.

### 30\. Durable pending operation records

Every potentially ambiguous financial or irreversible action must have a durable identity.

Suggested fields:

```typescript
interface ChainOperation {
  operationId: string;
  userId: string;
  operationType: string;
  idempotencyKey: string;
  payloadHash: string;
  transactionHash: string | null;
  network: string;
  provider: string | null;
  status: OperationStatus;
  attemptCount: number;
  reconciliationCount: number;
  createdAt: string;
  updatedAt: string;
  submittedAt: string | null;
  confirmedAt: string | null;
  nextReconciliationAt: string | null;
  lastErrorCode: string | null;
  version: number;
}
```

Do not store private signing keys in operation records. Signed transaction payloads, if retained, require explicit security and retention controls.

### 31\. Operation state machine

Use explicit operation states:

| State           | Meaning                                               |
| --------------- | ----------------------------------------------------- |
| CREATED         | Operation intent has been persisted                   |
| VALIDATED       | Validation has passed                                 |
| READY_TO_SUBMIT | Submission preconditions are satisfied                |
| SUBMITTING      | A broadcast attempt is in progress                    |
| SUBMITTED       | Provider accepted submission; finality may be pending |
| CONFIRMED       | Authoritative evidence confirms success               |
| FAILED_FINAL    | Authoritative evidence establishes terminal failure   |
| UNKNOWN         | Outcome is ambiguous                                  |
| RECONCILING     | The system is checking authoritative state            |
| RETRYABLE       | Evidence establishes that a retry is safe             |
| CANCELLED       | Operation was cancelled under a safe rule             |
| NEEDS_REVIEW    | Automated reconciliation cannot safely decide         |

Do not treat `UNKNOWN` as equivalent to `FAILED_FINAL`.

Do not permit the frontend to set operation states directly.

### 32\. Idempotency requirements

For each financial or irreversible operation:

* Require or generate a stable idempotency key.

* Scope the key to the authenticated user and operation type.

* Compute a canonical payload hash.

* Persist the operation record before broadcasting.

* Enforce uniqueness for the idempotency scope.

* Return the existing operation for a repeated request with the same key and payload.

* Reject reuse of the same key with a different payload.

* Keep the key stable when checking the status of the original attempt.

* Record actual submission attempts separately from client requests.

* Define retention and expiration behavior.

* Prevent concurrent workers from processing the same operation.

* Ensure status polling never repeats the side effect.

Disabling a frontend button is helpful, but it is not an idempotency guarantee.

### 33\. Handling ambiguous broadcasts

A timeout may occur before a provider receives a request, after it accepts a request, or while its response is returning.

The system must not assume which situation occurred without evidence.

Required procedure:

* Persist the operation and idempotency key.

* Validate the request and network configuration.

* Submit the transaction through the controlled broadcast path.

* If a definitive response arrives, record the evidence.

* If the result is ambiguous, mark the operation `UNKNOWN`.

* Do not create a new logical operation.

* Do not immediately rebroadcast using a new idempotency key.

* Query by transaction hash where supported.

* Check the authoritative provider or Horizon state.

* Account for indexing lag and temporary provider inconsistency.

* Retry status reconciliation with bounded backoff.

* Mark the operation confirmed only when the required confirmation evidence exists.

* Mark it retryable only when evidence establishes that retrying is safe.

* Leave unresolved cases in `UNKNOWN` or move them to `NEEDS_REVIEW`.

* Tell the user not to submit the payment again while the original outcome is being checked.

* Escalate operations that remain ambiguous beyond the defined threshold.

Critical rule: absence of an immediate response is not proof that the transaction failed.

### 34\. Example operation API

Example submission request:

```http
POST /api/operations
Authorization: Bearer <session-token>
Idempotency-Key: <stable-operation-key>
Content-Type: application/json
```

Example ambiguous response:

```json
{
  "operationId": "operation-id",
  "status": "UNKNOWN",
  "message": "We are checking the transaction status. Do not submit it again yet.",
  "retryAllowed": false,
  "statusUrl": "/api/operations/operation-id"
}
```

The operation status endpoint must:

* Authenticate the caller.

* Authorize access to the operation.

* Return the current persisted state.

* Avoid initiating another broadcast.

* Return a stable operation ID.

* Provide a correlation or support reference.

* Avoid exposing private provider credentials or internal stack traces.

### 35\. Transaction construction and signing

Before broadcast:

* Validate the requested operation.

* Verify the target network.

* Validate destination and amount where applicable.

* Obtain current sequence, nonce, or fee data when required.

* Apply the correct signing policy.

* Protect signing keys and signed payloads.

* Record the operation identity.

* Record the transaction hash when available.

* Avoid changing a signed transaction after signing.

* Do not rebuild and broadcast a replacement until the previous attempt's execution risk has been evaluated.

* Distinguish the application operation ID from the blockchain transaction hash.

* Ensure production and test networks cannot be confused.

Signing should be isolated from general web request handling where appropriate for the architecture.

### 36\. Horizon resilience

The Horizon integration must distinguish API availability from transaction indexing freshness.

Requirements:

* Normalize HTTP and application-level errors.

* Set deadlines for all requests.

* Validate returned response structures.

* Track indexing lag where relevant.

* Use bounded retries for transient read failures.

* Respect pagination and cursor semantics.

* Record the provider used for each status check.

* Account for indexing delays after a broadcast.

* Avoid interpreting one immediate not-found response as definitive failure.

* Use alternate providers only when their network identity and consistency are verified.

* Serve verified local historical data where the product policy permits it.

* Define what evidence constitutes final confirmation for the supported Stellar workflow.

### 37\. RPC resilience

The RPC client layer must:

* Normalize provider errors.

* Distinguish transport failures, timeouts, rate limits, authentication errors, invalid requests, and business rejections.

* Apply request deadlines.

* Retry safe reads with bounded exponential backoff and jitter.

* Avoid retries for invalid requests.

* Route mutations through the durable operation state machine.

* Avoid blind broadcasts to multiple providers.

* Verify provider network identity.

* Protect provider credentials.

* Track latency and error rates per method.

* Prevent synchronized retry storms during recovery.

* Ensure fallback providers cannot silently point to another network.

### 38\. Retry policy

### Read requests

* Use short deadlines.

* Retry only transient failures.

* Apply exponential backoff and jitter.

* Set a maximum number of attempts.

* Respect applicable rate-limit guidance.

* Use an approved fallback provider when supported.

* Use cached data only when its freshness policy permits it.

### Mutating requests

* Do not use generic HTTP retry middleware for broadcasts.

* Preserve the original operation identity.

* Reconcile ambiguous outcomes before retrying.

* Require authoritative evidence that a retry is safe.

* Prevent concurrent workers from processing the same operation.

* Record every attempt.

* Stop automatic retries when the outcome remains uncertain.

### 39\. Background reconciliation worker

The worker should:

* Find due operations in `UNKNOWN`, `SUBMITTED`, or `RECONCILING`.

* Acquire a durable lease or lock.

* Check the transaction using its known hash or supported chain identifier.

* Respect provider health and circuit state.

* Use bounded request deadlines.

* Record evidence and observation timestamps.

* Account for indexing lag.

* Apply documented confirmation thresholds.

* Transition the operation only when evidence supports the decision.

* Move unresolved cases to manual review when necessary.

* Release the lease safely.

* Emit metrics and structured events.

* Recover correctly after worker crashes.

* Avoid creating a second logical operation.

* Ensure repeated processing is harmless.

The worker must not mark a transaction failed solely because a provider temporarily cannot find it.

### 40\. Database concurrency for chain operations

The database must prevent multiple requests or workers from independently initiating the same logical operation.

Required protections:

* A unique constraint for the idempotency scope.

* Conditional updates for state transitions.

* Version checks for concurrent updates.

* Durable operation persistence before broadcasting.

* Worker leases or equivalent ownership controls.

* An outbox pattern where needed to reliably schedule asynchronous work.

* Idempotent event processing.

* Immutable attempt history when required for auditability.

* Protection against late provider responses overwriting newer confirmed states.

Do not hold long-running database transactions open while waiting for a provider unless the architecture has a strong, documented reason.

### 41\. Recovery behavior

When a provider recovers:

* Verify health using controlled probes.

* Move the circuit to half-open according to policy.

* Confirm that the required provider capability is functioning.

* Resume safe read operations.

* Reconcile pending and ambiguous operations.

* Keep unsafe retries disabled until their outcomes are known.

* Drain background queues gradually.

* Apply backpressure if the provider remains unstable.

* Monitor failure rates and transaction confirmation latency.

* Restore normal mutation availability only when the relevant safety conditions are satisfied.

Recovery must not automatically replay every request that failed during the outage.

### 42\. User interface requirements

The UI must communicate the actual state of the affected feature.

Requirements:

* Show a degradation notice when a relevant dependency is unhealthy.

* Identify the affected functionality.

* Display a timestamp for stale data.

* Disable unsafe controls while their required state is unavailable.

* Preserve user-entered form content after recoverable errors.

* Prevent repeated submissions while a request is in progress.

* Distinguish pending from confirmed operations.

* Distinguish unknown outcomes from definitive failures.

* Avoid offering a retry button when retry is unsafe.

* Poll operation status with bounded intervals and backoff.

* Provide a support reference for unresolved transactions.

* Keep unrelated product pages usable.

* Ensure degradation notices are accessible.

* Never expose raw stack traces or provider credentials.

Example user-facing messages:

* Read-only degradation: “Some information may be out of date. Last updated at 10:15.”

* Pending operation: “Your transaction is being processed.”

* Unknown outcome: “We are verifying your transaction. Please do not submit it again.”

* Confirmed operation: “Your transaction has been confirmed.”

* Safe retry: “We confirmed that the previous attempt was not accepted. You can safely try again.”

Only display the final message when the corresponding state is supported by authoritative evidence.

### 43\. Operations dashboard

The dashboard should display:

* RPC health by provider and method.

* Horizon health and indexing lag.

* Circuit breaker states.

* Provider response latency.

* Error rates by category.

* Pending operation counts.

* Unknown operation counts.

* Age of the oldest unknown operation.

* Reconciliation success and failure rates.

* Queue depth and worker lag.

* Broadcast attempt counts.

* Stale-cache usage.

* Database availability.

* Review eligibility rejections.

* Duplicate review conflicts.

* Administrative overrides.

Alerts should include the affected dependency, environment, start time, current state, and the applicable operational procedure.

### 44\. Configuration

Configuration should be validated at application startup and documented.

Suggested values include:

```typescript
interface ReliabilityConfig {
  reviewWindowDays: number;
  maximumReviewBodyLength: number;
  minimumRating: number;
  maximumRating: number;

  providerRequestTimeoutMs: number;
  healthCheckIntervalMs: number;
  circuitFailureThreshold: number;
  circuitCooldownMs: number;
  halfOpenProbeLimit: number;

  retryBaseDelayMs: number;
  retryMaximumDelayMs: number;
  maximumReadRetries: number;

  reconciliationIntervalMs: number;
  maximumAutomaticReconciliationAgeMs: number;
  maximumStaleReadAgeSeconds: number;
}
```

These values are illustrative. Select production values using product policy, provider documentation, measured latency, and operational requirements.

### 45\. Security and privacy

The implementation must:

* Restrict access to booking participant records.

* Protect review moderation and operation dashboards with role-based authorization.

* Protect user-specific cache entries.

* Avoid exposing sensitive eligibility details to unrelated accounts.

* Redact credentials and tokens from logs.

* Protect transaction hashes and user associations appropriately.

* Define retention for operation records and signed payloads.

* Audit administrative state changes.

* Ensure support tools follow the same access-control rules as the main application.

* Prevent clients from modifying operation states.

* Validate all external provider responses.

* Avoid logging full sensitive payloads unnecessarily.

### 46\. Performance requirements

* Index common review and booking lookups.

* Keep eligibility queries bounded.

* Avoid unnecessary database queries for participant relationships.

* Keep review transactions short.

* Avoid database locks during long provider calls.

* Bound the number of concurrent reconciliation jobs.

* Use backpressure during recovery.

* Apply deadlines to every external request.

* Monitor tail latency as well as averages.

* Prevent cache fallback from overwhelming the database.

* Test realistic request spikes and retry storms.

* Ensure operational dashboards do not run unbounded queries.

### 47\. Suggested project structure

Adapt this structure to the existing application instead of creating unnecessary duplicate layers.

```text
src/
  reviews/
    review.controller.ts
    review.service.ts
    review.eligibility.ts
    review.repository.ts
    review.validation.ts
    review.errors.ts
    review.types.ts

  dependencies/
    provider-health.ts
    circuit-breaker.ts
    provider-client.ts
    dependency-policy.ts

  operations/
    operation.controller.ts
    operation.service.ts
    operation.repository.ts
    operation.state-machine.ts
    operation.idempotency.ts
    operation.reconciliation.ts

  workers/
    reconciliation.worker.ts

  observability/
    metrics.ts
    tracing.ts
    structured-logging.ts

  config/
    validation.ts

migrations/
tests/
  unit/
  integration/
  concurrency/
  fault-injection/

docs/
  runbooks/
```

The filenames are suggestions, not assumptions about the project's existing language or framework.

### Part C — Testing and acceptance criteria

### 48\. Review unit tests

The test suite must verify:

* A completed booking permits an eligible renter to review.

* A completed booking permits an eligible host to review the permitted perspective.

* An unrelated account is rejected.

* A missing booking is rejected safely.

* An incomplete booking is rejected.

* A review before the opening time is rejected.

* A review after the closing time is rejected.

* The exact time boundary follows the documented rule.

* Cancelled bookings follow the cancellation policy.

* Fully refunded bookings follow the refund policy.

* Partially refunded bookings follow the refund policy.

* Modified participant records follow the modification policy.

* Pending disputes follow the dispute policy.

* Resolved disputes follow the dispute policy.

* Invalid ratings are rejected.

* Overlong review text is rejected.

* Client-supplied timestamps do not control eligibility.

* Client-supplied reviewer IDs do not control authorization.

* Stable eligibility reason codes are returned.

* The policy version is recorded correctly.

### 49\. Review integration tests

The integration suite must verify:

* Eligible review creation succeeds.

* An unrelated user cannot create a review.

* The database unique constraint rejects duplicate records.

* Unique violations map to a stable API error.

* Repeated requests do not create duplicate rows.

* Idempotency-key reuse with a different payload is rejected.

* Database failures do not produce false success responses.

* Concurrent booking modification and review creation follow policy.

* Existing duplicate data is detected during migration.

* Review retrieval respects authorization.

* All review creation paths invoke the shared eligibility logic.

* Database constraints and service decisions remain consistent.

### 50\. Review concurrency tests

Run concurrent requests against a real test database or an equivalent integration environment.

Required scenarios:

* Two identical submissions arrive simultaneously.

* Two requests use different idempotency keys for the same review perspective.

* Two devices submit the same review concurrently.

* Booking completion changes while a review is being submitted.

* Participant records change during eligibility evaluation.

* An administrative moderation action races with a submission.

* A transaction rolls back after eligibility checks.

* The unique constraint is triggered under load.

Acceptance condition: no test creates more than one eligible review for the defined uniqueness key.

### 51\. Health and circuit breaker tests

Verify:

* A provider timeout changes health according to configured thresholds.

* Repeated failures open the circuit.

* Open circuits fail quickly.

* Half-open probes are limited.

* Successful recovery probes restore service according to policy.

* Stale health information becomes `UNKNOWN`.

* Validation failures do not count as provider failures.

* RPC and Horizon health are tracked independently.

* A single provider outage does not disable unrelated dependencies.

* Circuit transitions are visible in metrics and logs.

### 52\. Safe read fallback tests

Verify:

* Public product reads remain available during an RPC outage.

* Approved cached data is served with freshness metadata.

* Sensitive stale values cannot authorize transactions.

* User-specific cache entries remain isolated.

* Cache misses for critical live data return a controlled error.

* Horizon indexing lag is not misreported as definitive transaction failure.

* Database-backed read-only features remain available when their own dependencies are healthy.

* Recovery does not trigger an uncontrolled cache refresh storm.

### 53\. Mutation and reconciliation tests

Verify:

* A successful submission is not repeated after a client retry.

* A lost provider response results in `UNKNOWN`.

* An unknown operation does not permit a blind retry.

* Reconciliation finding the transaction prevents a duplicate broadcast.

* Authoritative evidence of non-acceptance can enable a safe retry.

* Inconclusive evidence leaves the operation unknown.

* Repeated status polling causes no side effects.

* Worker crashes do not lose persisted operation records.

* Two workers cannot submit the same logical operation concurrently.

* Provider recovery triggers reconciliation before unsafe retries.

* Reused idempotency keys cannot create a second operation.

* Reusing a key with a different payload is rejected.

* Wrong-network provider configuration is detected.

* Provider acceptance is not confused with final confirmation.

### 54\. Fault-injection tests

Simulate:

* DNS failure.

* Connection timeout.

* Connection reset after request transmission.

* Provider acceptance followed by a lost response.

* Delayed Horizon indexing.

* Inconsistent responses from two providers.

* Database failure before operation persistence.

* Database failure after a broadcast.

* Worker termination during reconciliation.

* Rate limiting.

* Malformed provider responses.

* Provider recovery after a long outage.

* A large pending-operation backlog.

* A circuit breaker stuck open.

* A circuit breaker entering half-open state.

* A late response arriving after the operation is confirmed.

* Two concurrent idempotency-key claims.

* A stale cache entry exceeding its maximum permitted age.

The tests must confirm that failures do not cause duplicate financial actions or unauthorized reviews.

### 55\. Acceptance criteria: reviews

* Only authenticated, eligible participants can submit reviews.

* Booking completion is validated using authoritative records.

* Review windows are enforced using server-side time.

* Cancellation eligibility is explicitly defined.

* Refund eligibility is explicitly defined.

* Modified booking behavior is explicitly defined.

* Disputed booking behavior is explicitly defined.

* One-review-per-perspective is enforced by the database.

* Concurrent duplicate submissions cannot create duplicate rows.

* Duplicate attempts return safe, stable errors.

* Identity and booking state cannot be spoofed by the client.

* Existing data is audited before applying uniqueness constraints.

* Service and database decisions remain consistent.

* Automated unit, integration, and concurrency tests pass.

### 56\. Acceptance criteria: chain resilience

* RPC and Horizon health are measured independently.

* Circuit breakers are implemented and tested.

* Safe cached reads remain available under documented freshness rules.

* Unsafe mutations are restricted when required state is unavailable.

* Only proven-safe operations are queued.

* Mutations use durable operation records.

* Idempotency is enforced at the database boundary.

* Ambiguous outcomes are represented explicitly.

* An ambiguous timeout never triggers an uncontrolled duplicate broadcast.

* Reconciliation runs before unsafe retries are allowed.

* The UI distinguishes unknown, pending, failed, and confirmed states.

* Operations can monitor provider health and unresolved operations.

* Recovery and fault-injection tests pass.

* Unrelated read-only features remain available whenever their dependencies permit it.

### Part D — Deployment, monitoring, and maintenance

### 57\. Phase 1: Codebase discovery

Before changing code:

* Identify the framework, runtime, and database engine.

* Locate all booking and review creation endpoints.

* Locate all booking lifecycle transitions.

* Locate refund, cancellation, and dispute services.

* Locate RPC and Horizon clients.

* Find every transaction broadcast path.

* Identify existing retry middleware.

* Identify caching layers and background workers.

* Review existing migrations and constraints.

* Document the current transaction confirmation process.

* Identify the existing logging and metrics systems.

* Confirm the supported network and provider configurations.

Do not introduce duplicate services if equivalent functionality already exists.

### 58\. Phase 2: Review integrity implementation

Implement in this order:

* Document review eligibility policies.

* Create the shared eligibility function.

* Add server-side request validation.

* Update every review creation endpoint.

* Add stable eligibility error codes.

* Audit existing reviews for duplicates.

* Resolve historical conflicts under an approved policy.

* Add the unique database constraint.

* Add concurrency tests.

* Add structured logs and metrics.

* Verify all review creation paths use the same rules.

* Deploy and monitor rejection and duplicate-conflict rates.

### 59\. Phase 3: Chain resilience implementation

Implement in this order:

* Inventory RPC and Horizon dependencies.

* Normalize provider errors.

* Add dependency health instrumentation.

* Add circuit breakers.

* Classify application features by dependency.

* Define safe caching policies.

* Implement durable operation records.

* Add idempotency constraints.

* Implement explicit operation state transitions.

* Add the reconciliation worker.

* Remove unsafe generic broadcast retries.

* Add API operation-status responses.

* Update UI degradation messages.

* Add operations dashboards and alerts.

* Test ambiguous broadcasts and recovery.

Do not enable new retry behavior before durable operation tracking and reconciliation are ready.

### 60\. Phase 4: Staged rollout

Recommended release sequence:

* Deploy compatible database schema changes.

* Deploy review validation code.

* Enable review enforcement after confirming existing data is clean enough for the constraint.

* Deploy health telemetry in observation mode.

* Validate provider health measurements.

* Enable circuit breakers conservatively.

* Deploy durable operation tracking.

* Verify idempotency behavior.

* Enable reconciliation.

* Validate operation state transitions.

* Enable feature-specific degradation controls.

* Run recovery drills.

* Increase rollout gradually.

* Review operational metrics before full release.

Use an expand-and-contract migration strategy when required by the existing database and deployment architecture.

### 61\. Phase 5: Production verification

After deployment, confirm:

* Review creation works for eligible participants.

* Unrelated accounts cannot submit reviews.

* Duplicate review attempts are rejected.

* Database constraints are active.

* RPC and Horizon health indicators are accurate.

* Safe read-only features remain available during a simulated outage.

* Unsafe mutations are blocked appropriately.

* Ambiguous submissions remain pending until reconciled.

* No duplicate broadcast is caused by a timeout.

* Recovery does not replay operations blindly.

* Alerts and dashboards receive the expected events.

* Rollback or forward-fix procedures are documented.

### 62\. Runbook: RPC or Horizon outage

When a provider becomes unhealthy:

* Identify the affected provider and capability.

* Check health metrics and recent failures.

* Verify the current circuit breaker state.

* Confirm that unsafe mutations are restricted.

* Confirm that safe read-only features remain available.

* Inspect pending and unknown operations.

* Do not manually rebroadcast ambiguous transactions.

* Check whether an approved alternate provider is available.

* Verify network identity before using a fallback.

* Monitor recovery probes.

* Reconcile pending operations.

* Drain the backlog gradually.

* Check for unexpected duplicate attempts.

* Notify users about affected features.

* Record the incident and corrective actions.

### 63\. Runbook: unknown transaction

When a transaction has an ambiguous outcome:

* Retrieve the operation by its internal ID.

* Verify the initiating user and target network.

* Inspect the idempotency key and transaction hash.

* Review the recorded submission attempt.

* Check the authoritative transaction status.

* Account for indexing delay.

* Do not create a second logical operation.

* Reconcile against supported provider data.

* Mark the operation confirmed only when sufficient evidence exists.

* Mark it retryable only when a retry is demonstrably safe.

* Leave unresolved operations in `UNKNOWN` or `NEEDS_REVIEW`.

* Provide the user with a support reference.

* Escalate cases exceeding the defined resolution threshold.

* Audit manual intervention.

### 64\. Runbook: review eligibility dispute

When a user reports that they cannot submit a review:

* Retrieve the authoritative booking record.

* Verify the completion timestamp and evidence.

* Verify the authenticated user's participant relationship.

* Inspect cancellation, refund, modification, and dispute history.

* Identify the eligibility policy version.

* Review the rejection reason code.

* Confirm whether a review already exists for the relevant perspective.

* Do not bypass the uniqueness constraint with a direct database insert.

* If an exception is authorized, use an audited administrative process.

* Record the resolution and add a regression test if policy changes.

### 65\. Maintenance and ongoing monitoring

Review the following regularly:

* Duplicate review conflicts.

* Unexpected eligibility rejections.

* Historical duplicate review records.

* Provider latency and error rates.

* Circuit breaker transitions.

* Stale cache usage.

* Unknown operation counts.

* Age of unresolved transactions.

* Reconciliation worker lag.

* Retry attempts per operation.

* Provider recovery behavior.

* Database constraint failures.

* Administrative overrides.

* Security and privacy audit findings.

Update thresholds based on measured behavior rather than arbitrary assumptions.

### 66\. Pull request checklist

Before merging a change, verify:

* The review policy is documented.

* All review endpoints use the shared eligibility service.

* Authenticated identity is authoritative.

* The uniqueness constraint matches the product rules.

* Expected database conflicts map to safe API errors.

* Concurrent submissions are tested.

* Refund, cancellation, modification, and dispute cases are tested.

* RPC and Horizon failures are classified consistently.

* Circuit breaker transitions are tested.

* Cached reads enforce freshness and access control.

* Mutations use durable operation records.

* Idempotency is enforced.

* Ambiguous transactions cannot be blindly retried.

* Reconciliation is safe under concurrent workers.

* UI status reflects actual operation state.

* Metrics, alerts, and runbooks are updated.

* Migrations and recovery procedures have been verified.

### 67\. Definition of done

The implementation is complete only when:

* All review eligibility rules are documented and enforced.

* Database constraints prevent duplicate eligible reviews.

* Race conditions are covered by automated tests.

* Refund, cancellation, modification, and dispute behavior is deterministic.

* Provider health is observable.

* Circuit breakers prevent repeated calls to unhealthy dependencies.

* Safe cached reads work within their documented limits.

* Unsafe mutations are restricted when state cannot be trusted.

* Every financial or irreversible operation has durable identity and idempotency protection.

* Ambiguous submissions are reconciled before unsafe retries.

* User-facing degradation states are accurate.

* Operations has the dashboards and runbooks needed to manage incidents.

* Acceptance criteria pass in a production-like environment.

### 68\. Additional test inventory

| Test ID | Scenario                                      | Expected result                                               |
| ------- | --------------------------------------------- | ------------------------------------------------------------- |
| REV-001 | Eligible renter submits a review              | One review is created                                         |
| REV-002 | Eligible host submits a review                | One permitted review is created                               |
| REV-003 | Unrelated user submits a review               | Request is rejected                                           |
| REV-004 | Booking is incomplete                         | Request is rejected                                           |
| REV-005 | Review window has not opened                  | Request is rejected                                           |
| REV-006 | Review window has expired                     | Request is rejected                                           |
| REV-007 | Two identical concurrent submissions          | At most one row exists                                        |
| REV-008 | Two devices submit the same review            | No duplicate row is created                                   |
| REV-009 | Booking is refunded                           | Configured refund policy is applied                           |
| REV-010 | Booking is cancelled                          | Configured cancellation policy is applied                     |
| REV-011 | Booking has a pending dispute                 | Configured dispute policy is applied                          |
| REV-012 | Client spoofs reviewer ID                     | Spoofed identity is ignored                                   |
| NET-001 | RPC health check times out                    | Health state changes according to policy                      |
| NET-002 | Circuit is open                               | Calls fail quickly                                            |
| NET-003 | Recovery probe succeeds                       | Circuit recovery follows policy                               |
| NET-004 | Safe read during outage                       | Approved cache or fallback is used                            |
| NET-005 | Sensitive live read during outage             | Stale data is not represented as current                      |
| NET-006 | Broadcast response is lost                    | Operation becomes UNKNOWN                                     |
| NET-007 | Transaction is found during reconciliation    | No duplicate operation is created                             |
| NET-008 | Transaction absence is inconclusive           | Operation remains UNKNOWN                                     |
| NET-009 | Non-acceptance is authoritatively established | Safe retry may be permitted                                   |
| NET-010 | Same key and same payload                     | Existing operation is returned                                |
| NET-011 | Same key and different payload                | Request is rejected                                           |
| NET-012 | Two workers claim one operation               | Only one worker owns processing                               |
| NET-013 | Horizon indexing is delayed                   | No premature failure is inferred                              |
| NET-014 | Provider recovers after outage                | Backlog drains in a controlled manner                         |
| NET-015 | Database fails before operation persistence   | No broadcast is attempted                                     |
| NET-016 | Database fails after broadcast                | Recovery follows the durable operation record                 |
| NET-017 | Provider returns malformed data               | Response is rejected safely                                   |
| NET-018 | Fallback provider targets the wrong network   | Fallback is refused                                           |
| NET-019 | Profile preference changes during RPC outage  | Feature remains available if its own dependencies are healthy |
| NET-020 | User polls operation status repeatedly        | No financial side effect is repeated                          |

### 69\. Final engineering principles

* Reviews: No authoritative booking completion and participant relationship means no review.

* Uniqueness: Application checks improve user experience; database constraints enforce the invariant under concurrency.

* Refunds and disputes: Eligibility must follow an explicit policy rather than an arbitrary status check.

* Resilience: One failing provider must not automatically disable unrelated features.

* Caching: Stale information must never be represented as authoritative current state.

* Idempotency: Repeated requests must not create duplicate logical financial operations.

* Ambiguous outcomes: A timeout does not prove that a transaction failed.

* Reconciliation: Unsafe retries remain disabled until the original outcome is sufficiently established.

* Observability: Users and operators must see accurate states.

* Recovery: The application must reconcile pending work before restoring unsafe actions.

### 70\. Final acceptance statement

The engineering team should approve the release only after the review integrity and chain resilience acceptance criteria have been demonstrated through automated tests, migration verification, fault injection, and operational recovery exercises.

The implementation must ensure that completed rental reviews remain trustworthy, unrelated accounts cannot manipulate property ratings, duplicate submissions are safely rejected, and RPC or Horizon outages do not trigger duplicate financial actions or unnecessarily disable unrelated read-only product functionality.

Implementation note: This README is a specification and illustrative implementation guide. The exact production code, database migrations, RPC behavior, and Horizon reconciliation logic must be adapted to the existing repository and its actual network integration.
