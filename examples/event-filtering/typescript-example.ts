/**
 * TrustLink Event Filtering Example (TypeScript)
 *
 * This example demonstrates how to subscribe to contract events with topic filtering
 * using the TrustLink SDK. It shows both GraphQL subscriptions and direct ledger watching.
 */

import {
  TrustLinkClient,
  EventTopics,
  EventCategories,
  subscribeToGraphQLEvents,
  subscribeToDirectLedgerEvents,
  GraphQLSubscriptionOptions,
  LedgerEventWatchOptions,
} from "@trustlink/contract";

// ─── Example 1: Subscribe to specific attestation events via GraphQL ────────

async function example1_GraphQLTopicFiltering() {
  console.log("=== Example 1: GraphQL Subscriptions with Topic Filtering ===\n");

  const options: GraphQLSubscriptionOptions = {
    graphqlUrl: process.env.INDEXER_WS_URL || "wss://indexer.trustlink.io/graphql",
    // Only subscribe to attestation creation and revocation events
    topics: [EventTopics.CREATED, EventTopics.REVOKED],
    // Optional: filter by subject address
    subject: "GBBD5YHQVW53S3QQZVVTQ6S4RBFMU7GZGMG3MHEWJRUZRM5Y2I3R4D",
  };

  const subscription = await subscribeToGraphQLEvents(options, (event) => {
    console.log(`Event received: ${event.topic}`);
    console.log(`  Ledger: ${event.ledger}`);
    console.log(`  Data: ${JSON.stringify(event.data)}`);
    console.log();
  });

  // Unsubscribe after 30 seconds
  setTimeout(() => {
    subscription.unsubscribe();
    console.log("Unsubscribed from GraphQL events");
  }, 30000);
}

// ─── Example 2: Subscribe to category of events via direct ledger watching ──

async function example2_DirectLedgerWatchingWithCategories() {
  console.log("=== Example 2: Direct Ledger Watching with Event Categories ===\n");

  // Subscribe to all attestation lifecycle events
  const options: LedgerEventWatchOptions = {
    rpcUrl: "https://soroban-testnet.stellar.org",
    contractId: process.env.CONTRACT_ID || "C...",
    networkPassphrase: "Test SDF Future Network ; October 2024",
    // Use a category from EventCategories for related events
    topics: EventCategories.ATTESTATION_LIFECYCLE,
    pollingIntervalMs: 5000,
    pageSize: 100,
  };

  const subscription = await subscribeToDirectLedgerEvents(options, (event) => {
    console.log(`Event: ${event.topic} at ledger ${event.ledger}`);
    console.log(`  Timestamp: ${event.timestamp.toISOString()}`);
  });

  // Keep subscription active for demonstration
  console.log("Listening to attestation lifecycle events...");
  console.log("Press Ctrl+C to stop\n");
}

// ─── Example 3: Multi-topic subscription with filtering ──────────────────

async function example3_MultiTopicWithFiltering() {
  console.log("=== Example 3: Multi-Topic Subscription with Filtering ===\n");

  // Subscribe to compliance-related events
  const options: GraphQLSubscriptionOptions = {
    graphqlUrl: process.env.INDEXER_WS_URL,
    // Subscribe to issuer compliance events
    topics: EventCategories.ISSUER_COMPLIANCE,
    // Filter by specific issuer
    issuer: "GBRPYHIL2CI3WHPSKYXXRX7XQJ5RP4A5ECLYWWQSBHXVLZACVXULO5Z",
  };

  const subscription = await subscribeToGraphQLEvents(options, (event) => {
    console.log(`Issuer compliance event: ${event.topic}`);
    console.log(JSON.stringify(event.data, null, 2));
    console.log();
  });
}

// ─── Example 4: Comprehensive event monitoring ────────────────────────────

async function example4_ComprehensiveMonitoring() {
  console.log("=== Example 4: Comprehensive Event Monitoring ===\n");

  // Create multiple subscriptions for different event categories

  // 1. Monitor all attestation lifecycle changes
  const attestationSub = await subscribeToDirectLedgerEvents(
    {
      rpcUrl: "https://soroban-testnet.stellar.org",
      contractId: process.env.CONTRACT_ID || "C...",
      networkPassphrase: "Test SDF Future Network ; October 2024",
      topics: EventCategories.ATTESTATION_LIFECYCLE,
      subject: "GBBD5YHQVW53S3QQZVVTQ6S4RBFMU7GZGMG3MHEWJRUZRM5Y2I3R4D",
    },
    (event) => {
      console.log(`[ATTESTATION] ${event.topic}`);
    }
  );

  // 2. Monitor issuer compliance events
  const complianceSub = await subscribeToDirectLedgerEvents(
    {
      rpcUrl: "https://soroban-testnet.stellar.org",
      contractId: process.env.CONTRACT_ID || "C...",
      networkPassphrase: "Test SDF Future Network ; October 2024",
      topics: EventCategories.ISSUER_COMPLIANCE,
      issuer: "GBRPYHIL2CI3WHPSKYXXRX7XQJ5RP4A5ECLYWWQSBHXVLZACVXULO5Z",
    },
    (event) => {
      console.log(`[COMPLIANCE] ${event.topic}`);
    }
  );

  // 3. Monitor multi-sig operations
  const multisigSub = await subscribeToDirectLedgerEvents(
    {
      rpcUrl: "https://soroban-testnet.stellar.org",
      contractId: process.env.CONTRACT_ID || "C...",
      networkPassphrase: "Test SDF Future Network ; October 2024",
      topics: EventCategories.MULTISIG,
    },
    (event) => {
      console.log(`[MULTISIG] ${event.topic}`);
    }
  );

  console.log("Monitoring multiple event categories...");
  console.log("Press Ctrl+C to stop\n");

  // Cleanup on exit
  process.on("SIGINT", async () => {
    console.log("\nCleaning up subscriptions...");
    await attestationSub.unsubscribe();
    await complianceSub.unsubscribe();
    await multisigSub.unsubscribe();
    process.exit(0);
  });
}

// ─── Example 5: Using the React Hook with Topic Filtering ──────────────────

import { useEffect, useRef } from "react";

/**
 * Enhanced React hook for attestation subscriptions with topic filtering.
 * Demonstrates how to use the SDK subscription helpers in a React component.
 */
export function useEnhancedAttestationSubscription(
  address: string | null,
  topics?: string[],
  onEvent?: (event: any) => void
) {
  const onEventRef = useRef(onEvent);
  onEventRef.current = onEvent;

  useEffect(() => {
    if (!address) return;

    let subscription: any;

    (async () => {
      subscription = await subscribeToGraphQLEvents(
        {
          graphqlUrl: (import.meta as { env: Record<string, string> }).env
            .VITE_INDEXER_WS_URL,
          topics: topics || EventCategories.ATTESTATION_LIFECYCLE,
          subject: address,
        },
        (event) => {
          onEventRef.current?.(event);
        }
      );
    })();

    return () => {
      subscription?.unsubscribe();
    };
  }, [address, topics]);
}

// ─── Example 6: Error Handling and Retry Logic ─────────────────────────────

async function example6_ErrorHandlingAndRetry() {
  console.log("=== Example 6: Error Handling and Retry Logic ===\n");

  const options: LedgerEventWatchOptions = {
    rpcUrl: "https://soroban-testnet.stellar.org",
    contractId: process.env.CONTRACT_ID || "C...",
    networkPassphrase: "Test SDF Future Network ; October 2024",
    topics: [EventTopics.CREATED, EventTopics.REVOKED],
    pollingIntervalMs: 5000,
  };

  let retryCount = 0;
  const maxRetries = 3;

  const subscription = await subscribeToDirectLedgerEvents(options, (event) => {
    try {
      console.log(`Processing event: ${event.topic}`);
      // Your event processing logic here
      retryCount = 0; // Reset on successful processing
    } catch (error) {
      retryCount++;
      console.error(`Error processing event (attempt ${retryCount}):`, error);

      if (retryCount >= maxRetries) {
        console.error("Max retries exceeded, unsubscribing");
        subscription.unsubscribe();
      }
    }
  });
}

// ─── Main: Run examples ────────────────────────────────────────────────────

async function main() {
  console.log("TrustLink Event Filtering Examples\n");
  console.log("Select an example to run:\n");
  console.log("1. GraphQL Subscriptions with Topic Filtering");
  console.log("2. Direct Ledger Watching with Event Categories");
  console.log("3. Multi-Topic Subscription with Filtering");
  console.log("4. Comprehensive Event Monitoring");
  console.log("6. Error Handling and Retry Logic\n");

  const example = process.argv[2] || "1";

  switch (example) {
    case "1":
      await example1_GraphQLTopicFiltering();
      break;
    case "2":
      await example2_DirectLedgerWatchingWithCategories();
      break;
    case "3":
      await example3_MultiTopicWithFiltering();
      break;
    case "4":
      await example4_ComprehensiveMonitoring();
      break;
    case "6":
      await example6_ErrorHandlingAndRetry();
      break;
    default:
      console.log(`Unknown example: ${example}`);
  }
}

main().catch(console.error);
