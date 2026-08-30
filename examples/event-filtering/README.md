# TrustLink Event Filtering Examples

This directory contains examples demonstrating how to use TrustLink's event subscription features with topic filtering. Both TypeScript and Python examples are provided.

## Overview

TrustLink now supports unified event topic filtering across all SDKs and the indexer. Instead of subscribing to all events and filtering client-side, consumers can now specify which event topics they care about and receive only those events.

## Key Features

- **Topic Allowlists**: Subscribe to specific event topics instead of all events
- **Event Categories**: Pre-defined topic groupings for common use cases
- **Multiple Subscription Modes**: 
  - GraphQL subscriptions via the indexer for real-time updates
  - Direct ledger watching via Soroban RPC for comprehensive event coverage
- **Flexible Filtering**: Combine topic filters with entity filters (subject, issuer)

## Available Event Topics

See [docs/EVENT_TOPICS.md](../../docs/EVENT_TOPICS.md) for the complete canonical event topic reference.

### Common Topic Categories

- **ATTESTATION_LIFECYCLE**: Attestation creation, revocation, expiration, etc.
- **ISSUER_COMPLIANCE**: Issuer registration, tier updates, whitelist management
- **REQUEST_LIFECYCLE**: Attestation requests, fulfillment, rejection
- **MULTISIG**: Multi-sig proposals and signings
- **DISPUTE_AMENDMENT**: Dispute handling and metadata amendments
- **ADMIN_ACTIONS**: Admin initialization, transfers, and removals
- **COUNCIL_GOVERNANCE**: Council proposals and timelock events

## TypeScript Examples

### Setup

```bash
npm install @trustlink/contract
```

### Running Examples

```bash
# Example 1: GraphQL subscriptions with topic filtering
npx ts-node typescript-example.ts 1

# Example 2: Direct ledger watching with categories
npx ts-node typescript-example.ts 2

# Example 3: Multi-topic subscription with filtering
npx ts-node typescript-example.ts 3

# Example 4: Comprehensive multi-category monitoring
npx ts-node typescript-example.ts 4

# Example 5: Error handling and retry logic
npx ts-node typescript-example.ts 5
```

### Example: TypeScript GraphQL Subscription

```typescript
import { 
  subscribeToGraphQLEvents, 
  EventTopics, 
  GraphQLSubscriptionOptions 
} from "@trustlink/contract";

const options: GraphQLSubscriptionOptions = {
  graphqlUrl: "wss://indexer.trustlink.io/graphql",
  topics: [EventTopics.CREATED, EventTopics.REVOKED],
  subject: "GBBD5YHQVW53S3QQZVVTQ6S4RBFMU7GZGMG3MHEWJRUZRM5Y2I3R4D",
};

const subscription = await subscribeToGraphQLEvents(options, (event) => {
  console.log(`Event: ${event.topic} at ledger ${event.ledger}`);
});

// Later: unsubscribe
await subscription.unsubscribe();
```

### Example: TypeScript Direct Ledger Watching

```typescript
import { 
  subscribeToDirectLedgerEvents,
  EventCategories,
  LedgerEventWatchOptions 
} from "@trustlink/contract";

const options: LedgerEventWatchOptions = {
  rpcUrl: "https://soroban-testnet.stellar.org",
  contractId: "C...",
  networkPassphrase: "Test SDF Future Network ; October 2024",
  topics: EventCategories.ATTESTATION_LIFECYCLE,
  subject: "GBBD5...",
};

const subscription = await subscribeToDirectLedgerEvents(options, (event) => {
  console.log(`Event: ${event.topic}`);
});
```

## Python Examples

### Setup

```bash
pip install trustlink
```

### Running Examples

```bash
# Example 1: GraphQL subscriptions with topic filtering
python3 python-example.py 1

# Example 2: Direct ledger watching with categories
python3 python-example.py 2

# Example 3: Multi-topic subscription with filtering
python3 python-example.py 3

# Example 4: Comprehensive multi-category monitoring
python3 python-example.py 4

# Example 5: Error handling and retry logic
python3 python-example.py 5

# Example 6: Explore available topics and categories
python3 python-example.py 6
```

### Example: Python GraphQL Subscription

```python
import asyncio
from trustlink import (
    subscribe_to_graphql_events,
    EventTopics,
    GraphQLSubscriptionOptions,
)

async def main():
    options = GraphQLSubscriptionOptions(
        graphql_url="wss://indexer.trustlink.io/graphql",
        topics=[EventTopics.CREATED, EventTopics.REVOKED],
        subject="GBBD5YHQVW53S3QQZVVTQ6S4RBFMU7GZGMG3MHEWJRUZRM5Y2I3R4D",
    )

    async def on_event(event):
        print(f"Event: {event.topic} at ledger {event.ledger}")

    subscription = await subscribe_to_graphql_events(options, on_event)
    
    # Keep running until interrupted
    try:
        await asyncio.sleep(3600)
    finally:
        await subscription.unsubscribe()

asyncio.run(main())
```

### Example: Python Direct Ledger Watching

```python
import asyncio
from trustlink import (
    subscribe_to_direct_ledger_events,
    EventCategories,
    LedgerEventWatchOptions,
)

async def main():
    options = LedgerEventWatchOptions(
        rpc_url="https://soroban-testnet.stellar.org",
        contract_id="C...",
        network_passphrase="Test SDF Future Network ; October 2024",
        topics=EventCategories.ATTESTATION_LIFECYCLE,
        subject="GBBD5...",
    )

    async def on_event(event):
        print(f"Event: {event.topic}")

    subscription = await subscribe_to_direct_ledger_events(options, on_event)

asyncio.run(main())
```

## Environment Variables

Set these environment variables to customize the examples:

```bash
# Indexer GraphQL WebSocket URL
export INDEXER_WS_URL="wss://indexer.trustlink.io/graphql"

# TrustLink contract ID
export CONTRACT_ID="CDJVR36XC2HTUGDGSTHVUGMR3JTYHWQQFBOJGZAVDMJLQGZXVVGTJMZV"

# Optional: Soroban RPC URL (defaults to testnet)
export RPC_URL="https://soroban-testnet.stellar.org"

# Optional: Network passphrase (defaults to testnet)
export NETWORK_PASSPHRASE="Test SDF Future Network ; October 2024"
```

## Event Filtering Best Practices

1. **Always specify topic filters** when possible to reduce bandwidth and client-side processing
2. **Use event categories** for related workflows (e.g., all attestation lifecycle events together)
3. **Combine with entity filters** (subject, issuer) for fine-grained control
4. **Monitor indexed topics** for real-time updates via GraphQL subscriptions
5. **Use direct ledger watching** for comprehensive historical event coverage
6. **Handle duplicate events** — the same event may appear from both indexer and direct watching

## Subscription Modes Comparison

| Feature | GraphQL Subscriptions | Direct Ledger Watching |
|---------|----------------------|------------------------|
| Real-time updates | ✓ | ✓ (with polling) |
| Topic filtering | ✓ | ✓ |
| Entity filtering | Limited | ✓ |
| Historical events | Limited | ✓ |
| Bandwidth efficient | ✓ | ⚠️ (requires polling) |
| Indexer dependent | ✓ | ✗ |
| Comprehensive coverage | Limited | ✓ |

## Common Use Cases

### Monitor specific subject's attestations
```typescript
topics: [EventTopics.CREATED, EventTopics.REVOKED, EventTopics.EXPIRED],
subject: userAddress
```

### Monitor issuer compliance
```typescript
topics: EventCategories.ISSUER_COMPLIANCE,
issuer: issuerAddress
```

### Monitor request processing
```typescript
topics: EventCategories.REQUEST_LIFECYCLE
```

### Monitor all admin actions
```typescript
topics: EventCategories.ADMIN_ACTIONS
```

## Troubleshooting

### Not receiving events

1. Verify the indexer is running and events are being indexed
2. Check that the contract ID matches your deployment
3. Ensure topic names exactly match the canonical list (case-sensitive)
4. Verify network passphrase matches the RPC endpoint

### Missing events

- GraphQL subscriptions may not capture all events if the indexer is lagging
- Use direct ledger watching to ensure comprehensive event coverage
- Check the `INDEXED_PRIORITY` category to see which events are prioritized

### Connection issues

- Check network connectivity to the RPC and indexer endpoints
- Verify environment variables are set correctly
- Review logs for authentication or rate-limiting errors

## Related Documentation

- [EVENT_TOPICS.md](../../docs/EVENT_TOPICS.md) — Complete event topic reference
- [SDK Documentation](../../bindings/) — SDK reference and examples
- [Indexer Documentation](../../indexer/) — Indexer setup and configuration

## Support

For issues or questions:

1. Check the [EVENT_TOPICS.md](../../docs/EVENT_TOPICS.md) for complete topic documentation
2. Review existing examples for similar use cases
3. Check SDK documentation for API details
4. Open an issue on the project repository
