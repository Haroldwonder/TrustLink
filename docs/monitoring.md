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
