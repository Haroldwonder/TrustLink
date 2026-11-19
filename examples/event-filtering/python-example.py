#!/usr/bin/env python3
"""
TrustLink Event Filtering Example (Python)

This example demonstrates how to subscribe to contract events with topic filtering
using the TrustLink Python SDK. It shows both GraphQL subscriptions and direct ledger watching.
"""

import asyncio
import os
from trustlink import (
    EventTopics,
    EventCategories,
    subscribe_to_direct_ledger_events,
    subscribe_to_graphql_events,
    LedgerEventWatchOptions,
    GraphQLSubscriptionOptions,
)


# ─── Example 1: Subscribe to specific attestation events via GraphQL ────────


async def example1_graphql_topic_filtering():
    """Subscribe to specific attestation events via GraphQL with topic filtering."""
    print("=== Example 1: GraphQL Subscriptions with Topic Filtering ===\n")

    options = GraphQLSubscriptionOptions(
        graphql_url=os.getenv("INDEXER_WS_URL", "wss://indexer.trustlink.io/graphql"),
        # Only subscribe to attestation creation and revocation events
        topics=[EventTopics.CREATED, EventTopics.REVOKED],
        # Optional: filter by subject address
        subject="GBBD5YHQVW53S3QQZVVTQ6S4RBFMU7GZGMG3MHEWJRUZRM5Y2I3R4D",
    )

    async def on_event(event):
        print(f"Event received: {event.topic}")
        print(f"  Ledger: {event.ledger}")
        print(f"  Data: {event.data}")
        print()

    subscription = await subscribe_to_graphql_events(options, on_event)

    # Unsubscribe after 30 seconds
    await asyncio.sleep(30)
    await subscription.unsubscribe()
    print("Unsubscribed from GraphQL events")


# ─── Example 2: Subscribe to category of events via direct ledger watching ──


async def example2_direct_ledger_watching_with_categories():
    """Subscribe to all attestation lifecycle events via direct ledger watching."""
    print("=== Example 2: Direct Ledger Watching with Event Categories ===\n")

    options = LedgerEventWatchOptions(
        rpc_url="https://soroban-testnet.stellar.org",
        contract_id=os.getenv("CONTRACT_ID", "C..."),
        network_passphrase="Test SDF Future Network ; October 2024",
        # Use a category from EventCategories for related events
        topics=EventCategories.ATTESTATION_LIFECYCLE,
        polling_interval_ms=5000,
        page_size=100,
    )

    async def on_event(event):
        print(f"Event: {event.topic} at ledger {event.ledger}")
        print(f"  Timestamp: {event.timestamp.isoformat()}")
        print()

    subscription = await subscribe_to_direct_ledger_events(options, on_event)

    # Keep subscription active for demonstration
    print("Listening to attestation lifecycle events...")
    print("Press Ctrl+C to stop\n")

    try:
        await asyncio.sleep(3600)  # Run for 1 hour
    except KeyboardInterrupt:
        await subscription.unsubscribe()
        print("\nUnsubscribed")


# ─── Example 3: Multi-topic subscription with filtering ──────────────────


async def example3_multi_topic_with_filtering():
    """Subscribe to issuer compliance events with topic filtering."""
    print("=== Example 3: Multi-Topic Subscription with Filtering ===\n")

    options = GraphQLSubscriptionOptions(
        graphql_url=os.getenv("INDEXER_WS_URL"),
        # Subscribe to issuer compliance events
        topics=EventCategories.ISSUER_COMPLIANCE,
        # Filter by specific issuer
        issuer="GBRPYHIL2CI3WHPSKYXXRX7XQJ5RP4A5ECLYWWQSBHXVLZACVXULO5Z",
    )

    async def on_event(event):
        print(f"Issuer compliance event: {event.topic}")
        print(f"Data: {event.data}\n")

    subscription = await subscribe_to_graphql_events(options, on_event)

    await asyncio.sleep(60)
    await subscription.unsubscribe()


# ─── Example 4: Comprehensive event monitoring ────────────────────────────


async def example4_comprehensive_monitoring():
    """Monitor multiple event categories simultaneously."""
    print("=== Example 4: Comprehensive Event Monitoring ===\n")

    # 1. Monitor all attestation lifecycle changes
    attestation_sub = await subscribe_to_direct_ledger_events(
        LedgerEventWatchOptions(
            rpc_url="https://soroban-testnet.stellar.org",
            contract_id=os.getenv("CONTRACT_ID", "C..."),
            network_passphrase="Test SDF Future Network ; October 2024",
            topics=EventCategories.ATTESTATION_LIFECYCLE,
            subject="GBBD5YHQVW53S3QQZVVTQ6S4RBFMU7GZGMG3MHEWJRUZRM5Y2I3R4D",
        ),
        lambda event: print(f"[ATTESTATION] {event.topic}"),
    )

    # 2. Monitor issuer compliance events
    compliance_sub = await subscribe_to_direct_ledger_events(
        LedgerEventWatchOptions(
            rpc_url="https://soroban-testnet.stellar.org",
            contract_id=os.getenv("CONTRACT_ID", "C..."),
            network_passphrase="Test SDF Future Network ; October 2024",
            topics=EventCategories.ISSUER_COMPLIANCE,
            issuer="GBRPYHIL2CI3WHPSKYXXRX7XQJ5RP4A5ECLYWWQSBHXVLZACVXULO5Z",
        ),
        lambda event: print(f"[COMPLIANCE] {event.topic}"),
    )

    # 3. Monitor multi-sig operations
    multisig_sub = await subscribe_to_direct_ledger_events(
        LedgerEventWatchOptions(
            rpc_url="https://soroban-testnet.stellar.org",
            contract_id=os.getenv("CONTRACT_ID", "C..."),
            network_passphrase="Test SDF Future Network ; October 2024",
            topics=EventCategories.MULTISIG,
        ),
        lambda event: print(f"[MULTISIG] {event.topic}"),
    )

    print("Monitoring multiple event categories...")
    print("Press Ctrl+C to stop\n")

    try:
        await asyncio.sleep(3600)
    except KeyboardInterrupt:
        print("\nCleaning up subscriptions...")
        await attestation_sub.unsubscribe()
        await compliance_sub.unsubscribe()
        await multisig_sub.unsubscribe()


# ─── Example 5: Error Handling and Retry Logic ─────────────────────────────


async def example5_error_handling_and_retry():
    """Demonstrate error handling and retry logic in event processing."""
    print("=== Example 5: Error Handling and Retry Logic ===\n")

    options = LedgerEventWatchOptions(
        rpc_url="https://soroban-testnet.stellar.org",
        contract_id=os.getenv("CONTRACT_ID", "C..."),
        network_passphrase="Test SDF Future Network ; October 2024",
        topics=[EventTopics.CREATED, EventTopics.REVOKED],
        polling_interval_ms=5000,
    )

    retry_count = 0
    max_retries = 3

    async def on_event_with_retry(event):
        nonlocal retry_count
        try:
            print(f"Processing event: {event.topic}")
            # Your event processing logic here
            retry_count = 0  # Reset on successful processing
        except Exception as error:
            retry_count += 1
            print(f"Error processing event (attempt {retry_count}): {error}")

            if retry_count >= max_retries:
                print("Max retries exceeded, unsubscribing")
                # In a real app, you'd unsubscribe here
                raise

    subscription = await subscribe_to_direct_ledger_events(options, on_event_with_retry)

    await asyncio.sleep(60)
    await subscription.unsubscribe()


# ─── Example 6: Using EventTopics and EventCategories ─────────────────────


async def example6_topic_exploration():
    """Demonstrate exploring available topics and categories."""
    print("=== Example 6: Topic Exploration ===\n")

    # Print all available topics
    print("All available topics:")
    all_topics = EventTopics.all_topics()
    for i, topic in enumerate(all_topics, 1):
        print(f"  {i:2d}. {topic}")

    print("\n" + "=" * 50)
    print("\nEvent categories:")

    print("\nATTESTATION_LIFECYCLE:")
    for topic in EventCategories.ATTESTATION_LIFECYCLE:
        print(f"  - {topic}")

    print("\nISSUER_COMPLIANCE:")
    for topic in EventCategories.ISSUER_COMPLIANCE:
        print(f"  - {topic}")

    print("\nREQUEST_LIFECYCLE:")
    for topic in EventCategories.REQUEST_LIFECYCLE:
        print(f"  - {topic}")

    print("\nMULTISIG:")
    for topic in EventCategories.MULTISIG:
        print(f"  - {topic}")

    print("\nDISPUTE_AMENDMENT:")
    for topic in EventCategories.DISPUTE_AMENDMENT:
        print(f"  - {topic}")

    print("\nADMIN_ACTIONS:")
    for topic in EventCategories.ADMIN_ACTIONS:
        print(f"  - {topic}")

    print("\nCOUNCIL_GOVERNANCE:")
    for topic in EventCategories.COUNCIL_GOVERNANCE:
        print(f"  - {topic}")


# ─── Main: Run examples ────────────────────────────────────────────────────


async def main():
    """Main entry point for examples."""
    print("TrustLink Event Filtering Examples\n")
    print("Select an example to run:\n")
    print("1. GraphQL Subscriptions with Topic Filtering")
    print("2. Direct Ledger Watching with Event Categories")
    print("3. Multi-Topic Subscription with Filtering")
    print("4. Comprehensive Event Monitoring")
    print("5. Error Handling and Retry Logic")
    print("6. Topic Exploration\n")

    import sys

    example = sys.argv[1] if len(sys.argv) > 1 else "1"

    try:
        if example == "1":
            await example1_graphql_topic_filtering()
        elif example == "2":
            await example2_direct_ledger_watching_with_categories()
        elif example == "3":
            await example3_multi_topic_with_filtering()
        elif example == "4":
            await example4_comprehensive_monitoring()
        elif example == "5":
            await example5_error_handling_and_retry()
        elif example == "6":
            await example6_topic_exploration()
        else:
            print(f"Unknown example: {example}")
    except KeyboardInterrupt:
        print("\n\nInterrupted by user")


if __name__ == "__main__":
    asyncio.run(main())
