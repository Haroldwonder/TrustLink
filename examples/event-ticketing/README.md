# Event Ticketing Example

This example demonstrates how to integrate TrustLink into an event ticketing system. Event organizers can issue tickets only after their identity has been verified through TrustLink attestations.

## Use Case

Event ticketing platforms need to verify that ticket issuers (event organizers) are legitimate and properly vetted. This example shows how to gate ticket issuance behind a TrustLink attestation check.

## Features

- Gate ticket issuance behind attestation verification
- Require `EVENT_ORGANIZER` claim from trusted issuers
- Prevent unauthorized ticket creation

## Building

```bash
cargo build --target wasm32-unknown-unknown --release
```

## Testing

```bash
cargo test
```

## Integration

This contract expects to be called with the address of a deployed TrustLink instance. The calling contract verifies that the organizer address holds a valid `EVENT_ORGANIZER` attestation before allowing ticket issuance.

### API

```rust
pub fn issue_ticket(
    env: Env,
    attendee: Address,
    trustlink_id: Address,
    event_id: String,
) -> Result<bool, String>
```

**Parameters:**
- `attendee`: The address issuing the ticket (must have `EVENT_ORGANIZER` attestation)
- `trustlink_id`: The contract ID of the deployed TrustLink instance
- `event_id`: Unique identifier for the event

**Returns:** `true` if ticket issuance is successful

## Example Usage

```bash
# Deploy TrustLink first (see main README)
CONTRACT_ID=C...

# Register an event organizer as an issuer
soroban contract invoke \
  --id $CONTRACT_ID \
  --source admin \
  -- register_issuer \
  --issuer ORGANIZER_PUBLIC_KEY

# Issuer creates their own attestation (self-signed or via admin)
soroban contract invoke \
  --id $CONTRACT_ID \
  --source admin \
  -- create_attestation \
  --issuer ORGANIZER_PUBLIC_KEY \
  --subject ORGANIZER_PUBLIC_KEY \
  --claim_type EVENT_ORGANIZER \
  --expiration null

# Now the organizer can issue tickets
soroban contract invoke \
  --id EVENT_TICKETING_CONTRACT_ID \
  --source organizer \
  -- issue_ticket \
  --attendee ATTENDEE_ADDRESS \
  --trustlink_id $CONTRACT_ID \
  --event_id "summer-festival-2024"
```

## Further Reading

- [Integration Guide](../../docs/integration-guide.md)
- [TrustLink README](../../README.md)
