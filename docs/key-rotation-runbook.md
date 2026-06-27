# Key Rotation Runbook

This runbook covers the operational procedures for rotating admin and council signer keys in TrustLink. All privileged operations are performed through the on-chain contract; there are no off-chain secrets to rotate beyond the Stellar keypairs themselves.

## Background

TrustLink uses two overlapping admin primitives:

- **Admin council** — an ordered list of admin addresses (`AdminCouncil`). Any address in the council can call admin-only functions. The council can contain one or more members; the last member cannot be removed.
- **Two-step admin transfer** — a proposer nominates a new admin (`propose_admin_transfer`) and the nominee explicitly accepts (`accept_admin_transfer`). Until accepted, the old admin retains control. This is the safe default for planned rotations.
- **Single-step transfer** — `transfer_admin` atomically replaces the calling admin with a new address. Use only in emergencies when the nominee is on the same signing session.

Contract entry points involved in key rotation:

| Function | Who calls it | Effect |
|---|---|---|
| `propose_admin_transfer(current_admin, new_admin)` | Current admin | Records a pending transfer; does not yet grant access |
| `accept_admin_transfer(new_admin)` | Incoming admin | Removes old admin from council, adds new admin |
| `cancel_admin_transfer(current_admin)` | Current admin | Cancels a pending transfer |
| `transfer_admin(current_admin, new_admin)` | Current admin | Immediate single-step swap (no confirmation) |
| `add_admin(existing_admin, new_admin)` | Any current admin | Adds a member to the council without removing anyone |
| `remove_admin(existing_admin, admin_to_remove)` | Any current admin | Removes a council member (at least one must remain) |

---

## Planned Rotation (Recommended)

Use this procedure for routine key rotation — e.g., after a staff change or as part of a scheduled security cadence.

### Step 1 — Generate the new keypair

Generate the replacement Stellar keypair on an air-gapped machine or HSM:

```bash
stellar keys generate --global new-admin-key --network mainnet
stellar keys address new-admin-key
# Record the public key (G...) as NEW_ADMIN
```

Fund the account on-chain if it does not yet exist (minimum reserve):

```bash
stellar account fund --network mainnet NEW_ADMIN
```

### Step 2 — Propose the transfer

The current admin proposes the transfer:

```bash
stellar contract invoke \
  --id CONTRACT_ID \
  --source CURRENT_ADMIN_KEY \
  --network mainnet \
  -- \
  propose_admin_transfer \
  --current_admin CURRENT_ADMIN \
  --new_admin NEW_ADMIN
```

Verify the pending transfer was recorded:

```bash
stellar contract invoke \
  --id CONTRACT_ID \
  --source CURRENT_ADMIN_KEY \
  --network mainnet \
  -- \
  get_pending_admin_transfer
```

### Step 3 — Accept from the new keypair

The incoming admin signs the acceptance from the new keypair:

```bash
stellar contract invoke \
  --id CONTRACT_ID \
  --source new-admin-key \
  --network mainnet \
  -- \
  accept_admin_transfer \
  --new_admin NEW_ADMIN
```

After this call:
- `NEW_ADMIN` is a member of the admin council.
- `CURRENT_ADMIN` is removed from the admin council.
- The pending transfer record is deleted.

### Step 4 — Verify

Confirm the new admin is in place:

```bash
stellar contract invoke \
  --id CONTRACT_ID \
  --source new-admin-key \
  --network mainnet \
  -- \
  get_admin
```

Test that the old keypair is rejected:

```bash
stellar contract invoke \
  --id CONTRACT_ID \
  --source OLD_ADMIN_KEY \
  --network mainnet \
  -- \
  get_admin
# Expected: Unauthorized or similar error
```

### Cancelling a Pending Rotation

If the rotation needs to be aborted before the new admin accepts:

```bash
stellar contract invoke \
  --id CONTRACT_ID \
  --source CURRENT_ADMIN_KEY \
  --network mainnet \
  -- \
  cancel_admin_transfer \
  --current_admin CURRENT_ADMIN
```

---

## Emergency Rotation (Compromised Key)

Use this procedure when a key is suspected to be compromised and you need to revoke access immediately.

### Prerequisites

- At least one other valid admin keypair is available (council member).
- If no other council member exists, see [Single Surviving Admin](#single-surviving-admin) below.

### Step 1 — Add a safe replacement admin immediately

Using any uncompromised council member, add the replacement admin without waiting for a two-step confirmation:

```bash
stellar contract invoke \
  --id CONTRACT_ID \
  --source SAFE_ADMIN_KEY \
  --network mainnet \
  -- \
  add_admin \
  --existing_admin SAFE_ADMIN \
  --new_admin REPLACEMENT_ADMIN
```

### Step 2 — Remove the compromised admin

```bash
stellar contract invoke \
  --id CONTRACT_ID \
  --source SAFE_ADMIN_KEY \
  --network mainnet \
  -- \
  remove_admin \
  --existing_admin SAFE_ADMIN \
  --admin_to_remove COMPROMISED_ADMIN
```

`remove_admin` fails with `LastAdminCannotBeRemoved` if this would leave the council empty. If that occurs, add the replacement first (Step 1), then remove the compromised key.

### Step 3 — Pause the contract (optional)

While the incident is contained, consider pausing write operations to prevent the compromised key from creating attestations in the window between compromise and removal:

```bash
stellar contract invoke \
  --id CONTRACT_ID \
  --source SAFE_ADMIN_KEY \
  --network mainnet \
  -- \
  pause \
  --admin SAFE_ADMIN
```

Unpause once the rotation is confirmed:

```bash
stellar contract invoke \
  --id CONTRACT_ID \
  --source REPLACEMENT_ADMIN_KEY \
  --network mainnet \
  -- \
  unpause \
  --admin REPLACEMENT_ADMIN
```

### Step 4 — Audit recent activity

Check events on-chain for any attestations or issuer changes made by the compromised key after the suspected compromise time. Use the Stellar network's event streaming or a block explorer to filter events by the compromised address.

### Single Surviving Admin

If the compromised keypair is the only admin (council size = 1), `remove_admin` will fail. The only recovery path is to use the compromised key one last time to add a safe replacement before revoking it:

1. Ensure the compromised key has not yet been used maliciously since detection.
2. If the key material is still accessible (e.g., rotated out but not yet leaked), perform a normal planned rotation (Step 2–4 above) using the compromised key as `current_admin`.
3. If key material is fully unavailable, the contract admin seat is permanently locked. Contact the Stellar development foundation for guidance on contract recovery options available at the network level.

---

## Council Member Replacement

Use this procedure when rotating a council member's key without changing overall council composition — for example, when a team member leaves and their seat is taken by their successor.

### Step 1 — Add the new member

```bash
stellar contract invoke \
  --id CONTRACT_ID \
  --source ANY_CURRENT_ADMIN_KEY \
  --network mainnet \
  -- \
  add_admin \
  --existing_admin ANY_CURRENT_ADMIN \
  --new_admin NEW_MEMBER
```

`add_admin` is idempotent: if `NEW_MEMBER` is already in the council, it returns without error.

### Step 2 — Remove the outgoing member

```bash
stellar contract invoke \
  --id CONTRACT_ID \
  --source ANY_CURRENT_ADMIN_KEY \
  --network mainnet \
  -- \
  remove_admin \
  --existing_admin ANY_CURRENT_ADMIN \
  --admin_to_remove OUTGOING_MEMBER
```

### Step 3 — Verify council state

The contract does not currently expose a `get_admin_council` query; verify by attempting a privileged call with each expected member key and confirming the outgoing key is rejected.

---

## Post-Rotation Checklist

After any key rotation:

- [ ] New admin key verified to be functional (test call succeeded).
- [ ] Old admin key verified to be rejected (test call returned Unauthorized).
- [ ] Keypair for old admin securely destroyed or archived per your key management policy.
- [ ] Rotation event recorded in your change-management system.
- [ ] All team members informed of the new admin address.
- [ ] Off-chain tooling (deploy scripts, CI secrets, monitoring alerts) updated to reference the new keypair.
- [ ] If paused during emergency: contract unpaused and issuer operations verified.

## Related Documentation

- [Security Policy](security.md)
- [Mainnet Runbook](mainnet-runbook.md)
- [Compliance](compliance.md)
