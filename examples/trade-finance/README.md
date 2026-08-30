# Trade Finance Example

This example demonstrates how to integrate TrustLink into a trade finance platform. Trade credit is only issued when both the merchant and financial institution have been verified through TrustLink attestations.

## Use Case

Trade finance platforms need to verify the identity and credibility of both merchants seeking credit and financial institutions providing it. This example shows how to gate trade credit issuance behind multi-party attestation verification via TrustLink.

## Features

- Require merchant verification (`MERCHANT_VERIFIED` attestation)
- Require financial institution licensing (`FINANCIAL_INSTITUTION_LICENSED` attestation)
- Gate trade credit issuance behind dual attestation checks
- Support for compliance and audit trails

## Building

```bash
cargo build --target wasm32-unknown-unknown --release
```

## Testing

```bash
cargo test
```

## Integration

This contract expects to be called with the addresses of both a merchant and a financial institution, along with the address of a deployed TrustLink instance. Both parties must hold valid attestations before trade credit can be issued.

### API

```rust
pub fn issue_trade_credit(
    env: Env,
    merchant: Address,
    financial_institution: Address,
    trustlink_id: Address,
    amount: i128,
) -> Result<bool, String>
```

**Parameters:**
- `merchant`: The merchant address seeking trade credit (must have `MERCHANT_VERIFIED` attestation)
- `financial_institution`: The financial institution address providing credit (must have `FINANCIAL_INSTITUTION_LICENSED` attestation)
- `trustlink_id`: The contract ID of the deployed TrustLink instance
- `amount`: The amount of trade credit to issue

**Returns:** `true` if trade credit issuance is successful

## Example Usage

```bash
# Deploy TrustLink first (see main README)
TRUSTLINK_CONTRACT_ID=C...

# Register merchant issuer
soroban contract invoke \
  --id $TRUSTLINK_CONTRACT_ID \
  --source admin \
  -- register_issuer \
  --issuer MERCHANT_VERIFICATION_AUTHORITY

# Register financial institution issuer
soroban contract invoke \
  --id $TRUSTLINK_CONTRACT_ID \
  --source admin \
  -- register_issuer \
  --issuer FINANCIAL_REGULATOR

# Issue merchant verification attestation
soroban contract invoke \
  --id $TRUSTLINK_CONTRACT_ID \
  --source merchant_authority \
  -- create_attestation \
  --issuer MERCHANT_VERIFICATION_AUTHORITY \
  --subject MERCHANT_ADDRESS \
  --claim_type MERCHANT_VERIFIED \
  --expiration null

# Issue financial institution licensing attestation
soroban contract invoke \
  --id $TRUSTLINK_CONTRACT_ID \
  --source regulator \
  -- create_attestation \
  --issuer FINANCIAL_REGULATOR \
  --subject FINANCIAL_INSTITUTION_ADDRESS \
  --claim_type FINANCIAL_INSTITUTION_LICENSED \
  --expiration null

# Now trade credit can be issued
soroban contract invoke \
  --id TRADE_FINANCE_CONTRACT_ID \
  --source merchant \
  -- issue_trade_credit \
  --merchant MERCHANT_ADDRESS \
  --financial_institution FINANCIAL_INSTITUTION_ADDRESS \
  --trustlink_id $TRUSTLINK_CONTRACT_ID \
  --amount 100000000
```

## Multi-Sig Attestations

For high-value trade transactions, you can use TrustLink's multi-sig attestation feature to require multiple regulatory authorities to approve the transaction:

```bash
# Propose a 2-of-3 multi-sig attestation for a large trade credit
soroban contract invoke \
  --id $TRUSTLINK_CONTRACT_ID \
  --source regulator_a \
  -- propose_attestation \
  --proposer REGULATOR_A_ADDRESS \
  --subject MERCHANT_ADDRESS \
  --claim_type MERCHANT_VERIFIED \
  --required_signers '[REGULATOR_A, REGULATOR_B, REGULATOR_C]' \
  --threshold 2
```

## Further Reading

- [Integration Guide](../../docs/integration-guide.md)
- [TrustLink README](../../README.md)
- [Multi-Sig Attestations](../../docs/video-tutorial-guide.md#7-multi-sig-attestations)
