# Cross-border trade finance

This example models a bill-of-lading attestation chain for one shared shipment
reference. The exporter issues `SHIPMENT_CONFIRMED`, the customs authority issues
`CUSTOMS_CLEARED`, and the financing bank issues `FINANCING_CONDITIONS_MET`.

The verifier uses `has_all_claims`, so the shipment is trade-finance-clear only
while all three attestations exist and remain unrevoked. Each role is a distinct
authorized issuer; the example also revokes the customs attestation to show that
clearance is immediately lost.

Run it with:

```sh
cargo test --example trade_finance
```
