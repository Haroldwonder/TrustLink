# Attestation Requests Example

This example walks through the full attestation request lifecycle:

1. A **subject** calls `request_attestation` to ask an issuer for a claim.
2. The **issuer** lists its pending requests.
3. The issuer **fulfils** one request (issuing a claim) and **rejects** another.
4. The subject **verifies** the resulting claim.

It also demonstrates how requests **expire** and how **cleanup** removes them from the issuer's pending list.

## Lifecycle overview

```
subject                          issuer
   |                                |
   |-- request_attestation -------->|  (request stored as pending)
   |                                |
   |                                |-- list pending requests
   |                                |
   |                                |-- fulfill_request  -> claim issued
   |                                |-- reject_request   -> request removed
   |                                |-- cancel_request   -> request removed (by subject)
   |                                |
   |<------- verify claim ----------|
```

### Requesting

`request_attestation` creates a pending request on the issuer. Each request records:

- the **subject** address,
- the **claim type** being requested,
- an **`expires_at`** timestamp.

### Pending list

The issuer can list all outstanding requests with the pending-requests query. Only requests that are still pending (not fulfilled, rejected, cancelled, or expired) appear here.

### Fulfilling, rejecting, cancelling

- **`fulfill_request`** — the issuer approves the request and issues a claim to the subject. The request leaves the pending list.
- **`reject_request`** — the issuer declines the request. The request leaves the pending list and no claim is issued.
- **`cancel_request`** — the subject withdraws its own request. The request leaves the pending list.

Rejected and cancelled requests are removed from the issuer's pending list immediately, so they no longer show up in subsequent listings.

## Expiry

Every request carries an **`expires_at`** timestamp. Once the current ledger time passes `expires_at`, the request is considered expired and can no longer be fulfilled.

The default time-to-live for a request is controlled by the **`ATTESTATION_REQUEST_TTL_SECS`** environment variable (in seconds). If unset, the contract's default TTL is used. Set it before deploying/initialising the contract to change how long requests stay valid:

```bash
export ATTESTATION_REQUEST_TTL_SECS=86400   # 1 day
```

Expired requests are treated like stale entries: they cannot be fulfilled and should be cleaned up.

## Cleanup

`cleanup_expired_requests` removes expired entries from the issuer's pending list. Call it periodically (or before listing) so the pending list only contains actionable requests:

```
issuer: cleanup_expired_requests  ->  expired requests removed from pending list
```

After cleanup, listing pending requests returns only requests that are still within their TTL.

## Setup

From the repository root:

```bash
# Install dependencies
npm install

# Build the contracts (if not already built)
npm run build
```

Set the environment used by the script:

```bash
# Optional: override the request TTL (seconds)
export ATTESTATION_REQUEST_TTL_SECS=86400

# Point the script at your network / RPC as needed
export RPC_URL="http://localhost:8000"
export NETWORK_PASSPHRASE="Standalone Network ; February 2017"
```

## Running

```bash
npx ts-node examples/attestation-requests/index.ts
```

The script will:

1. Deploy/connect to the attestation contract.
2. Have the subject call `request_attestation` for a claim type.
3. Have the issuer list pending requests.
4. Fulfil one request and reject another.
5. Have the subject verify the issued claim.
6. Demonstrate expiry and `cleanup_expired_requests` (optional, depending on TTL).

## Notes

- Rejected and cancelled requests disappear from the pending list right away.
- Expired requests remain until `cleanup_expired_requests` is called.
- Keep `ATTESTATION_REQUEST_TTL_SECS` short (e.g. a few seconds) if you want to observe expiry during a local run.