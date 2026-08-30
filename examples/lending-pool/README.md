# Lending Pool — IssuerTier-Gated Reference Integration

A minimal collateralized lending pool that queries TrustLink for the borrower's
`CREDITWORTHY` attestation, resolves the attesting issuer's **IssuerTier**, and
adjusts loan-to-value (LTV) and liquidation thresholds accordingly.

This is the runnable DeFi reference for attestation-gated risk parameters —
complementing the simpler KYC-only lending mock in `tests/integration_test.rs`.

## What It Demonstrates

1. **Deposit** — lenders add borrowable liquidity to the pool.
2. **Collateral + borrow** — borrowers post collateral and draw debt up to a
   tier-specific max LTV.
3. **Liquidate** — underwater positions (debt/collateral above the tier's
   liquidation threshold) can be liquidated.

## Tier → Risk Parameters

| IssuerTier | Max LTV | Liquidation threshold |
|------------|---------|------------------------|
| Basic      | 50%     | 60%                    |
| Verified   | 65%     | 75%                    |
| Premium    | 80%     | 90%                    |

Borrowers without a valid `CREDITWORTHY` attestation (or whose attestation is
revoked) cannot borrow.

## Contract Pattern

```rust
let attestation = trustlink.get_attestation_by_type(&borrower, &claim)?;
let tier = trustlink.get_issuer_tier(&attestation.issuer)?;
let params = match tier {
    IssuerTier::Basic => /* 50% LTV / 60% liq */,
    IssuerTier::Verified => /* 65% / 75% */,
    IssuerTier::Premium => /* 80% / 90% */,
};
```

Higher-tier issuers unlock higher leverage and wider liquidation buffers,
reflecting greater confidence in the borrower's attested creditworthiness.

## Files

- `src/lib.rs` — lending-pool contract, TrustLink client trait, and unit tests
- `Cargo.toml` — example crate configuration

## Run Tests

```bash
cd examples/lending-pool
cargo test
```

The suite covers:

| Scenario | Test |
|---|---|
| Tier parameter table | `tier_params_match_spec` |
| Borrow blocked without attestation | `borrow_rejected_without_attestation` |
| Basic 50% LTV ceiling | `basic_tier_enforces_50_percent_ltv` |
| Verified 65% LTV | `verified_tier_ltv_between_basic_and_premium` |
| Premium 80% LTV | `premium_tier_allows_higher_ltv_than_basic` |
| Healthy position not liquidatable | `liquidate_rejects_healthy_position` |
| Full deposit → borrow → liquidate | `full_deposit_borrow_liquidate_flow` |
| Premium survives drop that liquidates Basic | `premium_survives_price_drop_that_liquidates_basic` |
| Revoked attestation blocks borrow | `revoked_attestation_blocks_borrow` |

## Flow Walkthrough

```text
Lender                Pool                 Borrower              TrustLink
  |                    |                      |                      |
  |-- deposit(5_000) ->|                      |                      |
  |                    |<- deposit_collateral-|                      |
  |                    |                      |-- get_attestation -->|
  |                    |                      |<-- CREDITWORTHY ------|
  |                    |                      |-- get_issuer_tier -->|
  |                    |                      |<-- Basic/Verified/...|
  |                    |<------ borrow -------|                      |
  |                    |   (LTV from tier)    |                      |
  |                    |   ...price drop...   |                      |
  |                    |<---- liquidate ------| (liquidator)         |
```

## Build WASM

```bash
cd examples/lending-pool
cargo build --target wasm32-unknown-unknown --release
```

## Extending

- Wire a price oracle so liquidation reacts to collateral mark-to-market
  instead of unit adjustments in tests.
- Require `has_valid_claim_from_tier(borrower, "CREDITWORTHY", Verified)` as a
  hard gate before any borrow.
- Combine with `KYC_PASSED` via `has_all_claims` for regulated markets.
