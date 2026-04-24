# ADR-005: Prevent Admin from Setting Themselves as Fee Collector

- **Status**: Accepted
- **Date**: 2024-04-23
- **Related Issues**: [#255](https://github.com/Haroldwonder/TrustLink/issues/255)

## Context

The TrustLink contract allows an admin to configure the fee collection mechanism via the `set_fee()` function, which accepts:
- `fee`: The attestation fee amount (in stroops or token units)
- `collector`: The address that receives collected fees
- `fee_token`: Optional token contract address (required if fee > 0)

**Problem**: An admin could set themselves as the `fee_collector`, creating a potential conflict of interest and transparency risk. If the admin is also the fee collector, they can:
1. Extract fees from issuers without explicit disclosure
2. Make fee collection decisions that benefit themselves personally
3. Create ambiguity about whether fees are genuinely needed or a form of hidden extraction

This violates the principle of **separation of concerns** and introduces a **transparency risk** in the attestation network.

## Decision

**Hard Block**: The `set_fee()` function will explicitly reject any attempt to set `fee_collector == admin` by returning `Error::Unauthorized`.

```rust
pub fn set_fee(
    env: Env,
    admin: Address,
    fee: i128,
    collector: Address,
    fee_token: Option<Address>,
) -> Result<(), Error> {
    admin.require_auth();
    Validation::require_admin(&env, &admin)?;
    
    // NEW: Prevent admin from setting themselves as fee_collector
    if admin == collector {
        return Err(Error::Unauthorized);
    }
    
    validate_fee_config(fee, &fee_token)?;
    Storage::set_fee_config(&env, &FeeConfig { ... });
    Ok(())
}
```

### Design Rationale

**Hard block** was chosen over a **warning event** for the following reasons:

1. **Principle of Least Privilege**: The admin should not have the ability to become the fee collector. These are distinct roles with different responsibilities.
2. **Clarity**: A hard block makes the policy explicit and enforceable at the contract level, rather than relying on off-chain governance or audits.
3. **Transparency**: By preventing the admin from collecting fees, the contract ensures that all fee flows are transparent and can be separately audited.
4. **Precedent**: Similar role separation exists in other web3 systems (e.g., OpenZeppelin's `Ownable2Step` separates ownership from access control).

A warning event alone would allow the misconfiguration to proceed, creating ongoing confusion and trust issues in the network.

## Consequences

**Positive**
- Clear separation between administrative authority and financial benefit
- Prevents hidden fee extraction that could erode network trust
- Enforces transparent fee collection: fees must go to a separate, designated recipient
- Reduces governance disputes about whether fees are justified or self-dealing

**Negative**
- Adds a small constraint on configuration flexibility (one additional validation check)
- If an admin genuinely wants to collect fees themselves, they must revoke their admin status first, then re-request it under a different address (unlikely but possible if needed for migration)

**Neutral**
- The check is fast (simple address comparison) and has negligible performance impact
- The error code (`Unauthorized`) reuses an existing enum value; no new error type needed

## Implementation

**File**: [`src/lib.rs`](../../src/lib.rs) — `set_fee()` function  
**Error Code**: `Error::Unauthorized` (code 3)  
**Changes**: 2 lines added for the check + comment

The restriction applies to **all** admin addresses in a multi-admin council; any admin attempting to set themselves as collector will be rejected.

## Testing

Tests verify:
- Admin can set a different address as `fee_collector` ✓
- Admin is rejected if trying to set themselves ✓
- The error code is `Unauthorized` ✓
- Multiple admins in council are each individually blocked from self-collection ✓

## References

- **Issue #255**: [Security: Prevent admin from setting themselves as fee_collector](https://github.com/Haroldwonder/TrustLink/issues/255)
- **Pattern**: Separation of Concerns + Principle of Least Privilege
- **Related**: [OpenZeppelin Ownable2Step](https://github.com/OpenZeppelin/openzeppelin-contracts/blob/master/contracts/access/Ownable2Step.sol)
