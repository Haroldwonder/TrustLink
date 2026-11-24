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
const HIGH_SEVERITY = new Set([
  "revoked",
  "adm_xfer",
  "iss_rem",
  "brg_reg",
  "brg_rem",
  "cfg_set",
]);
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
| **Critical** | `adm_xfer`, `iss_rem`, `brg_reg`                                              | Page on-call immediately — admin control changed, issuer revoked, or a new bridge authority added |
| **High**     | `revoked`, `brg_rem`, `cfg_set`                                               | Alert within minutes — a trust decision was reversed or configuration changed |
| **Medium**   | `created`, `imported`, `bridged`, `iss_reg`, `ms_actv`                        | Log and notify via Slack/email within the hour                     |
| **Low**      | `renewed`, `updated`, `endorsed`, `clmtype`, `ms_prop`, `ms_sign`, `exp_hook` | Aggregate in dashboards, review daily                              |

### What to Monitor

1. **Revocation spikes** — A sudden increase in `revoked` events may indicate a compromised issuer or policy change. Alert if the count exceeds a rolling threshold (e.g. >10 revocations in 5 minutes).

2. **Admin transfers** (`adm_xfer`) — Should be extremely rare. Any occurrence warrants immediate verification.

3. **Issuer removals** (`iss_rem`) — Verify that the removal was intentional and that affected attestations are handled.

4. **Bridge activity** (`bridged`) — Monitor for unexpected source chains or abnormal volume, which could indicate a bridge compromise. Pair this with `brg_reg` / `brg_rem` alerts so that any change to the set of bridge authorities is reviewed immediately.

5. **Configuration changes** (`cfg_set`) — Every admin setter emits a `cfg_set` event. Alert on all changes and page on changes to `fee`, `council_timelock_delay`, or `multisig_ttl`, which directly affect economics and security guarantees.

6. **Expiration hooks** (`exp_hook`) — Track these to ensure callback contracts are responding. A backlog of unacknowledged hooks suggests the callback endpoint is down.

7. **Fee collection** — Cross-reference `created` events with fee token transfer operations on Horizon to confirm fees are reaching the collector.

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

### Prometheus Alerting Rules

Ready-to-use Prometheus alerting rules are provided in
[`monitoring/alerts.yml`](../monitoring/alerts.yml). Load the file into your
Prometheus configuration:

```yaml
# prometheus.yml
rule_files:
  - "monitoring/alerts.yml"
```

The file defines one alert group (`trustlink`) with four rules. Each rule
requires the following metrics to be exported by your TrustLink event indexer:

| Metric | Type | Description |
|---|---|---|
| `trustlink_contract_paused` | Gauge | `1` when the contract is paused, `0` otherwise |
| `trustlink_admin_added_total` | Counter | Cumulative count of `adm_add` events |
| `trustlink_admin_removed_total` | Counter | Cumulative count of `adm_rem` events |
| `trustlink_issuer_removed_total` | Counter | Cumulative count of `iss_rem` events |
| `trustlink_attestation_revoked_total` | Counter | Cumulative count of `revoked` events |
| `trustlink_latest_ledger` | Gauge | Current chain tip ledger sequence |
| `trustlink_indexer_last_processed_ledger` | Gauge | Last ledger processed by the indexer |
| `trustlink_bridge_registered_total` | Counter | Cumulative count of `brg_reg` events |
| `trustlink_bridge_removed_total` | Counter | Cumulative count of `brg_rem` events |
| `trustlink_config_changed_total` | Counter | Cumulative count of `cfg_set` events, labelled by `key` |

#### `TrustLinkContractPaused`

**Severity:** Critical  
**Condition:** `trustlink_contract_paused == 1`

Fires immediately when the contract is paused via `pause()`. All attestation
write operations are halted while the contract is paused. Verify the pause was
an intentional admin action (e.g. incident response) and call `unpause()` once
the threat is contained.

#### `TrustLinkIssuerRemoved`

**Severity:** Critical  
**Condition:** Any `iss_rem` event in the last 5 minutes

Fires when an issuer is removed from the registry. Existing attestations from
the removed issuer remain valid but no new ones can be created, and the issuer
can no longer revoke their own attestations. Confirm the removal was intentional
and assess whether affected attestations need to be transferred to a successor
issuer via `transfer_attestation`.

#### `TrustLinkHighRevocationRate`

**Severity:** High  
**Condition:** More than 10 `revoked` events in any 5-minute window

A revocation spike may indicate a compromised issuer key, a bulk compliance
action (e.g. sanctions list update), or a bug in issuer automation. Follow the
runbook in [Section 8](#8-investigating-a-spike-in-revocations) to determine
the root cause. Adjust the threshold (`> 10`) to match your expected baseline
traffic after one week of observation.

#### `TrustLinkIndexerLag`

**Severity:** High  
**Condition:** Indexer more than 100 ledgers behind the chain tip for 2 minutes

When the indexer lags, all event-based alerts become unreliable — revocation
spikes, issuer removals, and bridge anomalies may go undetected. Check that the
indexer process is running, verify RPC connectivity (see the
[`no_events_received` runbook](#no_events_received--monitor-silence)), and
switch to a backup RPC endpoint if needed.

---

### Dashboard Metrics to Track

- **Attestations created per hour** — baseline for normal activity
- **Revocations per hour** — spike detection
- **Bridge attestations per day per source chain** — detect anomalies
- **Mean time between `exp_hook` and renewal** — issuer responsiveness
- **Active issuers** — track `iss_reg` minus `iss_rem` over time
- **Registered bridges** — track `brg_reg` minus `brg_rem` over time
- **Configuration changes per day by key** — `rate(trustlink_config_changed_total[1d])` grouped by `key`
- **Multi-sig proposals pending** — `ms_prop` minus `ms_actv` backlog

### Indexer Metrics

The TrustLink indexer exposes Prometheus metrics for monitoring event processing:

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `trustlink_events_processed_total` | Counter | `type` | Total events processed by type |
| `trustlink_events_failed_total` | Counter | `type` | Total events that failed to process by type |

Event types tracked:
- `created` — Attestation created
- `imported` — Attestation imported
- `bridged` — Attestation bridged from another chain
- `revoked` — Attestation revoked
- `renewed` — Attestation renewed
- `updated` — Attestation updated
- `expired` — Attestation expired
- `endorsed` — Attestation endorsed
- `iss_reg` — Issuer registered
- `iss_tier` — Issuer tier updated
- `iss_rem` — Issuer removed
- `clmtype` — Claim type registered
- `ms_prop` — Multi-sig proposed
- `ms_sign` — Multi-sig co-signed
- `ms_actv` — Multi-sig activated
- `adm_init` — Admin initialized
- `adm_xfer` — Admin transferred
- `brg_reg` — Bridge registered
- `brg_rem` — Bridge removed
- `cfg_set` — Admin configuration changed

Example Prometheus query for revocation rate:
```promql
rate(trustlink_events_processed_total{type="revoked"}[5m])
```

---

## 5. Service Level Objectives and Error Budget

These SLOs define the reliability commitments for the TrustLink indexer and its
public-facing GraphQL API. Targets are measured over a rolling **30-day** window.

| ID | Service | Metric | Target | Error budget (30 days) |
|---|---|---|---|---|
| SLO-1 | GraphQL API | Availability (non-5xx responses / total requests) | **≥ 99.9 %** | 43.8 min/month |
| SLO-2 | Indexer | Lag ≤ 100 ledgers | **≥ 99 %** of minutes | 7.2 h/month |
| SLO-3 | GraphQL API | Request latency p95 | **≤ 2 s** | 43.8 min/month |
| SLO-4 | Event pipeline | Event-to-queryable latency p95 | **≤ 10 s** | 43.8 min/month |

### PromQL expressions for SLO tracking

**SLO-1 — GraphQL availability (30-day rolling)**

```promql
# Error rate over 30 days
sum(rate(trustlink_http_requests_total{status=~"5.."}[30d]))
  /
sum(rate(trustlink_http_requests_total[30d]))
```

**SLO-2 — Indexer lag compliance (30-day rolling)**

```promql
# Fraction of 1-minute intervals where lag exceeded 100 ledgers
avg_over_time(
  (
    (trustlink_latest_ledger - trustlink_indexer_last_processed_ledger) > bool 100
  )[30d:1m]
)
```

**SLO-3 — GraphQL p95 latency**

```promql
histogram_quantile(0.95,
  sum(rate(trustlink_http_request_duration_seconds_bucket[5m])) by (le)
)
```

**SLO-4 — Event-to-queryable p95 latency**

```promql
histogram_quantile(0.95,
  sum(rate(trustlink_event_processing_duration_seconds_bucket[5m])) by (le)
)
```

### Error budget policy

| Budget consumed | Action |
|---|---|
| 0 – 25 % | No restrictions — normal development velocity |
| 25 – 50 % | Review recent deployments; ensure rollback readiness |
| 50 – 75 % | Freeze non-critical features; focus on reliability work |
| 75 – 100 % | Freeze all changes except P0 incident fixes; conduct blameless post-mortem |
| > 100 % (exhausted) | No deployments until budget resets; mandatory reliability sprint |

The `TrustLinkIndexerLag` alert (see [`monitoring/alerts.yml`](../monitoring/alerts.yml))
fires when SLO-2 is actively being breached and includes per-minute budget burn
rate in its annotation.

---

## 6. Synthetic Uptime Checks

In addition to Prometheus-scraped metrics, two external synthetic probes run on
a schedule to verify that the indexer GraphQL endpoint and the upstream Stellar
Soroban RPC are reachable from outside the cluster.

The checks live in [`.github/workflows/synthetic-uptime.yml`](../.github/workflows/synthetic-uptime.yml)
and run every **5 minutes** via `schedule: cron`. They are intentionally
kept outside the indexer process so they catch failures that the indexer's own
metrics cannot (e.g. a network partition or a crashed process that still exports
stale metrics via push-gateway).

### What is checked

| Check | Target | Pass condition |
|---|---|---|
| GraphQL `healthCheck` | `GQL_URL/graphql` | Response `status == "ok"` within 10 s |
| REST `/health` | `INDEXER_URL/health` | HTTP 200 within 10 s |
| Stellar Soroban RPC | `RPC_URL` via `getLatestLedger` | Valid `sequence` returned within 10 s |

### Alert on failure

When any check fails, the workflow exits with a non-zero status and GitHub
Actions notifies the repository's configured alert channel. For PagerDuty or
Slack integration, add a step to the workflow calling the respective API with
the `${{ failure() }}` condition.

### Environment variables

| Variable | Default | Description |
|---|---|---|
| `GQL_URL` | (required) | Base URL of the GraphQL endpoint, e.g. `https://indexer.trustlink.io` |
| `INDEXER_URL` | (required) | Base URL of the REST API, e.g. `https://indexer.trustlink.io` |
| `RPC_URL` | `https://soroban-testnet.stellar.org` | Stellar Soroban RPC endpoint |
| `LATENCY_THRESHOLD_MS` | `10000` | Milliseconds above which a check is considered slow |

Set `GQL_URL` and `INDEXER_URL` as repository secrets (`Settings → Secrets →
Actions`) before enabling the workflow.

---

## 11. Issuer Activity Dashboard

A second Grafana dashboard focused on per-issuer activity is provided at
[`monitoring/grafana-issuer-dashboard.json`](../monitoring/grafana-issuer-dashboard.json).
It is intended for both the platform team (spotting abuse patterns) and issuer
operators reviewing their own usage.

### Panels

| Panel | Type | What it shows |
|---|---|---|
| **Attestation Rate by Issuer** | Time series | `rate(trustlink_issuer_attestations_total[$__rate_interval])` per issuer |
| **Revocation Rate by Issuer** | Time series | `rate(trustlink_issuer_revocations_total[$__rate_interval])` per issuer |
| **Issuer Rate-Limit Utilisation** | Bar gauge | `trustlink_issuer_rate_limit_ratio` (0–1) per issuer |
| **Revocation Ratio** | Table | Revocations ÷ attestations per issuer |
| **Top Issuers by Volume** | Bar chart | `topk(10, trustlink_issuer_attestations_total)` |

### Required metrics

| Metric | Type | Labels | Description |
|---|---|---|---|
| `trustlink_issuer_attestations_total` | Counter | `issuer` | Attestations created per issuer |
| `trustlink_issuer_revocations_total` | Counter | `issuer` | Attestations revoked per issuer |
| `trustlink_issuer_rate_limit_ratio` | Gauge | `issuer` | Fraction of per-issuer attestation capacity consumed (0–1) |

These metrics are exported by the TrustLink indexer alongside the existing
system metrics.

### Importing the dashboard

Follow the same import steps described in §9 but select
`monitoring/grafana-issuer-dashboard.json`.

---

## 7. Production Checklist

- [ ] Deploy the monitor as a long-running service (systemd, Docker, or Kubernetes)
- [ ] Set `POLL_INTERVAL_MS` based on ledger close time (~5–6 s on mainnet)
- [ ] Persist the pagination `cursor` to disk or a database so restarts do not miss events
- [ ] Authenticate webhook endpoints (use HMAC signatures or bearer tokens)
- [ ] Rate-limit outbound webhook calls to avoid flooding downstream services
- [ ] Set up a dead-letter queue for failed webhook deliveries
- [ ] Test alerting end-to-end on the local network before enabling on testnet/mainnet
- [ ] Confirm the indexer exports `trustlink_bridge_registered_total`, `trustlink_bridge_removed_total`, and `trustlink_config_changed_total{key}` and that `brg_reg`, `brg_rem`, and `cfg_set` alerts are wired to the on-call rotation

---

## 6. Alert Definitions and Thresholds

These are the recommended alert rules for a production TrustLink deployment.
Adjust thresholds to match your expected traffic volume.

| Alert name | Condition | Severity | Response time |
|---|---|---|---|
| `TrustLinkContractPaused` | `trustlink_contract_paused == 1` | Critical | Immediate page |
| `TrustLinkIssuerRemoved` | Any `iss_rem` event in last 5 min | Critical | Immediate page |
| `TrustLinkHighRevocationRate` | `revoked` events > 10 in any 5-minute window (all issuers) | High | 5 minutes |
| `TrustLinkIssuerRevocationSpike` | Any single issuer exceeds 10 `revoked` events in 5 min | High | 5 minutes |
| `TrustLinkIndexerLag` | Indexer > 100 ledgers behind chain tip for 2 min (SLO-2 breach) | High | 5 minutes |
| `admin_added` | Any `adm_add` event | Critical | Immediate page |
| `admin_removed` | Any `adm_rem` event | Critical | Immediate page |
| `admin_transfer` | Any `adm_xfer` event | Critical | Immediate page |
| `issuer_removed` | Any `iss_rem` event | Critical | Immediate page |
| `revocation_spike` | `revoked` events > 10 in any 5-minute window | High | 5 minutes |
| `bridge_anomaly` | `bridged` events from an unrecognised `source_chain` | High | 5 minutes |
| `bridge_registered` | Any `brg_reg` event | Critical | Immediate page |
| `bridge_removed` | Any `brg_rem` event | High | 5 minutes |
| `config_changed` | Any `cfg_set` event | High | 5 minutes |
| `config_critical_changed` | `cfg_set` with `key` in (`fee`, `council_timelock_delay`, `multisig_ttl`) | Critical | Immediate page |
| `issuer_deregistered_with_active_attestations` | `iss_rem` where issuer has > 0 active attestations | High | 15 minutes |
| `multisig_proposal_expired` | `ms_prop` with no `ms_actv` within 7 days | Medium | Next business day |
| `expiration_hook_backlog` | `exp_hook` events > 50 with no corresponding renewal in 24 h | Medium | 1 hour |
| `no_events_received` | Zero events from contract in > 30 minutes during business hours | Low | 1 hour |

The `TrustLinkIssuerRevocationSpike` alert fires per issuer, unlike `TrustLinkHighRevocationRate`
which aggregates across all issuers. Both use the same 5-minute rolling window. Adjust
the `> 10` threshold in [`monitoring/alerts.yml`](../monitoring/alerts.yml) to match your
baseline after one week of observation.

**Threshold tuning:** Start with the defaults above, then adjust after observing
one week of baseline traffic. A `revocation_spike` threshold that fires daily is
too sensitive; one that never fires may be too loose.

The `admin_added` and `admin_removed` rules require the following additional
metrics to be exported by your indexer:

| Metric | Type | Description |
|--------|------|-------------|
| `trustlink_admin_added_total` | Counter | Cumulative count of `adm_add` events, labelled with `actor` and `timestamp` |
| `trustlink_admin_removed_total` | Counter | Cumulative count of `adm_rem` events, labelled with `actor` and `timestamp` |

The bridge and configuration rules require these additional metrics:

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `trustlink_bridge_registered_total` | Counter | `bridge` | Cumulative count of `brg_reg` events (bridge contracts added to the registry) |
| `trustlink_bridge_removed_total` | Counter | `bridge` | Cumulative count of `brg_rem` events (bridge contracts removed from the registry) |
| `trustlink_config_changed_total` | Counter | `key`, `admin` | Cumulative count of `cfg_set` events, one increment per configuration key changed |

`cfg_set` keys currently emitted by the contract include `fee`,
`rate_limit`, `rate_limit_claim_type`, `limits`, `chunk_size`, `decay_config`,
`council_timelock_delay`, `registered_claim_type`, `metadata_hash_only`,
`claim_type_constraints`, `multisig_ttl`, and `max_attestations_per_subject`.

#### `bridge_registered` — Bridge contract registered

**Severity:** Critical  
**Condition:** Any `brg_reg` event

A newly registered bridge can mint `Bridged` attestations for any subject and
claim type, so an unexpected registration is equivalent to granting a new
attestation authority. Verify the registration was planned and that the bridge
address matches the expected cross-chain contract. If unplanned, remove the
bridge immediately with `remove_bridge` and follow the incident response plan
in `docs/security.md`.

#### `bridge_removed` — Bridge contract removed

**Severity:** High  
**Condition:** Any `brg_rem` event

A bridge was deregistered; its `bridge_attestation` calls now fail with
`Unauthorized`. Confirm the removal was intentional (deprecation, rotation, or
incident response). If the bridge is still expected to be live, re-register it
with `register_bridge` after verifying the address.

#### `config_changed` / `config_critical_changed` — Admin configuration changed

**Severity:** High (Critical for `fee`, `council_timelock_delay`, `multisig_ttl`)  
**Condition:** Any `cfg_set` event; escalate when `key` is security- or
economics-relevant

Every admin setter emits a `cfg_set` event carrying the acting admin, the
configuration key, and the old and new values. Alert on any change and page
immediately for the following keys:

- `fee` — attestation fee changed; verify the new value against the published
  fee schedule.
- `council_timelock_delay` — a value of `0` disables the timelock on council
  operations; treat as a security incident unless explicitly planned.
- `multisig_ttl` — changing the proposal TTL alters how long multi-sig
  proposals remain actionable.

For all other keys, confirm the change matches a scheduled configuration
ticket and record the old/new values in the operations log.

Example PromQL rules:

```yaml
groups:
  - name: trustlink-config
    rules:
      - alert: TrustLinkBridgeRegistered
        expr: increase(trustlink_bridge_registered_total[5m]) > 0
        labels:
          severity: critical
        annotations:
          summary: "Bridge {{ $labels.bridge }} registered on TrustLink"

      - alert: TrustLinkBridgeRemoved
        expr: increase(trustlink_bridge_removed_total[5m]) > 0
        labels:
          severity: high
        annotations:
          summary: "Bridge {{ $labels.bridge }} removed from TrustLink"

      - alert: TrustLinkConfigChanged
        expr: increase(trustlink_config_changed_total[5m]) > 0
        labels:
          severity: high
        annotations:
          summary: "TrustLink config {{ $labels.key }} changed by {{ $labels.admin }}"

      - alert: TrustLinkCriticalConfigChanged
        expr: increase(trustlink_config_changed_total{key=~"fee|council_timelock_delay|multisig_ttl"}[5m]) > 0
        labels:
          severity: critical
        annotations:
          summary: "TrustLink critical config {{ $labels.key }} changed by {{ $labels.admin }}"
```

---

## 7. On-Call Runbook for Common Alerts

### `admin_added` — Admin council member added

**What happened:** A new address was added to the admin council via an `adm_add` event.

1. Confirm the change was planned — cross-reference with your deployment log or
   scheduled admin rotation ticket.
2. Verify the actor address (the admin who performed the addition) is a known,
   authorized admin.
3. Verify the new admin address is the expected key (not a compromised or unknown address).
4. If unplanned:
   - Notify the security lead immediately.
   - Freeze application-layer operations (block UI/API gateway) while investigating.
   - Identify the transaction on the Stellar explorer and determine which key signed it.
   - If the addition was unauthorized, remove the rogue admin with `remove_admin` from a
     known-good admin account and follow the incident response plan in `docs/security.md`.
5. If planned, update `DEPLOYMENT.md` with the new admin's address and role.

---

### `admin_removed` — Admin council member removed

**What happened:** An address was removed from the admin council via an `adm_rem` event.

1. Confirm the removal was intentional — check the admin activity log and deployment history.
2. Verify the actor address (the admin who performed the removal) is authorized.
3. Check that at least one admin remains in the council:
   ```bash
   stellar contract invoke --id "$CONTRACT_ID" --network mainnet -- get_admin_council
   ```
   If the council is now empty, the contract is unmanageable — escalate immediately.
4. If unplanned, treat as a potential account takeover:
   - Notify the security lead.
   - Verify remaining admins can still authenticate and operate the contract.
   - Follow the incident response plan in `docs/security.md`.

---

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

### `bridge_registered` — Unexpected bridge registration

**What happened:** A bridge contract was added to the registry via `brg_reg`.

1. Confirm the registration was planned — cross-reference with the deployment
   log or a bridge onboarding ticket.
2. Verify the actor address (the admin who registered the bridge) is a known,
   authorized admin.
3. Verify the bridge address matches the expected cross-chain contract:
   ```bash
   stellar contract invoke --id "$CONTRACT_ID" --network mainnet \
     -- is_bridge --bridge <BRIDGE_ADDRESS>
   ```
4. If unplanned, treat as a security incident:
   - Remove the bridge immediately:
     ```bash
     stellar contract invoke --id "$CONTRACT_ID" --source "$ADMIN_SECRET" \
       --network mainnet -- remove_bridge \
       --admin "$ADMIN_PUBLIC" --bridge <BRIDGE_ADDRESS>
     ```
   - Audit `bridged` events emitted by the rogue bridge since registration.
   - Follow the incident response plan in `docs/security.md`.

---

### `bridge_removed` — Bridge contract removed

**What happened:** A bridge contract was removed from the registry via `brg_rem`.

1. Confirm the removal was intentional (deprecation, key rotation, or incident
   response).
2. Check whether the bridge is still expected to be live; if so, re-register it
   with `register_bridge` after verifying the address.
3. Review `bridged` events from the removed bridge around the removal time to
   confirm no in-flight cross-chain attestations were dropped.

---

### `config_changed` — Admin configuration changed

**What happened:** An admin setter emitted a `cfg_set` event.

1. Identify the `key`, `admin`, `old_value`, and `new_value` from the event.
2. Confirm the change matches a scheduled configuration ticket.
3. For `fee`, `council_timelock_delay`, and `multisig_ttl`, treat unplanned
   changes as security incidents — a `council_timelock_delay` of `0` disables
   the timelock on council operations.
4. Record the change in the operations log and update any dependent monitoring
   thresholds (e.g. fee-based revenue alerts).

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

## 9. Grafana Dashboard

A pre-built Grafana dashboard is provided at [`monitoring/grafana-dashboard.json`](../monitoring/grafana-dashboard.json). It includes five panels:

| Panel | Type | What it shows |
|---|---|---|
| **Total Attestations Over Time** | Time series | Cumulative `trustlink_attestations_total` counter |
| **Revocation Rate** | Time series | `rate(trustlink_revocations_total[$__rate_interval])` — spikes indicate bulk revocations |
| **Active Issuers** | Stat | Current value of `trustlink_active_issuers` |
| **Indexer Lag** | Gauge | `trustlink_indexer_lag_ledgers` — ledgers the indexer is behind the chain tip |
| **Webhook Delivery Success Rate** | Time series | Ratio of `trustlink_webhook_deliveries_success_total` to total deliveries |

### Prerequisites

- Grafana 10.0+ with a **Prometheus** datasource configured and named `Prometheus`.
- Your TrustLink indexer exposes the metrics listed above on a `/metrics` endpoint that Prometheus scrapes.

### Importing the dashboard

**Via the Grafana UI:**

1. Open Grafana and go to **Dashboards → Import** (or navigate to `/dashboard/import`).
2. Click **Upload dashboard JSON file** and select `monitoring/grafana-dashboard.json`.
3. In the **Prometheus** dropdown, select your Prometheus datasource.
4. Click **Import**.

**Via the Grafana HTTP API:**

```bash
# Replace <GRAFANA_URL>, <USER>, and <PASSWORD> with your values
curl -s -X POST \
  -H "Content-Type: application/json" \
  -u "<USER>:<PASSWORD>" \
  --data-binary @monitoring/grafana-dashboard.json \
  "<GRAFANA_URL>/api/dashboards/import"
```

**Via Grafana provisioning (GitOps):**

Copy the file into your Grafana provisioning directory and add a provider config:

```yaml
# grafana/provisioning/dashboards/trustlink.yaml
apiVersion: 1
providers:
  - name: TrustLink
    folder: TrustLink
    type: file
    options:
      path: /etc/grafana/dashboards/trustlink
```

```bash
cp monitoring/grafana-dashboard.json /etc/grafana/dashboards/trustlink/
# Grafana picks up the file automatically on the next provisioning cycle (default: 10 s)
```

### Metric names reference

| Metric | Type | Description |
|---|---|---|
| `trustlink_attestations_total` | Counter | All attestations ever created |
| `trustlink_revocations_total` | Counter | All revocations ever performed |
| `trustlink_active_issuers` | Gauge | Current registered issuer count |
| `trustlink_indexer_lag_ledgers` | Gauge | Ledgers the indexer is behind the chain tip |
| `trustlink_webhook_deliveries_success_total` | Counter | Webhook calls that returned HTTP 2xx |
| `trustlink_webhook_deliveries_failure_total` | Counter | Webhook calls that failed or timed out |

---

## 10. Detecting and Responding to Storage Exhaustion

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
