# TrustLink Event Topics Reference

This document defines the canonical event topic taxonomy used throughout TrustLink. All event topics are emitted by the smart contract and can be subscribed to via the indexer's GraphQL subscriptions or direct ledger event watching through the SDK.

## Event Topic Structure

Each event topic is a short symbol (≤9 characters) emitted as the first element of the contract event tuple. Event topics are organized into five primary categories:

1. **Attestation Lifecycle** — creation, revocation, renewal, expiration, and status changes
2. **Issuer Lifecycle** — registration, tier updates, removal
3. **Admin & Governance** — initialization, transfers, additions, removals, council actions
4. **Compliance & Requests** — attestation requests, claim type registration, delegation, whitelist management
5. **Multi-Sig & Proposals** — proposal creation, signing, activation
6. **Dispute & Amendment** — dispute lifecycle, attestation metadata changes

---

## Attestation Lifecycle Events

### `created`
Emitted when a new attestation is created.

- **Topic:** `created`
- **Indexed:** Yes (primary events watched by indexer)
- **Topics in Event:** `(created, subject)`
- **Data:** `(attestation_id, issuer, claim_type, timestamp, metadata)`
- **Subject Filter:** Yes

### `imported`
Emitted when an attestation is imported from an external source.

- **Topic:** `imported`
- **Indexed:** Yes
- **Topics in Event:** `(imported, subject)`
- **Data:** `(attestation_id, issuer, claim_type, timestamp, expiration)`
- **Subject Filter:** Yes

### `bridged`
Emitted when an attestation is bridged from another blockchain.

- **Topic:** `bridged`
- **Indexed:** Yes
- **Topics in Event:** `(bridged, subject)`
- **Data:** `(attestation_id, issuer, claim_type, source_chain, source_tx)`
- **Subject Filter:** Yes

### `revoked`
Emitted when an attestation is revoked by its issuer.

- **Topic:** `revoked`
- **Indexed:** Yes
- **Topics in Event:** `(revoked, issuer)`
- **Data:** `(attestation_id, reason?)`
- **Issuer Filter:** Yes

### `renewed`
Emitted when an attestation's expiration is extended.

- **Topic:** `renewed`
- **Indexed:** Yes
- **Topics in Event:** `(renewed, issuer)`
- **Data:** `(attestation_id, new_expiration?)`

### `updated`
Emitted when an attestation's expiration or status is updated.

- **Topic:** `updated`
- **Indexed:** Yes
- **Topics in Event:** `(updated, issuer)`
- **Data:** `(attestation_id, new_expiration?)`

### `expired`
Emitted when an attestation reaches its expiration time.

- **Topic:** `expired`
- **Indexed:** No (informational)
- **Topics in Event:** `(expired, subject)`
- **Data:** `(attestation_id)`

### `endorsed`
Emitted when a registered issuer endorses an existing attestation.

- **Topic:** `endorsed`
- **Indexed:** No
- **Topics in Event:** `(endorsed, endorser)`
- **Data:** `(attestation_id, timestamp)`

### `amended`
Emitted when an issuer amends the metadata of an existing attestation.

- **Topic:** `amended`
- **Indexed:** No
- **Topics in Event:** `(amended, issuer)`
- **Data:** `(attestation_id, timestamp)`

### `del_req`
Emitted when a subject requests deletion of an attestation.

- **Topic:** `del_req`
- **Indexed:** No
- **Topics in Event:** `(del_req, subject)`
- **Data:** `(attestation_id, timestamp)`

### `xfer`
Emitted when an attestation's issuer is changed by the admin.

- **Topic:** `xfer`
- **Indexed:** No
- **Topics in Event:** `(xfer, old_issuer)`
- **Data:** `(attestation_id, new_issuer)`

### `att_xfer`
Alternate transfer topic (see `xfer` for details).

- **Topic:** `att_xfer`
- **Indexed:** No

---

## Issuer Lifecycle Events

### `iss_reg`
Emitted when an issuer is registered.

- **Topic:** `iss_reg`
- **Indexed:** Yes
- **Topics in Event:** `(iss_reg, issuer)`
- **Data:** `(admin, timestamp)`

### `iss_tier`
Emitted when an issuer's tier is updated.

- **Topic:** `iss_tier`
- **Indexed:** No
- **Topics in Event:** `(iss_tier, issuer)`
- **Data:** `(tier)` — contains tier level and limits

### `iss_rem`
Emitted when an issuer is removed.

- **Topic:** `iss_rem`
- **Indexed:** No
- **Topics in Event:** `(iss_rem, issuer)`
- **Data:** `(admin, timestamp)`

---

## Admin & Governance Events

### `adm_init`
Emitted when the contract is initialized with an admin.

- **Topic:** `adm_init`
- **Indexed:** No
- **Topics in Event:** `(adm_init,)`
- **Data:** `(admin, timestamp)`

### `adm_xfer`
Emitted when admin authority is transferred.

- **Topic:** `adm_xfer`
- **Indexed:** No
- **Topics in Event:** `(adm_xfer,)`
- **Data:** `(old_admin, new_admin)`

### `adm_add`
Emitted when an additional admin is added.

- **Topic:** `adm_add`
- **Indexed:** No
- **Topics in Event:** `(adm_add, by_admin)`
- **Data:** `(new_admin, timestamp)`

### `adm_rem`
Emitted when an admin is removed.

- **Topic:** `adm_rem`
- **Indexed:** No
- **Topics in Event:** `(adm_rem, by_admin)`
- **Data:** `(removed_admin, timestamp)`

### `adm_prop`
Emitted when a proposer initiates an admin transfer proposal.

- **Topic:** `adm_prop`
- **Indexed:** No
- **Topics in Event:** `(adm_prop, current_admin)`
- **Data:** `(new_admin)`

---

## Compliance & Requests

### `clm_type` / `clm_type`
Emitted when a claim type is registered.

- **Topic:** `clm_type`
- **Indexed:** No
- **Topics in Event:** `(clm_type, claim_type)`
- **Data:** `(description)`

### `att_req`
Emitted when a subject submits an attestation request to an issuer.

- **Topic:** `att_req`
- **Indexed:** No
- **Topics in Event:** `(att_req, issuer)`
- **Data:** `(request_id, subject, claim_type, expires_at)`

### `req_ok`
Emitted when an attestation request is fulfilled.

- **Topic:** `req_ok`
- **Indexed:** No
- **Topics in Event:** `(req_ok, issuer)`
- **Data:** `(request_id, attestation_id)`

### `req_no`
Emitted when an attestation request is rejected.

- **Topic:** `req_no`
- **Indexed:** No
- **Topics in Event:** `(req_no, issuer)`
- **Data:** `(request_id, reason?)`

### `req_cncl`
Emitted when a subject cancels their own pending attestation request.

- **Topic:** `req_cncl`
- **Indexed:** No
- **Topics in Event:** `(req_cncl, subject)`
- **Data:** `(request_id)`

### `del_crtd`
Emitted when an issuer creates a delegation to a sub-issuer for a claim type.

- **Topic:** `del_crtd`
- **Indexed:** No
- **Topics in Event:** `(del_crtd, delegator)`
- **Data:** `(delegate, claim_type, expiration?)`

### `del_rvkd`
Emitted when a delegation is revoked.

- **Topic:** `del_rvkd`
- **Indexed:** No
- **Topics in Event:** `(del_rvkd, delegator)`
- **Data:** `(delegate, claim_type)`

### `wl_on`
Emitted when whitelist mode is enabled for an issuer.

- **Topic:** `wl_on`
- **Indexed:** No
- **Topics in Event:** `(wl_on, issuer)`
- **Data:** `()`

### `wl_add`
Emitted when a subject is added to an issuer's whitelist.

- **Topic:** `wl_add`
- **Indexed:** No
- **Topics in Event:** `(wl_add, issuer)`
- **Data:** `(subject)`

### `wl_rem`
Emitted when a subject is removed from an issuer's whitelist.

- **Topic:** `wl_rem`
- **Indexed:** No
- **Topics in Event:** `(wl_rem, issuer)`
- **Data:** `(subject)`

---

## Multi-Sig & Proposals

### `ms_prop`
Emitted when a multi-sig proposal is created.

- **Topic:** `ms_prop`
- **Indexed:** Yes
- **Topics in Event:** `(ms_prop, subject)`
- **Data:** `(proposal_id, proposer, threshold)`

### `ms_sign`
Emitted when a multi-sig proposal is cosigned.

- **Topic:** `ms_sign`
- **Indexed:** Yes
- **Topics in Event:** `(ms_sign, signer)`
- **Data:** `(proposal_id, signatures_so_far, threshold)`

### `ms_actv`
Emitted when a multi-sig proposal is activated (executed).

- **Topic:** `ms_actv`
- **Indexed:** Yes
- **Topics in Event:** `(ms_actv,)`
- **Data:** `(proposal_id, attestation_id)`

### `ms_cancel`
Emitted when a proposer cancels a multi-sig proposal.

- **Topic:** `ms_cancel`
- **Indexed:** No
- **Topics in Event:** `(ms_cancel, proposer)`
- **Data:** `(proposal_id)`

---

## Dispute & Amendment

### `disputed`
Emitted when a subject raises a dispute against an attestation.

- **Topic:** `disputed`
- **Indexed:** No
- **Topics in Event:** `(disputed, subject)`
- **Data:** `(attestation_id, reason, timestamp)`

### `dsp_res`
Emitted when a dispute is resolved.

- **Topic:** `dsp_res`
- **Indexed:** No
- **Topics in Event:** `(dsp_res, resolver)`
- **Data:** `(attestation_id, timestamp)`

---

## Pause & Lifecycle

### `paused`
Emitted when the contract is paused.

- **Topic:** `paused`
- **Indexed:** No
- **Topics in Event:** `(paused,)`
- **Data:** `(admin, timestamp)`

### `unpaused`
Emitted when the contract is unpaused.

- **Topic:** `unpaused`
- **Indexed:** No
- **Topics in Event:** `(unpaused,)`
- **Data:** `(admin, timestamp)`

---

## Templates

### `tmpl_crt`
Emitted when an issuer creates or overwrites a template.

- **Topic:** `tmpl_crt`
- **Indexed:** No
- **Topics in Event:** `(tmpl_crt, issuer)`
- **Data:** `(template_id)`

### `tpl_del`
Emitted when an issuer deletes an attestation template.

- **Topic:** `tpl_del`
- **Indexed:** No
- **Topics in Event:** `(tpl_del, issuer)`
- **Data:** `(template_id)`

---

## Council Governance

### `cncl_ini`
Emitted when the council is initialized.

- **Topic:** `cncl_ini`
- **Indexed:** No
- **Topics in Event:** `(cncl_ini,)`
- **Data:** `(quorum, member_count)`

### `prop_new`
Emitted when a council proposal is created.

- **Topic:** `prop_new`
- **Indexed:** No
- **Topics in Event:** `(prop_new, proposer)`
- **Data:** `(proposal_id)`

### `prop_ok`
Emitted when a council proposal is approved.

- **Topic:** `prop_ok`
- **Indexed:** No
- **Topics in Event:** `(prop_ok, approver)`
- **Data:** `(proposal_id)`

### `prop_exe`
Emitted when a council proposal is executed.

- **Topic:** `prop_exe`
- **Indexed:** No
- **Topics in Event:** `(prop_exe,)`
- **Data:** `(proposal_id)`

### `tl_start`
Emitted when a council proposal reaches quorum, starting the timelock clock.

- **Topic:** `tl_start`
- **Indexed:** No
- **Topics in Event:** `(tl_start,)`
- **Data:** `(proposal_id, quorum_reached_at)`

### `exp_hook`
Emitted when an expiration hook is triggered.

- **Topic:** `exp_hook`
- **Indexed:** No
- **Topics in Event:** `(exp_hook, subject)`
- **Data:** `(attestation_id, expiration)`

---

## Using Event Topics with SDKs

### TypeScript SDK

```typescript
import { TrustLinkClient, EventTopics } from '@trustlink/sdk-typescript';

const client = new TrustLinkClient({
  contractId: 'C...',
  rpcUrl: 'https://soroban-testnet.stellar.org',
});

// Subscribe to specific event topics
const unsubscribe = await client.subscribeToEvents(
  {
    topics: [
      EventTopics.CREATED,
      EventTopics.REVOKED,
      EventTopics.ISS_REG,
    ],
  },
  (event) => {
    console.log('Event:', event);
  }
);
```

### Python SDK

```python
from trustlink import TrustLinkClient, EventTopics

client = TrustLinkClient(
    contract_id='C...',
    rpc_url='https://soroban-testnet.stellar.org',
)

# Subscribe to specific event topics
client.subscribe_to_events(
    topics=[
        EventTopics.CREATED,
        EventTopics.REVOKED,
        EventTopics.ISS_REG,
    ],
    on_event=lambda event: print('Event:', event),
)
```

### GraphQL Subscriptions

```graphql
subscription {
  onAttestationCreated(topics: ["created", "imported", "bridged"]) {
    id
    subject
    issuer
    claimType
  }
}
```

---

## Topic Grouping by Use Case

### For Attestation Lifecycle Monitoring
`created`, `imported`, `bridged`, `revoked`, `renewed`, `updated`, `expired`, `endorsed`, `amended`, `xfer`

### For Issuer Compliance
`iss_reg`, `iss_tier`, `iss_rem`, `wl_on`, `wl_add`, `wl_rem`, `del_crtd`, `del_rvkd`

### For Request Processing
`att_req`, `req_ok`, `req_no`, `req_cncl`

### For Multi-Sig Operations
`ms_prop`, `ms_sign`, `ms_actv`, `ms_cancel`

### For Disputes & Amendments
`disputed`, `dsp_res`, `amended`

### For Administrative Actions
`adm_init`, `adm_xfer`, `adm_add`, `adm_rem`, `adm_prop`, `paused`, `unpaused`

### For Council Governance
`cncl_ini`, `prop_new`, `prop_ok`, `prop_exe`, `tl_start`

---

## Indexer Event Processing

The indexer currently prioritizes the following topics for real-time indexing:

- `created`
- `revoked`
- `imported`
- `bridged`
- `ms_prop`
- `ms_sign`
- `ms_actv`
- `iss_reg`
- `rate_limit_set`

Other events are logged but may not be immediately queryable via GraphQL subscriptions. Use direct ledger event watching via the SDK for comprehensive event monitoring.

---

## Best Practices

1. **Always specify topic filters** when subscribing to reduce bandwidth and client-side processing.
2. **Use topic categories** for related workflows (e.g., subscribe to all attestation lifecycle events together).
3. **Combine with entity filters** (subject, issuer, etc.) when available for fine-grained control.
4. **Monitor indexed topics** for real-time updates; use direct ledger watching for comprehensive history.
5. **Handle duplicate events** — the same event may appear from both indexer and direct ledger watching.

