# Monitoring TrustLink Contract Events

This guide explains how to stream real-time events from the TrustLink Soroban smart contract using Stellar Horizon, build a webhook handler, and set up alerting.

## Prerequisites

- A running Stellar node or access to a Horizon endpoint
  - **Local**: `http://localhost:8000` (see [CONTRIBUTING.md](../CONTRIBUTING.md) for local network setup)
  - **Testnet**: `https://horizon-testnet.stellar.org`
  - **Mainnet**: `https://horizon.stellar.org`
- The deployed TrustLink contract ID (stored in `.local.contract-id` after `make local-deploy`)
- Node.js 18+ (for the example webhook handler)

---

## 1. Horizon Event Streaming API

Stellar Horizon exposes a Server-Sent Events (SSE) endpoint that streams contract events in real time.

### Soroban RPC `getEvents`

The primary method for Soroban contract events is the JSON-RPC `getEvents` call against the Soroban RPC endpoint:

```bash
curl -s -X POST "$RPC_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getEvents",
    "params": {
      "startLedger": "'"$START_LEDGER"'",
      "filters": [
        {
          "type": "contract",
          "contractIds": ["'"$CONTRACT_ID"'"],
          "topics": [["*"]]
        }
      ],
      "pagination": { "limit": 100 }
    }
  }'
```

### Filtering by Event Topic

TrustLink events use a topic symbol as their first topic element. You can narrow the stream to specific event types:

| Filter goal          | `topics` value              |
| -------------------- | --------------------------- |
| All TrustLink events | `[["*"]]`                   |
| Attestation created  | `[["SymbolVal(created)"]]`  |
| Attestation revoked  | `[["SymbolVal(revoked)"]]`  |
| Issuer registered    | `[["SymbolVal(iss_reg)"]]`  |
| Admin transfers      | `[["SymbolVal(adm_xfer)"]]` |

Example — stream only `created` and `revoked` events:

```bash
curl -s -X POST "$RPC_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getEvents",
    "params": {
      "startLedger": "'"$START_LEDGER"'",
      "filters": [
        {
          "type": "contract",
          "contractIds": ["'"$CONTRACT_ID"'"],
          "topics": [["SymbolVal(created)"]]
        },
        {
          "type": "contract",
          "contractIds": ["'"$CONTRACT_ID"'"],
          "topics": [["SymbolVal(revoked)"]]
        }
      ],
      "pagination": { "limit": 100 }
    }
  }'
```

### Horizon SSE Streaming (Effects / Operations)

For Horizon-based streaming (useful for tracking payments, e.g. fee transfers), open a persistent SSE connection:

```
GET /accounts/{fee_collector}/effects?cursor=now&order=asc
Accept: text/event-stream
```

---

## 2. TrustLink Event Reference

### Attestation Lifecycle Events

#### `created`

Emitted when a registered issuer creates a new attestation.

| Field        | Type             | Description                          |
| ------------ | ---------------- | ------------------------------------ |
| `id`         | `String`         | Deterministic attestation ID         |
| `issuer`     | `Address`        | Issuer who created the attestation   |
| `claim_type` | `String`         | Claim identifier (e.g. `KYC_PASSED`) |
| `timestamp`  | `u64`            | Ledger timestamp at creation         |
| `metadata`   | `Option<String>` | Optional issuer-supplied metadata    |

**Topic**: `["created", <subject_address>]`

#### `imported`

Emitted when the admin imports a historical attestation.

| Field        | Type          | Description                   |
| ------------ | ------------- | ----------------------------- |
| `id`         | `String`      | Attestation ID                |
| `issuer`     | `Address`     | Original issuer               |
| `claim_type` | `String`      | Claim identifier              |
| `timestamp`  | `u64`         | Original historical timestamp |
| `expiration` | `Option<u64>` | Optional expiration time      |

**Topic**: `["imported", <subject_address>]`

#### `bridged`

Emitted when a bridge contract creates a cross-chain attestation.

| Field          | Type      | Description                    |
| -------------- | --------- | ------------------------------ |
| `id`           | `String`  | Attestation ID                 |
| `issuer`       | `Address` | Bridge contract address        |
| `claim_type`   | `String`  | Claim identifier               |
| `source_chain` | `String`  | Origin chain (e.g. `ethereum`) |
| `source_tx`    | `String`  | Source transaction reference   |

**Topic**: `["bridged", <subject_address>]`

#### `revoked`

Emitted when an issuer revokes an attestation.

| Field            | Type             | Description                |
| ---------------- | ---------------- | -------------------------- |
| `attestation_id` | `String`         | ID of revoked attestation  |
| `reason`         | `Option<String>` | Optional revocation reason |

**Topic**: `["revoked", <issuer_address>]`

#### `renewed`

Emitted when an issuer renews (extends) an attestation.

| Field            | Type          | Description                  |
| ---------------- | ------------- | ---------------------------- |
| `attestation_id` | `String`      | Attestation ID               |
| `new_expiration` | `Option<u64>` | Updated expiration timestamp |

**Topic**: `["renewed", <issuer_address>]`

#### `updated`

Emitted when attestation metadata or expiration is updated.

| Field            | Type          | Description                  |
| ---------------- | ------------- | ---------------------------- |
| `attestation_id` | `String`      | Attestation ID               |
| `new_expiration` | `Option<u64>` | Updated expiration timestamp |

**Topic**: `["updated", <issuer_address>]`

#### `expired`

Emitted when an attestation is detected as expired during a query.

| Field            | Type     | Description    |
| ---------------- | -------- | -------------- |
| `attestation_id` | `String` | Attestation ID |

**Topic**: `["expired", <subject_address>]`

#### `endorsed`

Emitted when another issuer endorses an existing attestation.

| Field            | Type     | Description           |
| ---------------- | -------- | --------------------- |
| `attestation_id` | `String` | Attestation ID        |
| `timestamp`      | `u64`    | Endorsement timestamp |

**Topic**: `["endorsed", <endorser_address>]`

### Issuer Management Events

#### `iss_reg`

| Field       | Type      | Description                     |
| ----------- | --------- | ------------------------------- |
| `admin`     | `Address` | Admin who registered the issuer |
| `timestamp` | `u64`     | Registration timestamp          |

**Topic**: `["iss_reg", <issuer_address>]`

#### `iss_tier`

| Field  | Type         | Description                                 |
| ------ | ------------ | ------------------------------------------- |
| `tier` | `IssuerTier` | New tier (`Basic` / `Verified` / `Premium`) |

**Topic**: `["iss_tier", <issuer_address>]`

#### `iss_rem`

| Field       | Type      | Description                  |
| ----------- | --------- | ---------------------------- |
| `admin`     | `Address` | Admin who removed the issuer |
| `timestamp` | `u64`     | Removal timestamp            |

**Topic**: `["iss_rem", <issuer_address>]`

### Multi-Signature Events

#### `ms_prop`

| Field         | Type      | Description                       |
| ------------- | --------- | --------------------------------- |
| `proposal_id` | `String`  | Proposal identifier               |
| `proposer`    | `Address` | Address that created the proposal |
| `threshold`   | `u32`     | Required signature count          |

**Topic**: `["ms_prop", <subject_address>]`

#### `ms_sign`

| Field               | Type     | Description              |
| ------------------- | -------- | ------------------------ |
| `proposal_id`       | `String` | Proposal identifier      |
| `signatures_so_far` | `u32`    | Current signature count  |
| `threshold`         | `u32`    | Required signature count |

**Topic**: `["ms_sign", <signer_address>]`

#### `ms_actv`

| Field            | Type     | Description              |
| ---------------- | -------- | ------------------------ |
| `proposal_id`    | `String` | Proposal identifier      |
| `attestation_id` | `String` | Resulting attestation ID |

**Topic**: `["ms_actv"]`

### System Events

#### `adm_init`

| Field       | Type      | Description              |
| ----------- | --------- | ------------------------ |
| `admin`     | `Address` | Initial admin address    |
| `timestamp` | `u64`     | Initialization timestamp |

**Topic**: `["adm_init"]`

#### `adm_xfer`

| Field       | Type      | Description    |
| ----------- | --------- | -------------- |
| `old_admin` | `Address` | Previous admin |
| `new_admin` | `Address` | New admin      |

**Topic**: `["adm_xfer"]`

#### `clmtype`

| Field         | Type     | Description            |
| ------------- | -------- | ---------------------- |
| `description` | `String` | Claim type description |

**Topic**: `["clmtype", <claim_type_string>]`

#### `exp_hook`

| Field            | Type     | Description                    |
| ---------------- | -------- | ------------------------------ |
| `attestation_id` | `String` | Attestation nearing expiration |
| `expiration`     | `u64`    | Expiration timestamp           |

**Topic**: `["exp_hook", <subject_address>]`

---

## 3. Example Webhook Handler (Node.js)

The following service polls Soroban RPC for TrustLink events and forwards them to a configurable webhook URL.

### Install dependencies

```bash
mkdir trustlink-monitor && cd trustlink-monitor
npm init -y
npm install node-fetch@3
```

### `monitor.mjs`

```js
import fetch from "node-fetch";

// ---------------------------------------------------------------------------
// Configuration — override with environment variables
// ---------------------------------------------------------------------------
const RPC_URL = process.env.RPC_URL || "http://localhost:8000/soroban/rpc";
const CONTRACT_ID = process.env.CONTRACT_ID;
const WEBHOOK_URL = process.env.WEBHOOK_URL; // e.g. https://hooks.slack.com/...
const POLL_INTERVAL_MS = parseInt(process.env.POLL_INTERVAL_MS || "5000", 10);

if (!CONTRACT_ID) {
  console.error("CONTRACT_ID env var is required");
  process.exit(1);
}

// ---------------------------------------------------------------------------
// State — track the pagination cursor so we never re-process events
// ---------------------------------------------------------------------------
let cursor = undefined;
let latestLedger = undefined;

/** Fetch the latest ledger sequence from Soroban RPC. */
async function fetchLatestLedger() {
  const res = await fetch(RPC_URL, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 1,
      method: "getLatestLedger",
    }),
  });
  const json = await res.json();
  return json.result.sequence;
}

/** Poll Soroban RPC getEvents for new TrustLink contract events. */
async function pollEvents() {
  // On first poll, start from the current ledger.
  if (!latestLedger) {
    latestLedger = await fetchLatestLedger();
  }

  const params = {
    filters: [
      {
        type: "contract",
        contractIds: [CONTRACT_ID],
        topics: [["*"]],
      },
    ],
    pagination: { limit: 100 },
  };

  if (cursor) {
    params.pagination.cursor = cursor;
  } else {
    params.startLedger = String(latestLedger);
  }

  const res = await fetch(RPC_URL, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 1,
      method: "getEvents",
      params,
    }),
  });

  const json = await res.json();

  if (json.error) {
    console.error("RPC error:", json.error);
    return;
  }

  const events = json.result?.events || [];
  if (events.length > 0) {
    cursor = events[events.length - 1].pagingToken;
  }
  latestLedger = json.result?.latestLedger ?? latestLedger;

  for (const event of events) {
    await handleEvent(event);
  }
}

// ---------------------------------------------------------------------------
// Event classification helpers
// ---------------------------------------------------------------------------
const HIGH_SEVERITY = new Set(["revoked", "adm_xfer", "iss_rem"]);
const MEDIUM_SEVERITY = new Set([
  "created",
  "bridged",
  "imported",
  "iss_reg",
  "ms_actv",
]);

function classifyEvent(topicSymbol) {
  if (HIGH_SEVERITY.has(topicSymbol)) return "high";
  if (MEDIUM_SEVERITY.has(topicSymbol)) return "medium";
  return "low";
}

/** Process a single contract event — log it and forward to webhook. */
async function handleEvent(event) {
  const topicSymbol = event.topic?.[0] ?? "unknown";
  const severity = classifyEvent(topicSymbol);

  const payload = {
    contractId: CONTRACT_ID,
    ledger: event.ledger,
    timestamp: new Date().toISOString(),
    topic: event.topic,
    value: event.value,
    severity,
  };

  console.log(
    `[${severity.toUpperCase()}] ${topicSymbol}`,
    JSON.stringify(payload, null, 2),
  );

  if (WEBHOOK_URL) {
    try {
      await fetch(WEBHOOK_URL, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });
    } catch (err) {
      console.error("Webhook delivery failed:", err.message);
    }
  }
}

// ---------------------------------------------------------------------------
// Main loop
// ---------------------------------------------------------------------------
console.log(`Monitoring TrustLink contract ${CONTRACT_ID}`);
console.log(`RPC: ${RPC_URL} | Poll interval: ${POLL_INTERVAL_MS}ms`);
if (WEBHOOK_URL) console.log(`Webhook: ${WEBHOOK_URL}`);

async function loop() {
  while (true) {
    try {
      await pollEvents();
    } catch (err) {
      console.error("Poll error:", err.message);
    }
    await new Promise((r) => setTimeout(r, POLL_INTERVAL_MS));
  }
}

loop();
```

### Run the monitor

```bash
# Local network
CONTRACT_ID=$(cat .local.contract-id) node monitor.mjs

# With webhook forwarding
CONTRACT_ID=$(cat .local.contract-id) \
  WEBHOOK_URL=https://hooks.slack.com/services/T00/B00/xxx \
  node monitor.mjs

# Testnet
RPC_URL=https://soroban-testnet.stellar.org \
  CONTRACT_ID=CABC...XYZ \
  node monitor.mjs
```

---

## 4. Alerting Recommendations

### Severity Classification

| Severity     | Events                                                                        | Recommended Action                                                 |
| ------------ | ----------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| **Critical** | `adm_xfer`, `iss_rem`                                                         | Page on-call immediately — admin control changed or issuer revoked |
| **High**     | `revoked`                                                                     | Alert within minutes — an attestation trust decision was reversed  |
| **Medium**   | `created`, `imported`, `bridged`, `iss_reg`, `ms_actv`                        | Log and notify via Slack/email within the hour                     |
| **Low**      | `renewed`, `updated`, `endorsed`, `clmtype`, `ms_prop`, `ms_sign`, `exp_hook` | Aggregate in dashboards, review daily                              |

### What to Monitor

1. **Revocation spikes** — A sudden increase in `revoked` events may indicate a compromised issuer or policy change. Alert if the count exceeds a rolling threshold (e.g. >10 revocations in 5 minutes).

2. **Admin transfers** (`adm_xfer`) — Should be extremely rare. Any occurrence warrants immediate verification.

3. **Issuer removals** (`iss_rem`) — Verify that the removal was intentional and that affected attestations are handled.

4. **Bridge activity** (`bridged`) — Monitor for unexpected source chains or abnormal volume, which could indicate a bridge compromise.

5. **Expiration hooks** (`exp_hook`) — Track these to ensure callback contracts are responding. A backlog of unacknowledged hooks suggests the callback endpoint is down.

6. **Fee collection** — Cross-reference `created` events with fee token transfer operations on Horizon to confirm fees are reaching the collector.

### Integration Targets

| Platform      | Integration Method                                                                            |
| ------------- | --------------------------------------------------------------------------------------------- |
| **Slack**     | POST event payload to an [Incoming Webhook URL](https://api.slack.com/messaging/webhooks)     |
| **PagerDuty** | POST critical events to the [Events API v2](https://developer.pagerduty.com/api-reference/)   |
| **Grafana**   | Push metrics to Prometheus via a push-gateway; build dashboards on event counts               |
| **Datadog**   | Send events via the [Datadog API](https://docs.datadoghq.com/api/latest/events/) or DogStatsD |
| **Email**     | Use the webhook handler to relay critical events through an SMTP service                      |

### Example: Slack Alert Format

```json
{
  "text": ":rotating_light: *TrustLink Alert — HIGH*",
  "blocks": [
    {
      "type": "section",
      "text": {
        "type": "mrkdwn",
        "text": "*Event*: `revoked`\n*Attestation*: `abc123...`\n*Issuer*: `GABC...XYZ`\n*Reason*: Compliance review\n*Ledger*: 12345678"
      }
    }
  ]
}
```

### Dashboard Metrics to Track

- **Attestations created per hour** — baseline for normal activity
- **Revocations per hour** — spike detection
- **Bridge attestations per day per source chain** — detect anomalies
- **Mean time between `exp_hook` and renewal** — issuer responsiveness
- **Active issuers** — track `iss_reg` minus `iss_rem` over time
- **Multi-sig proposals pending** — `ms_prop` minus `ms_actv` backlog

---

## 5. Production Checklist

- [ ] Deploy the monitor as a long-running service (systemd, Docker, or Kubernetes)
- [ ] Set `POLL_INTERVAL_MS` based on ledger close time (~5–6 s on mainnet)
- [ ] Persist the pagination `cursor` to disk or a database so restarts do not miss events
- [ ] Authenticate webhook endpoints (use HMAC signatures or bearer tokens)
- [ ] Rate-limit outbound webhook calls to avoid flooding downstream services
- [ ] Set up a dead-letter queue for failed webhook deliveries
- [ ] Test alerting end-to-end on the local network before enabling on testnet/mainnet

---

## 6. Alert Definitions and Thresholds

These are the recommended alert rules for a production TrustLink deployment.
Adjust thresholds to match your expected traffic volume.

| Alert name | Condition | Severity | Response time |
|---|---|---|---|
| `admin_transfer` | Any `adm_xfer` event | Critical | Immediate page |
| `issuer_removed` | Any `iss_rem` event | Critical | Immediate page |
| `revocation_spike` | `revoked` events > 10 in any 5-minute window | High | 5 minutes |
| `bridge_anomaly` | `bridged` events from an unrecognised `source_chain` | High | 5 minutes |
| `issuer_deregistered_with_active_attestations` | `iss_rem` where issuer has > 0 active attestations | High | 15 minutes |
| `multisig_proposal_expired` | `ms_prop` with no `ms_actv` within 7 days | Medium | Next business day |
| `expiration_hook_backlog` | `exp_hook` events > 50 with no corresponding renewal in 24 h | Medium | 1 hour |
| `no_events_received` | Zero events from contract in > 30 minutes during business hours | Low | 1 hour |

**Threshold tuning:** Start with the defaults above, then adjust after observing
one week of baseline traffic. A `revocation_spike` threshold that fires daily is
too sensitive; one that never fires may be too loose.

---

## 7. On-Call Runbook for Common Alerts

### `admin_transfer` — Admin key changed

**What happened:** The contract admin address was replaced via `adm_xfer`.

1. Confirm the change was planned — check the deployment log and Slack/email for
   a scheduled admin rotation.
2. If unplanned, treat as a security incident:
   - Notify the security lead immediately.
   - Freeze all issuer registrations and attestation creation at the application
     layer (block the UI / API gateway) while investigating.
   - Identify the transaction on the explorer and determine which key signed it.
   - Follow the incident response plan in `docs/security.md`.
3. If planned, verify the new admin address matches the expected key and update
   `DEPLOYMENT.md`.

---

### `issuer_removed` — Issuer deregistered

**What happened:** An issuer was removed from the registry via `iss_rem`.

1. Confirm the removal was intentional — check the admin activity log.
2. Identify all active attestations issued by the removed issuer:
   ```bash
   stellar contract invoke --id "$CONTRACT_ID" --network mainnet \
     -- get_issuer_attestations \
     --issuer <ISSUER_ADDRESS> --start 0 --limit 100
   ```
3. Decide whether existing attestations need to be transferred to a successor
   issuer (`transfer_attestation`) or left as-is (they remain valid until
   revoked or expired).
4. Notify any integrators that relied on attestations from this issuer.

---

### `revocation_spike` — Unusual number of revocations

See the dedicated runbook in section 8.

---

### `bridge_anomaly` — Unknown source chain

**What happened:** A `bridged` event arrived with a `source_chain` value not in
the expected set.

1. Identify the bridge contract address from the event topic.
2. Verify it is still in the bridge registry:
   ```bash
   stellar contract invoke --id "$CONTRACT_ID" --network mainnet \
     -- is_bridge --bridge <BRIDGE_ADDRESS>
   ```
3. If the bridge is registered but the source chain is unexpected, contact the
   bridge operator to confirm the event is legitimate.
4. If the bridge is not registered, the event should not have been possible —
   escalate to the security lead as a potential contract bug.

---

### `no_events_received` — Monitor silence

**What happened:** The event poller has not received any events for > 30 minutes
during expected active hours.

1. Check that the monitor process is running:
   ```bash
   systemctl status trustlink-monitor   # or: docker ps | grep trustlink-monitor
   ```
2. Verify RPC connectivity:
   ```bash
   curl -s -X POST "$RPC_URL" \
     -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger"}' | jq .result.sequence
   ```
3. Confirm the contract is still active:
   ```bash
   stellar contract invoke --id "$CONTRACT_ID" --network mainnet -- health_check
   ```
4. If the RPC node is unresponsive, switch to a backup RPC endpoint and restart
   the monitor.

---

## 8. Investigating a Spike in Revocations

A revocation spike (> 10 revocations in 5 minutes) can indicate a compromised
issuer, a bulk compliance action, or a bug in an issuer's automation.

### Step 1 — Quantify the spike

Pull the raw revocation events for the last hour:

```bash
curl -s -X POST "$RPC_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0", "id": 1, "method": "getEvents",
    "params": {
      "startLedger": "'$START_LEDGER'",
      "filters": [{
        "type": "contract",
        "contractIds": ["'$CONTRACT_ID'"],
        "topics": [["SymbolVal(revoked)"]]
      }],
      "pagination": { "limit": 200 }
    }
  }' | jq '.result.events | length'
```

### Step 2 — Identify the issuer(s)

The second topic element of a `revoked` event is the issuer address. Extract
the unique issuers involved:

```bash
# Parse issuer addresses from revoked events (requires jq)
curl -s ... | jq '[.result.events[].topic[1]] | unique'
```

### Step 3 — Determine intent

- **Planned bulk action:** Contact the issuer. If they confirm a deliberate
  compliance sweep (e.g. sanctions list update), document it and close the
  alert.
- **Issuer automation bug:** Ask the issuer to pause their automation
  immediately. Assess whether any incorrectly revoked attestations need to be
  re-issued.
- **Compromised issuer key:** Treat as a security incident:
  1. Remove the issuer immediately:
     ```bash
     stellar contract invoke --id "$CONTRACT_ID" --source "$ADMIN_SECRET" \
       --network mainnet -- remove_issuer \
       --admin "$ADMIN_PUBLIC" --issuer <COMPROMISED_ISSUER>
     ```
  2. Notify affected subjects — their attestations are now revoked and they
     will need to re-verify with a new issuer.
  3. Transfer any legitimate attestations to a successor issuer if applicable.
  4. File an incident report.

### Step 4 — Post-incident

- Record the event in the incident log with: timestamp, issuer, count of
  revocations, root cause, and resolution.
- Review whether the revocation threshold needs adjusting.

---

## 9. Detecting and Responding to Storage Exhaustion

TrustLink enforces per-issuer and per-subject attestation limits
(`max_attestations_per_issuer`, `max_attestations_per_subject`). When a limit
is hit, `create_attestation` returns `Error::LimitExceeded` (code `#10`).

### Detection

**Monitor for `LimitExceeded` errors** in your application layer — these will
surface as failed contract invocations, not as contract events. Log every
`Error(Contract, #10)` response from the RPC.

**Proactively check high-volume issuers** before they hit the limit:

```bash
# Count attestations for a specific issuer
stellar contract invoke --id "$CONTRACT_ID" --network mainnet \
  -- get_issuer_attestation_count --issuer <ISSUER_ADDRESS>

# Read current limits
stellar contract invoke --id "$CONTRACT_ID" --network mainnet -- get_limits
```

Alert when any issuer reaches 80% of `max_attestations_per_issuer`.

**Check global stats** for overall growth trends:

```bash
stellar contract invoke --id "$CONTRACT_ID" --network mainnet -- get_global_stats
```

### Response

**Option A — Raise the limit (admin action)**

If the limit was set conservatively and the issuer's volume is legitimate:

```bash
stellar contract invoke --id "$CONTRACT_ID" --source "$ADMIN_SECRET" \
  --network mainnet -- set_limits \
  --admin "$ADMIN_PUBLIC" \
  --max_attestations_per_issuer 20000 \
  --max_attestations_per_subject 200
```

Document the change and the reason in the operations log.

**Option B — Revoke stale attestations**

If the issuer has a large backlog of expired or superseded attestations, revoke
them in batch to free up headroom:

```bash
stellar contract invoke --id "$CONTRACT_ID" --source "$ISSUER_SECRET" \
  --network mainnet -- revoke_attestations_batch \
  --issuer <ISSUER_ADDRESS> \
  --attestation_ids '["id1","id2","id3"]'
```

**Option C — Distribute across multiple issuers**

If a single issuer is handling disproportionate volume, register additional
issuer addresses and distribute new attestation creation across them.

### Prevention

- Set a monitoring alert at 80% of each limit.
- Review `get_global_stats` weekly during the first month after mainnet launch
  to establish a baseline growth rate.
- Include limit headroom in capacity planning before onboarding high-volume
  issuers.
