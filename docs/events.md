# TrustLink Contract Events

Every on-chain event follows the Soroban convention:

```
env.events().publish(topics, data)
```

`topics` is a tuple; `data` is a single value (which may itself be a tuple for
multi-field payloads). All types must implement `TryFromVal` / `IntoVal` —
this rules out `usize`, bare references, and unbounded generics.

---

## Admin events

### `adm_init` — Admin initialized

Emitted once during `initialize`.

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("adm_init")` | Discriminator |
| data.0 | `Address` | Admin address |
| data.1 | `u64` | Ledger timestamp |

### `adm_xfer` — Admin transferred

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("adm_xfer")` | Discriminator |
| data.0 | `Address` | Old admin |
| data.1 | `Address` | New admin |

### `adm_prop` — Admin transfer proposed

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("adm_prop")` | Discriminator |
| topic[1] | `Address` | Current admin |
| data | `Address` | Proposed new admin |

### `adm_add` — Admin added

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("adm_add")` | Discriminator |
| topic[1] | `Address` | Acting admin |
| data.0 | `Address` | New admin |
| data.1 | `u64` | Timestamp |

### `adm_rem` — Admin removed

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("adm_rem")` | Discriminator |
| topic[1] | `Address` | Acting admin |
| data.0 | `Address` | Removed admin |
| data.1 | `u64` | Timestamp |

---

## Issuer events

### `iss_reg` — Issuer registered

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("iss_reg")` | Discriminator |
| topic[1] | `Address` | Issuer address |
| data.0 | `Address` | Admin who registered |
| data.1 | `u64` | Timestamp |

### `iss_tier` — Issuer tier updated

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("iss_tier")` | Discriminator |
| topic[1] | `Address` | Issuer address |
| data | `IssuerTier` | New tier value |

### `iss_rem` — Issuer removed

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("iss_rem")` | Discriminator |
| topic[1] | `Address` | Issuer address |
| data.0 | `Address` | Admin who removed |
| data.1 | `u64` | Timestamp |

### `iss_meta` — Issuer metadata updated

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("iss_meta")` | Discriminator |
| topic[1] | `Address` | Issuer address |
| data.0 | `String` | Name |
| data.1 | `String` | URL |

---

## Bridge events

### `brg_reg` — Bridge registered

Emitted by `register_bridge` after the `Bridge(addr)` key and the
`BridgeList` entry are written. The bridge may then mint `Bridged`
attestations for any subject and claim type.

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("brg_reg")` | Discriminator |
| topic[1] | `Address` | Bridge contract address |
| data.0 | `Address` | Admin who registered the bridge |
| data.1 | `u64` | Timestamp |

### `brg_rem` — Bridge removed

Emitted by `remove_bridge` after the `Bridge(addr)` key and the
`BridgeList` entry are deleted. Subsequent `bridge_attestation` calls from
this address fail with `Unauthorized`.

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("brg_rem")` | Discriminator |
| topic[1] | `Address` | Bridge contract address |
| data.0 | `Address` | Admin who removed the bridge |
| data.1 | `u64` | Timestamp |

---

## Attestation events

### `created` — Attestation created

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("created")` | Discriminator |
| topic[1] | `Address` | Subject |
| data.0 | `String` | Attestation ID |
| data.1 | `Address` | Issuer |
| data.2 | `String` | Claim type |
| data.3 | `u64` | Timestamp |
| data.4 | `Option<u64>` | Expiration |
| data.5 | `Option<String>` | Metadata |

### `imported` — Attestation imported

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("imported")` | Discriminator |
| topic[1] | `Address` | Subject |
| data.0 | `String` | Attestation ID |
| data.1 | `Address` | Issuer |
| data.2 | `String` | Claim type |
| data.3 | `u64` | Timestamp |
| data.4 | `Option<u64>` | Expiration |

### `bridged` — Attestation bridged from another chain

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("bridged")` | Discriminator |
| topic[1] | `Address` | Subject |
| data.0 | `String` | Attestation ID |
| data.1 | `Address` | Issuer |
| data.2 | `String` | Claim type |
| data.3 | `String` | Source chain |
| data.4 | `String` | Source tx |

### `revoked` — Attestation revoked

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("revoked")` | Discriminator |
| topic[1] | `Address` | Issuer |
| data.0 | `String` | Attestation ID |
| data.1 | `Option<String>` | Reason |

### `renewed` — Attestation renewed

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("renewed")` | Discriminator |
| topic[1] | `Address` | Issuer |
| data.0 | `String` | Attestation ID |
| data.1 | `Option<u64>` | New expiration |

### `updated` — Attestation expiration updated

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("updated")` | Discriminator |
| topic[1] | `Address` | Issuer |
| data.0 | `String` | Attestation ID |
| data.1 | `Option<u64>` | New expiration |

### `expired` — Attestation expired

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("expired")` | Discriminator |
| topic[1] | `Address` | Subject |
| data | `String` | Attestation ID |

### `del_req` — Deletion requested

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("del_req")` | Discriminator |
| topic[1] | `Address` | Subject |
| data.0 | `String` | Attestation ID |
| data.1 | `u64` | Timestamp |

### `amended` — Attestation metadata amended

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("amended")` | Discriminator |
| topic[1] | `Address` | Issuer |
| data.0 | `String` | Attestation ID |
| data.1 | `u64` | Timestamp |

### `xfer` — Attestation transferred to new issuer

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("xfer")` | Discriminator |
| topic[1] | `Address` | Old issuer |
| data.0 | `String` | Attestation ID |
| data.1 | `Address` | New issuer |

### `endorsed` — Attestation endorsed

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("endorsed")` | Discriminator |
| topic[1] | `Address` | Endorser |
| data.0 | `String` | Attestation ID |
| data.1 | `u64` | Timestamp |

### `disputed` — Dispute raised

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("disputed")` | Discriminator |
| topic[1] | `Address` | Subject |
| data.0 | `String` | Attestation ID |
| data.1 | `String` | Reason |
| data.2 | `u64` | Timestamp |

### `dsp_res` — Dispute resolved

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("dsp_res")` | Discriminator |
| topic[1] | `Address` | Resolver |
| data.0 | `String` | Attestation ID |
| data.1 | `u64` | Timestamp |

---

## Bundle events

### `bundle` — Bundle created

Emitted by `create_attestation_bundle` when a new multi-claim bundle is
recorded. The `count` field is `u32` (not `usize`) so it is convertible to
`Val` on all Soroban targets.

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("bundle")` | Discriminator |
| topic[1] | `Address` | Subject |
| data.0 | `String` | Bundle ID |
| data.1 | `Address` | Issuer |
| data.2 | `Vec<String>` | Claim types |
| data.3 | `u64` | Timestamp |
| data.4 | `Vec<String>` | Individual attestation IDs |

> **Note (issue #1684):** earlier code had a stray free function that emitted
> a `bundle` event with `count: usize`. `usize` has no `TryFromVal` impl and
> does not compile for `wasm32`. That function has been removed. The canonical
> payload above comes from `Events::bundle_created` inside the `impl Events`
> block, which publishes the full bundle struct and uses only `Val`-compatible
> types.

---

## Claim type events

### `clm_type` — Claim type registered or status changed

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("clm_type")` | Discriminator |
| topic[1] | `String` | Claim type |
| data | `String` \| `ClaimTypeStatus` | Description (on registration) or new status |

### `clm_cons` — Claim type constraints updated

Emitted when `set_claim_type_constraints` or `clear_claim_type_constraints` is
called. The constraints value is cloned before publishing so the event data is
owned and `Val`-compatible.

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("clm_cons")` | Discriminator |
| topic[1] | `String` | Claim type |
| data | `Option<ClaimTypeConstraints>` | New constraints, or `None` when cleared |

> **Note (issue #1684):** an earlier version passed `&Option<ClaimTypeConstraints>`
> (a reference) as the event data value, which is not convertible to `Val`.
> The fix is `constraints.clone()`.

---

## Multi-sig events

### `ms_prop` — Multi-sig proposed

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("ms_prop")` | Discriminator |
| topic[1] | `Address` | Subject |
| data.0 | `String` | Proposal ID |
| data.1 | `Address` | Proposer |
| data.2 | `u32` | Threshold |

### `ms_sign` — Multi-sig co-signed

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("ms_sign")` | Discriminator |
| topic[1] | `Address` | Signer |
| data.0 | `String` | Proposal ID |
| data.1 | `u32` | Signatures so far |
| data.2 | `u32` | Threshold |

### `ms_actv` — Multi-sig activated

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("ms_actv")` | Discriminator |
| data.0 | `String` | Proposal ID |
| data.1 | `String` | Resulting attestation ID |

### `ms_cancel` — Multi-sig proposal cancelled

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("ms_cancel")` | Discriminator |
| topic[1] | `Address` | Proposer |
| data | `String` | Proposal ID |

---

## Attestation request events

### `att_req` — Attestation requested

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("att_req")` | Discriminator |
| topic[1] | `Address` | Issuer |
| data.0 | `String` | Request ID |
| data.1 | `Address` | Subject |
| data.2 | `String` | Claim type |
| data.3 | `u64` | Expires at |

### `req_ok` — Request fulfilled

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("req_ok")` | Discriminator |
| topic[1] | `Address` | Issuer |
| data.0 | `String` | Request ID |
| data.1 | `String` | Attestation ID |

### `req_no` — Request rejected

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("req_no")` | Discriminator |
| topic[1] | `Address` | Issuer |
| data.0 | `String` | Request ID |
| data.1 | `Option<String>` | Reason |

### `req_cncl` — Request cancelled

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("req_cncl")` | Discriminator |
| topic[1] | `Address` | Subject |
| data | `String` | Request ID |

### `req_exp` — Request expired

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("req_exp")` | Discriminator |
| topic[1] | `Address` | Issuer |
| data.0 | `String` | Request ID |
| data.1 | `Address` | Subject |
| data.2 | `u64` | Expires at |

---

## Delegation events

### `del_crtd` — Delegation created

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("del_crtd")` | Discriminator |
| topic[1] | `Address` | Delegator |
| data.0 | `Address` | Delegate |
| data.1 | `String` | Claim type |
| data.2 | `Option<u64>` | Expiration |

### `del_upd` — Delegation updated

Same schema as `del_crtd`.

### `del_rvkd` — Delegation revoked

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("del_rvkd")` | Discriminator |
| topic[1] | `Address` | Delegator |
| data.0 | `Address` | Delegate |
| data.1 | `String` | Claim type |

---

## Whitelist events

### `wl_on` / `wl_off` — Whitelist mode toggled

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("wl_on")` or `Symbol("wl_off")` | Discriminator |
| topic[1] | `Address` | Issuer |
| data | `()` | (empty) |

### `wl_add` / `wl_rem` — Subject added to / removed from whitelist

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("wl_add")` or `Symbol("wl_rem")` | Discriminator |
| topic[1] | `Address` | Issuer |
| data | `Address` | Subject |

---

## Template events

### `tmpl_crt` — Template created

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("tmpl_crt")` | Discriminator |
| topic[1] | `Address` | Issuer |
| data | `String` | Template ID |

### `tmpl_upd` — Template updated

Same schema as `tmpl_crt`.

### `tpl_del` — Template deleted

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("tpl_del")` | Discriminator |
| topic[1] | `Address` | Issuer |
| data | `String` | Template ID |

---

## Council events

### `cncl_ini` — Council initialized

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("cncl_ini")` | Discriminator |
| data.0 | `u32` | Quorum |
| data.1 | `u32` | Member count |

### `prop_new` — Council proposal created

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("prop_new")` | Discriminator |
| topic[1] | `Address` | Proposer |
| data | `u32` | Proposal ID |

### `prop_ok` — Council proposal approved

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("prop_ok")` | Discriminator |
| topic[1] | `Address` | Approver |
| data | `u32` | Proposal ID |

### `prop_exe` — Council proposal executed

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("prop_exe")` | Discriminator |
| data | `u32` | Proposal ID |

### `tl_start` — Council timelock started

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("tl_start")` | Discriminator |
| data.0 | `u32` | Proposal ID |
| data.1 | `u64` | Quorum reached at |

---

## Contract state events

### `paused` — Contract paused

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("paused")` | Discriminator |
| data.0 | `Address` | Admin |
| data.1 | `u64` | Timestamp |

### `unpaused` — Contract unpaused

Same schema as `paused`.

### `exp_hook` — Expiration hook triggered

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("exp_hook")` | Discriminator |
| topic[1] | `Address` | Subject |
| data.0 | `String` | Attestation ID |
| data.1 | `u64` | Expiration |

### `mx_subj` — Max attestations per subject changed

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("mx_subj")` | Discriminator |
| topic[1] | `Address` | Admin |
| data.0 | `&str` | `"max_per_subject"` label |
| data.1 | `u32` | New limit (0 = unlimited) |

### `cfg_set` — Admin configuration changed

Generic config-change event emitted by every admin configuration setter.
Indexers persist these to reconstruct the contract's configuration history.

| Field | Type | Description |
|---|---|---|
| topic[0] | `Symbol("cfg_set")` | Discriminator |
| topic[1] | `Symbol` | Config key (see table below) |
| data.0 | `Address` | Admin who made the change |
| data.1 | `Val` | Old value (`()` / `None` when previously unset) |
| data.2 | `Val` | New value |

#### `cfg_set` keys

| Setter | Key symbol | New value type |
|---|---|---|
| `set_fee` | `fee` | `i128` |
| `set_rate_limit` | `rate_lim` | `RateLimitConfig` |
| `set_rate_limit_for_claim_type` | `rate_clm` | `(String, RateLimitConfig)` |
| `set_limits` | `limits` | `Limits` |
| `set_chunk_size` | `chunk` | `u32` |
| `set_decay_config` | `decay` | `DecayConfig` |
| `set_council_timelock_delay` | `tl_delay` | `u64` |
| `set_registered_claim_type` | `clm_reg` | `(String, bool)` |
| `set_metadata_hash_only` | `meta_hsh` | `bool` |
| `set_claim_type_constraints` | `clm_cons` | `(String, Option<ClaimTypeConstraints>)` |
| `set_multisig_ttl` | `ms_ttl` | `u64` |
| `register_bridge` | `brg_reg` | `Address` |
