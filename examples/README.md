# Event ticketing

This example uses a `TICKET_VALID` attestation as an event ticket. On purchase,
an authorized issuer creates the attestation for the purchaser's Stellar address.
At venue check-in, the venue verifies the claim for that address and immediately
revokes the attestation, making the ticket single-use.

Attestations have no transfer entry point and their `subject` is immutable. A
different address therefore cannot use a ticket issued to the purchaser, avoiding
the bearer-token behavior that enables ticket resale and scalping.

Run the demonstration and its assertions with:

```sh
cargo test --example event_ticketing
```
