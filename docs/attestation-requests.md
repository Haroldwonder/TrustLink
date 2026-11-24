# Attestation Request Lifecycle

This document describes how attestation requests move through the system, who is
allowed to trigger each transition, how expiry and cleanup work, what happens
while the contract is paused, and which events and error codes are involved.

## Overview

An attestation request is created by a requester and is later resolved by an
issuer (or by the requester themselves, or by the passage of time). Every
request is in exactly one state at any moment.

## States

```
                 fulfill (issuer)
        ┌──────────────────────────────► Fulfilled
        │
        │ reject (issuer)
        ├──────────────────────────────► Rejected
        │
Pending ├ cancel (requester)
        ├──────────────────────────────► Cancelled
        │
        │ expire (anyone, after TTL)
        └──────────────────────────────► Expired
```

`Pending` is the only non-terminal state. `Fulfilled`, `Rejected`, `Cancelled`,
and `Expired` are terminal: once a request leaves `Pending` it can never change
state again.

### Pending

The initial state. The request is open and awaiting a decision. While pending,
the request can be fulfilled, rejected, cancelled, or expired.

### Fulfilled

The issuer accepted the request and recorded an attestation. Triggered by the
**issuer** (the account authorized to issue attestations). This is the only
transition that produces an attestation.

### Rejected

The issuer declined the request. Triggered by the **issuer**. No attestation is
recorded.

### Cancelled

The requester withdrew their own request before it was resolved. Triggered by
the **requester** (the account that created the request). An issuer cannot
cancel a request on the requester's behalf, and a requester cannot cancel a
request that has already been resolved.

### Expired

The request outlived its time-to-live without being resolved. Triggered by
**anyone** — expiry is permissionless so that stale requests can always be
cleaned up. Expiry is only valid once the TTL has elapsed (see below).

### Who can trigger each transition

| Transition | Triggered by | Precondition |
|------------|--------------|--------------|
| Pending → Fulfilled | Issuer | Request is `Pending` and not expired |
| Pending → Rejected  | Issuer | Request is `Pending` and not expired |
| Pending → Cancelled | Requester | Request is `Pending` |
| Pending → Expired   | Anyone | Request is `Pending` and TTL has elapsed |

## Time-to-live and expiry

### `ATTESTATION_REQUEST_TTL_SECS`

`ATTESTATION_REQUEST_TTL_SECS` is the configured lifetime of a request, in
seconds. It is measured from the moment the request is created. A request
becomes eligible for expiry once `created_at + ATTESTATION_REQUEST_TTL_SECS`
has passed.

Before the TTL elapses, a request cannot be expired; attempts to do so fail
with `RequestExpired`-family validation (see error codes below). After the TTL
elapses, the request is still stored until it is explicitly expired or swept by
cleanup, but it can no longer be fulfilled or rejected.

### `cleanup_expired_requests`

`cleanup_expired_requests` is a permissionless maintenance entry point. It
scans stored requests and moves any `Pending` request whose TTL has elapsed to
`Expired`, emitting the corresponding event for each. It is safe to call at any
time and is idempotent with respect to already-terminal requests: requests that
are already `Fulfilled`, `Rejected`, `Cancelled`, or `Expired` are left
untouched.

Callers do not need special authorization to run cleanup; it only ever expires
requests that are already past their TTL.

## Pause behaviour

When the contract is paused:

- **Creating** new attestation requests is blocked.
- **Fulfilling** and **rejecting** requests are blocked.
- **Cancelling** a request is blocked.
- **Expiring** a request (directly or via `cleanup_expired_requests`) is
  blocked.

In short, all state-changing request operations are halted while paused. Reads
(querying request state) remain available. Operations attempted during a pause
fail with the contract's standard paused error. Once the contract is unpaused,
normal operation resumes; requests that aged past their TTL during the pause
can then be expired or cleaned up.

## Events

Each transition emits an event so off-chain indexers can track request state:

| Transition | Event |
|------------|-------|
| Request created | Request-created event (carries requester, issuer, and request id) |
| Pending → Fulfilled | Request-fulfilled event |
| Pending → Rejected | Request-rejected event |
| Pending → Cancelled | Request-cancelled event |
| Pending → Expired | Request-expired event (emitted per request by cleanup) |

Events include the request identifier so consumers can correlate them with the
originating request.

## Error codes

| Error | Meaning |
|-------|---------|
| `RequestAlreadyProcessed` | The request is already in a terminal state (`Fulfilled`, `Rejected`, `Cancelled`, or `Expired`) and cannot transition again. |
| `RequestExpired` | The request's TTL has elapsed; it can no longer be fulfilled or rejected and must be expired instead. |
| `DuplicateRequest` | An equivalent request already exists (e.g. same requester/issuer/subject), so a new one cannot be created. |
| Paused error | The contract is paused and the attempted state-changing operation is not allowed. |
| Unauthorized error | The caller is not permitted to perform the transition (e.g. a non-issuer trying to fulfill/reject, or a non-requester trying to cancel). |

## Summary

- Requests start `Pending` and end in exactly one of `Fulfilled`, `Rejected`,
  `Cancelled`, or `Expired`.
- Issuers fulfill/reject, requesters cancel, and anyone may expire after the
  TTL.
- `ATTESTATION_REQUEST_TTL_SECS` sets the lifetime; `cleanup_expired_requests`
  sweeps stale requests permissionlessly.
- Pausing halts all request state changes; reads still work.
- Every transition emits an event, and misuse surfaces as
  `RequestAlreadyProcessed`, `RequestExpired`, `DuplicateRequest`, or the
  paused/unauthorized errors.