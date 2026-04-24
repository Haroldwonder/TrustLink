# TrustLink Security Model

This document describes the trust hierarchy, threat model, known limitations, and
operational security recommendations for TrustLink. It is intended for auditors,
integrators, and operators deploying the contract in production.

For the line-by-line authorization audit performed before mainnet, see
[docs/security-review.md](security-review.md).

---

## Trust Hierarchy

TrustLink has three principal roles. Each role is strictly scoped — a principal
can only perform the actions listed for their role.

```
┌─────────────────────────────────────────────────────┐
│                       Admin                         │
│  Single address stored in instance storage.         │
│  Controls the issuer registry and contract config.  │
└──────────────────────┬──────────────────────────────┘
                       │ registers / removes
          ┌────────────▼────────────┐
          │        Issuers          │
          │  Registered addresses.  │
          │  Create attestations.   │
          └────────────┬────────────┘
                       │ attest about
          ┌────────────▼────────────┐
          │        Subjects         │
          │  Any Stellar address.   │
          │  Passive recipients.    │
          └─────────────────────────┘
```

### Admin

The admin is a single Stellar address stored in instance storage at the `Admin`
key. It is set once during `initialize` and can be transferred atomically via
`transfer_admin`.

**What the admin can do:**

| Action | Function |
|--------|----------|
| Register a new issuer | `register_issuer` |
| Remove an existing issuer | `remove_issuer` |
| Assign or update an issuer's trust tier | `update_issuer_tier` |
| Register a bridge contract | `register_bridge` |
| Import a historical attestation on behalf of a registered issuer | `import_attestation` |
| Register a claim type with a description | `register_claim_type` |
| Set the attestation fee, collector, and fee token | `set_fee` |
| Transfer admin rights to a new address | `transfer_admin` |
| Pause all write operations during an incident | `pause` |
| Resume write operations after an incident | `unpause` |

**What the admin cannot do:**

- Create a native attestation directly — `create_attestation` requires a
  registered issuer, and the admin is not automatically an issuer.
- Revoke an attestation they did not issue — revocation is scoped to the
  original issuer of each attestation.
- Modify or delete an existing attestation's content — attestations are
  immutable once written; only the `revoked` flag and `expiration` field can
  be updated, and only by the original issuer.
- Impersonate an issuer — `require_auth` is called on the actual transaction
  signer; passing a different address as a parameter does not grant that
  address's permissions.
- Bypass the pause — `pause` and `unpause` are the only mechanism; there is no
  back-door that skips the `require_not_paused` check.

### Issuers

Issuers are Stellar addresses present in the persistent `Issuer(Address)` registry.
Membership is controlled exclusively by the admin.

**What issuers can do:**

| Action | Function |
|--------|----------|
| Create a native attestation about a subject | `create_attestation` |
| Create attestations in bulk | `create_attestations_batch` |
| Revoke an attestation they originally issued | `revoke_attestation`, `revoke_attestations_batch` |
| Renew the expiration of their own attestation | `renew_attestation` |
| Update the expiration of their own attestation | `update_expiration` |
| Propose a multi-sig attestation | `propose_attestation` |
| Co-sign a multi-sig proposal | `cosign_attestation` |
| Endorse another issuer's attestation | `endorse_attestation` |
| Set their own public metadata | `set_issuer_metadata` |

**What issuers cannot do:**

- Attest about themselves — `create_attestation` rejects calls where
  `issuer == subject` with `Error::Unauthorized`.
- Revoke another issuer's attestation — ownership is checked against the stored
  `attestation.issuer` field.
- Issue attestations after being de-registered — `require_issuer` checks the
  live registry on every call; removal takes effect immediately.
- Issue attestations while the contract is paused — `require_not_paused` is
  checked before `require_issuer` in both `create_attestation` and
  `revoke_attestation`.

### Subjects

Subjects are the addresses that attestations are *about*. They are passive
recipients and have no privileged role in the contract. Any Stellar address can
be a subject.

Subjects can:
- Register an expiration notification hook for their own address
  (`register_expiration_hook`).
- Remove their own hook (`remove_expiration_hook`).

Subjects cannot create, modify, or revoke attestations about themselves.

### Bridge Contracts

Bridge contracts are a separate registry (`Bridge(Address)`) from issuers.
A registered bridge can call `bridge_attestation` to mirror an attestation from
another chain. The bridge address becomes the on-chain `issuer` of the resulting
attestation, and the original chain and transaction reference are stored in
`source_chain` / `source_tx`.

Bridge contracts cannot call any issuer-only functions, and issuers cannot call
bridge-only functions.

---

## Admin Check Implementation

The admin check is implemented in `Validation::require_admin` in
`src/validation.rs`:

```rust
pub fn require_admin(env: &Env, caller: &Address) -> Result<(), Error> {
    let admin = Storage::get_admin(env)?;   // reads from storage
    if caller != &admin {
        return Err(Error::Unauthorized);
    }
    Ok(())
}
```

The stored value is the source of truth. Passing a different address as the
`admin` parameter to any function does not grant admin privileges — the
parameter is only used to call `require_auth()` on the transaction signer, and
then immediately compared against the stored admin. There is no way to bypass
this check by manipulating the parameter.

---

## Threat Model

### Attacks Prevented

**Unauthorized attestation issuance**

Only addresses in the issuer registry can call `create_attestation`. The check
is `Storage::is_issuer(env, caller)` — a persistent storage key lookup — not a
parameter comparison. An attacker cannot pass a registered issuer's address as a
parameter and gain their privileges; Soroban's `require_auth` enforces that the
actual transaction signer matches the address.

**Self-attestation / self-certification**

`create_attestation` explicitly rejects calls where `issuer == subject`:

```rust
if issuer == subject {
    return Err(Error::Unauthorized);
}
```

An issuer cannot issue a credential about themselves.

**Replay attacks**

Attestation IDs are deterministic SHA-256 hashes of `(issuer, subject,
claim_type, timestamp)`. The contract checks `Storage::has_attestation` before
writing and returns `Error::DuplicateAttestation` if the ID already exists.
Because the timestamp is the ledger timestamp at the time of the call, two
identical calls in different ledgers produce different IDs. Two identical calls
in the same ledger are rejected as duplicates.

Bridge attestation IDs additionally include `source_chain` and `source_tx` in
the hash, making cross-chain replay impossible even if the same transaction is
submitted twice.

**Admin impersonation**

`require_auth()` is called on the address passed as the `admin` parameter before
any state is read or written. Soroban's auth framework verifies the transaction
signature matches that address. The stored admin is then compared against the
parameter. Both checks must pass.

**Issuer bypass after de-registration**

`require_issuer` reads the live registry on every call. Removing an issuer via
`remove_issuer` takes effect immediately — subsequent calls from that address
will fail with `Error::Unauthorized` even if they hold a valid signature.

**Unauthorized revocation**

Revocation requires both:
1. The caller to be a registered issuer (`require_issuer`).
2. The caller to be the original issuer of the specific attestation
   (`attestation.issuer != issuer` check).

Neither condition alone is sufficient.

**Unauthorized admin transfer**

`transfer_admin` requires the *current* admin's signature (`current_admin.require_auth()`)
and validates against the stored admin (`Validation::require_admin`). A new
admin address cannot be installed without the current admin's private key.

**Incident response — emergency pause**

The admin can call `pause()` to immediately halt all attestation write
operations (`create_attestation`, `revoke_attestation`). Read functions remain
available so integrators can continue verifying existing attestations during an
incident. The pause state is stored in instance storage and checked atomically
at the start of every write path.

**Multi-sig collusion prevention**

High-value attestations can require M-of-N registered issuers to co-sign via
`propose_attestation` / `cosign_attestation`. This prevents a single compromised
issuer key from unilaterally issuing sensitive credentials. Proposals expire
after 7 days if the threshold is not reached.

**Endorsement abuse**

An issuer cannot endorse their own attestation (`CannotEndorseOwn`), cannot
endorse a revoked attestation (`AlreadyRevoked`), and can only endorse each
attestation once (`AlreadyEndorsed`). Endorsements are social proof only and do
not affect the validity status returned by `has_valid_claim`.

### Attacks Not Prevented (Known Limitations)

See the [Known Limitations](#known-limitations) section below.

---

## Known Limitations

These are honest assessments of what the contract does not protect against.
Operators should account for these in their deployment and operational design.

**1. Admin key compromise is catastrophic**

There is a single admin address. If the admin private key is compromised, an
attacker can:
- Register arbitrary issuers.
- Import fabricated historical attestations.
- Register malicious bridge contracts.
- Change the fee collector to drain issuer funds.
- Pause the contract indefinitely.

There is no multi-sig or time-lock on admin operations. Mitigation: use a
hardware wallet or multisig account (e.g. a Stellar multisig account) as the
admin address. See [Operational Security](#operational-security) below.

**2. Admin can import fabricated attestations**

`import_attestation` allows the admin to write an attestation with an arbitrary
historical timestamp and any registered issuer as the attributed author. There
is no cryptographic proof that the imported attestation was ever issued
off-chain. Integrators that need to distinguish native from imported attestations
should check the `imported: bool` field on the `Attestation` struct.

**3. Issuer key compromise**

A compromised issuer key can issue arbitrary attestations for any subject until
the admin calls `remove_issuer`. There is no rate limiting or per-issuer
attestation cap. Mitigation: monitor issuer activity via events and have an
incident response plan that includes calling `remove_issuer` promptly.

**4. No on-chain claim type validation**

`create_attestation` accepts any string as `claim_type`. The claim type registry
(`register_claim_type`) is informational only — it does not gate attestation
creation. An issuer can create attestations with unregistered or misspelled claim
types. Integrators should validate claim type strings against the registry
off-chain or use `get_claim_type_description` to confirm a type is registered
before trusting it.

**5. Metadata is unverified**

The `metadata` field on an attestation is a free-form string supplied by the
issuer. The contract enforces a 256-character length limit but does not validate
the content. Integrators must not make security decisions based on metadata
content without independent verification.

**6. Storage TTL expiry**

All persistent storage entries have a TTL (default 30 days, configurable).
If the contract is not interacted with for longer than the TTL, storage entries
may be evicted by the Stellar network. Operators must ensure regular interaction
or TTL extension to keep critical data alive. The TTL is refreshed on every read
and write, so active attestations are unlikely to expire, but dormant ones may.

**7. Expiration hook callback trust**

When `has_valid_claim` triggers an expiration hook, it calls an arbitrary
external contract (`notify_expiring`). The call is best-effort — failures are
silently swallowed. However, a malicious or buggy callback contract could
consume significant compute budget, potentially causing the outer `has_valid_claim`
call to run out of resources. Subjects should only register hooks pointing to
contracts they control and trust.

**8. Bridge contract trust is binary**

A registered bridge contract has unconditional authority to create attestations
for any subject with any claim type. There is no per-bridge claim type
restriction. Operators should register only bridge contracts whose source-chain
verification logic they have audited.

**9. No subject consent**

Subjects have no mechanism to reject or dispute an attestation issued about
them. Any registered issuer can attest about any address. This is by design for
permissionless verification flows, but operators building consent-based systems
must implement consent logic in their application layer.

**10. `create_attestations_batch` is not pause-gated**

`create_attestations_batch` does not call `require_not_paused`. Only
`create_attestation` (single) and `revoke_attestation` (single) are pause-gated.
If the contract is paused to stop a compromised issuer, that issuer could still
use the batch function. This is a known gap that should be addressed before
mainnet.

---

## Operational Security

### Admin Key

The admin key is the highest-privilege credential in the system. Treat it
accordingly.

- **Use a hardware wallet.** The admin address should be controlled by a
  hardware security module (HSM) or hardware wallet (e.g. Ledger). Never store
  the admin private key on an internet-connected machine.

- **Consider a Stellar multisig account.** Stellar natively supports M-of-N
  multisig at the account level. Setting the admin address to a multisig account
  (e.g. 2-of-3) means no single key compromise can take over the contract.
  Use `transfer_admin` to migrate to a multisig account after deployment.

- **Separate deployment and operation keys.** The key used to deploy and
  initialize the contract should not be the long-term admin key. Transfer admin
  to a cold key immediately after initialization.

- **Store the admin address publicly.** The admin address is readable on-chain
  via `get_admin`. Document it in your deployment registry so auditors and
  integrators can verify it matches your stated key management policy.

- **Have a key rotation plan.** Know in advance how you will execute
  `transfer_admin` if the current admin key is suspected to be compromised.
  Test the rotation procedure on testnet before mainnet deployment.

### Issuer Key Management

- Treat issuer keys as high-value credentials. A compromised issuer key can
  issue fraudulent attestations until the admin revokes it.
- Monitor issuer activity via on-chain events (`iss_reg`, `created`, `revoked`).
  Set up alerting for unexpected issuance volume or unusual claim types.
- Rotate issuer keys periodically: call `remove_issuer` on the old address and
  `register_issuer` on the new one. Note that existing attestations issued by
  the old key remain valid — they are not retroactively invalidated.
- For high-value claim types, use multi-sig proposals (`propose_attestation` /
  `cosign_attestation`) to require M-of-N issuer agreement.

### Incident Response

If you suspect a key compromise or fraudulent attestation activity:

1. **Pause the contract immediately** — call `pause(admin)`. This halts all new
   attestation creation and revocation while reads remain available.
2. **Remove the compromised issuer** — call `remove_issuer(admin, issuer)`.
   This prevents further issuance from that key even after unpausing.
3. **Audit recent attestations** — query events from the compromised issuer's
   address and revoke any fraudulent attestations using `revoke_attestation`.
4. **Rotate the admin key if needed** — if the admin key itself is suspected,
   call `transfer_admin` to a new secure address before taking other actions.
5. **Unpause** — call `unpause(admin)` once the threat is contained.

### Deployment Checklist

Before deploying to mainnet:

- [ ] Admin address is a hardware wallet or multisig account.
- [ ] Deployment key is different from the long-term admin key.
- [ ] `transfer_admin` has been tested on testnet.
- [ ] All initial issuers have been reviewed and their key management confirmed.
- [ ] TTL configuration is appropriate for your expected interaction frequency.
- [ ] Fee configuration (if any) has been reviewed — the fee collector address
      is correct and the fee token contract is trusted.
- [ ] Bridge contracts (if any) have been audited.
- [ ] An incident response runbook exists and has been rehearsed.
- [ ] Event monitoring and alerting is in place.
- [ ] The `create_attestations_batch` pause gap (see Known Limitations §10) has
      been assessed and accepted or patched.

---

## Attestation ID Scheme

Attestation IDs are deterministic SHA-256 hashes encoded as 64-character
lowercase hex strings. The pre-image is the XDR serialization of
`(issuer, subject, claim_type, timestamp)` concatenated in that order.

This means:
- The same issuer cannot issue the same claim type to the same subject twice
  within the same ledger (same timestamp → same ID → `DuplicateAttestation`).
- IDs are reproducible off-chain for indexing without querying the contract.
- IDs are not sequential and do not leak the total number of attestations.

Bridge attestation IDs additionally include `source_chain` and `source_tx` in
the hash, ensuring uniqueness across chains.

---

## Audit Trail

Every state-changing operation appends an immutable entry to the attestation's
audit log (`AuditLog(attestation_id)` in persistent storage). Entries record the
action (`Created`, `Revoked`, `Renewed`, `Updated`), the actor address, the
ledger timestamp, and optional details (e.g. revocation reason).

The audit log is append-only by design — there is no function to modify or
delete entries. It provides a tamper-evident history of every change to an
attestation's lifecycle.

---

*Last updated: 2026-03-25. Reflects contract version 1.0.0.*
