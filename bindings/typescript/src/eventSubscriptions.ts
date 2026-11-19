/**
 * Event subscription helpers for TrustLink SDK.
 *
 * Provides convenient methods to subscribe to contract events via GraphQL subscriptions
 * or direct ledger event watching with optional topic filtering.
 */

import { Server as SorobanRpc } from "@stellar/stellar-sdk";
import type { EventSubscriptionOptions, EventTopic } from "./events";
import { validateEventTopics } from "./events";

/**
 * Represents a raw event from the ledger.
 */
export interface RawContractEvent {
  id: string;
  type: string;
  ledger: number;
  txHash: string;
  topics: string[];
  data: unknown;
  timestamp: Date;
}

/**
 * Represents a mapped/normalized contract event.
 */
export interface ContractEvent {
  topic: EventTopic;
  ledger: number;
  txHash: string;
  data: Record<string, unknown>;
  timestamp: Date;
  raw: RawContractEvent;
}

/**
 * Callback type for event subscriptions.
 */
export type EventCallback = (event: ContractEvent) => void | Promise<void>;

/**
 * A subscription handle that can be used to unsubscribe from events.
 */
export interface EventSubscription {
  /** Unsubscribe from the event stream */
  unsubscribe(): Promise<void>;

  /** Check if the subscription is still active */
  isActive(): boolean;
}

/**
 * GraphQL subscription event - represents streaming events from the indexer.
 */
interface GraphQLSubscriptionMessage {
  id: string;
  type: "subscribe" | "next" | "complete" | "error";
  payload?: {
    data?: Record<string, unknown>;
    errors?: Array<{ message: string }>;
  };
}

/**
 * Options for GraphQL-based event subscriptions (via indexer).
 */
export interface GraphQLSubscriptionOptions extends EventSubscriptionOptions {
  /** GraphQL endpoint WebSocket URL */
  graphqlUrl?: string;

  /** Custom headers to include in the WebSocket connection */
  headers?: Record<string, string>;

  /** Reconnection config */
  reconnectAttempts?: number;
  reconnectDelayMs?: number;
}

/**
 * Options for direct ledger event watching.
 */
export interface LedgerEventWatchOptions extends EventSubscriptionOptions {
  /** Soroban RPC endpoint (required) */
  rpcUrl: string;

  /** Contract ID to watch */
  contractId: string;

  /** Network passphrase */
  networkPassphrase: string;

  /** Starting ledger (optional - defaults to current ledger) */
  startLedger?: number;
}

/**
 * Subscribes to contract events via direct ledger watching with optional topic filtering.
 *
 * This method polls the Soroban RPC for new ledger events, allowing fine-grained
 * filtering by topics without depending on the indexer.
 *
 * @param options - Configuration for ledger event watching
 * @param callback - Callback invoked for each matching event
 * @returns EventSubscription handle for unsubscribing
 *
 * @example
 * ```ts
 * import { subscribeToDirect LedgerEvents, EventTopics } from '@trustlink/contract';
 *
 * const unsub = await subscribeToDirectLedgerEvents(
 *   {
 *     rpcUrl: 'https://soroban-testnet.stellar.org',
 *     contractId: 'C...',
 *     networkPassphrase: 'Test SDF Future Network ; October 2024',
 *     topics: [EventTopics.CREATED, EventTopics.REVOKED],
 *     startLedger: 1000000,
 *   },
 *   (event) => console.log('Event:', event)
 * );
 * ```
 */
export async function subscribeToDirectLedgerEvents(
  options: LedgerEventWatchOptions,
  callback: EventCallback
): Promise<EventSubscription> {
  const {
    rpcUrl,
    contractId,
    networkPassphrase,
    topics: rawTopics,
    subject,
    issuer,
    pollingIntervalMs = 5000,
    pageSize = 100,
    startLedger,
  } = options;

  // Validate topics if provided
  const topics = rawTopics ? validateEventTopics(rawTopics) : null;

  const rpc = new SorobanRpc(rpcUrl, { allowHttp: true });

  let isActive = true;
  let cursor = startLedger;

  // Get initial ledger if not specified
  if (!cursor) {
    const latestLedger = await rpc.getLatestLedger();
    cursor = latestLedger.sequence;
  }

  // Start polling loop
  const pollInterval = setInterval(async () => {
    if (!isActive) return;

    try {
      const latestLedger = await rpc.getLatestLedger();
      const latest = latestLedger.sequence;

      if (cursor <= latest) {
        // Fetch events in the range
        const events = await rpc.getEvents({
          startLedger: cursor,
          filters: [
            {
              type: "contract",
              contractIds: [contractId],
            },
          ],
          limit: pageSize,
        });

        for (const event of events.events) {
          // Filter by topic if specified
          if (topics && topics.length > 0) {
            const eventTopic = event.topics?.[0]?.toString();
            if (!eventTopic || !topics.includes(eventTopic as EventTopic)) {
              continue;
            }
          }

          // Filter by subject if specified
          if (subject && event.topics?.[1] !== subject) {
            continue;
          }

          // Filter by issuer if specified
          if (issuer && event.topics?.[1] !== issuer) {
            continue;
          }

          // Map and invoke callback
          const mappedEvent = mapRawEvent(event);
          if (mappedEvent) {
            await callback(mappedEvent);
          }
        }

        cursor = latest + 1;
      }
    } catch (error) {
      console.error("Error polling ledger events:", error);
    }
  }, pollingIntervalMs);

  return {
    async unsubscribe(): Promise<void> {
      isActive = false;
      clearInterval(pollInterval);
    },
    isActive(): boolean {
      return isActive;
    },
  };
}

/**
 * Subscribes to contract events via GraphQL subscriptions from the indexer.
 *
 * This method uses the indexer's GraphQL subscriptions for real-time event streaming.
 * Note: Event topic filtering via the indexer is implementation-dependent and may
 * require server-side support.
 *
 * @param options - Configuration for GraphQL subscriptions
 * @param callback - Callback invoked for each matching event
 * @returns EventSubscription handle for unsubscribing
 *
 * @example
 * ```ts
 * import { subscribeToGraphQLEvents, EventTopics, EventCategories } from '@trustlink/contract';
 *
 * const unsub = await subscribeToGraphQLEvents(
 *   {
 *     graphqlUrl: 'wss://indexer.trustlink.io/graphql',
 *     topics: EventCategories.ATTESTATION_LIFECYCLE,
 *     subject: 'GBBD...',
 *   },
 *   (event) => console.log('Event:', event)
 * );
 * ```
 */
export async function subscribeToGraphQLEvents(
  options: GraphQLSubscriptionOptions,
  callback: EventCallback
): Promise<EventSubscription> {
  const {
    graphqlUrl = process.env.VITE_INDEXER_WS_URL || "ws://localhost:4000/graphql",
    topics: rawTopics,
    subject,
    issuer,
    headers = {},
    reconnectAttempts = 5,
    reconnectDelayMs = 1000,
  } = options;

  // Validate topics if provided
  const topics = rawTopics ? validateEventTopics(rawTopics) : null;

  let isActive = true;
  let ws: WebSocket | null = null;
  let reconnectCount = 0;

  const connect = (): Promise<void> => {
    return new Promise((resolve, reject) => {
      try {
        ws = new WebSocket(graphqlUrl, "graphql-transport-ws");

        ws.onopen = () => {
          reconnectCount = 0;
          ws!.send(JSON.stringify({ type: "connection_init", payload: {} }));
          resolve();
        };

        ws.onmessage = async (event: MessageEvent) => {
          if (!isActive) return;

          let message: GraphQLSubscriptionMessage;
          try {
            message = JSON.parse(event.data as string);
          } catch {
            return;
          }

          if (message.type === "connection_ack") {
            // Subscribe to relevant subscriptions based on topic filters
            subscribeToRelevantTopics();
          } else if (message.type === "next" && message.payload?.data) {
            const payload = message.payload.data;

            // Handle attestation events
            if ("onAttestationCreated" in payload) {
              const data = payload.onAttestationCreated as Record<string, unknown>;
              if (
                !topics ||
                topics.length === 0 ||
                topics.includes("created" as EventTopic)
              ) {
                if (!subject || data.subject === subject) {
                  const mappedEvent: ContractEvent = {
                    topic: "created" as EventTopic,
                    ledger: 0,
                    txHash: "",
                    data,
                    timestamp: new Date(),
                    raw: {
                      id: data.id as string,
                      type: "contract",
                      ledger: 0,
                      txHash: "",
                      topics: ["created"],
                      data,
                      timestamp: new Date(),
                    },
                  };
                  await callback(mappedEvent);
                }
              }
            }

            if ("onAttestationRevoked" in payload) {
              const data = payload.onAttestationRevoked as Record<string, unknown>;
              if (
                !topics ||
                topics.length === 0 ||
                topics.includes("revoked" as EventTopic)
              ) {
                if (!issuer || data.issuer === issuer) {
                  const mappedEvent: ContractEvent = {
                    topic: "revoked" as EventTopic,
                    ledger: 0,
                    txHash: "",
                    data,
                    timestamp: new Date(),
                    raw: {
                      id: data.id as string,
                      type: "contract",
                      ledger: 0,
                      txHash: "",
                      topics: ["revoked"],
                      data,
                      timestamp: new Date(),
                    },
                  };
                  await callback(mappedEvent);
                }
              }
            }

            if ("onIssuerRegistered" in payload) {
              const data = payload.onIssuerRegistered as Record<string, unknown>;
              if (
                !topics ||
                topics.length === 0 ||
                topics.includes("iss_reg" as EventTopic)
              ) {
                const mappedEvent: ContractEvent = {
                  topic: "iss_reg" as EventTopic,
                  ledger: 0,
                  txHash: "",
                  data,
                  timestamp: new Date(),
                  raw: {
                    id: JSON.stringify(data),
                    type: "contract",
                    ledger: 0,
                    txHash: "",
                    topics: ["iss_reg"],
                    data,
                    timestamp: new Date(),
                  },
                };
                await callback(mappedEvent);
              }
            }
          }
        };

        ws.onerror = (error: Event) => {
          if (isActive && reconnectCount < reconnectAttempts) {
            reconnectCount++;
            setTimeout(() => {
              connect().catch(console.error);
            }, reconnectDelayMs * Math.pow(2, reconnectCount - 1));
          } else {
            reject(new Error(`WebSocket error: ${error}`));
          }
        };

        ws.onclose = () => {
          if (isActive && reconnectCount < reconnectAttempts) {
            reconnectCount++;
            setTimeout(() => {
              connect().catch(console.error);
            }, reconnectDelayMs * Math.pow(2, reconnectCount - 1));
          }
        };
      } catch (error) {
        reject(error);
      }
    });
  };

  const subscribeToRelevantTopics = () => {
    if (!ws || ws.readyState !== WebSocket.OPEN) return;

    // Subscribe to attestation created if relevant
    if (!topics || topics.length === 0 || topics.includes("created" as EventTopic)) {
      const subId = "sub_created_" + Date.now();
      const query = subject
        ? `subscription { onAttestationCreated(subject: "${subject}") { id subject issuer claimType } }`
        : `subscription { onAttestationCreated { id subject issuer claimType } }`;

      ws.send(
        JSON.stringify({
          id: subId,
          type: "subscribe",
          payload: { query },
        })
      );
    }

    // Subscribe to attestation revoked if relevant
    if (!topics || topics.length === 0 || topics.includes("revoked" as EventTopic)) {
      const subId = "sub_revoked_" + Date.now();
      const query = issuer
        ? `subscription { onAttestationRevoked(issuer: "${issuer}") { id issuer revokedAt } }`
        : `subscription { onAttestationRevoked { id issuer revokedAt } }`;

      ws.send(
        JSON.stringify({
          id: subId,
          type: "subscribe",
          payload: { query },
        })
      );
    }

    // Subscribe to issuer registered if relevant
    if (!topics || topics.length === 0 || topics.includes("iss_reg" as EventTopic)) {
      const subId = "sub_iss_reg_" + Date.now();
      ws.send(
        JSON.stringify({
          id: subId,
          type: "subscribe",
          payload: { query: "subscription { onIssuerRegistered { issuer registeredAt } }" },
        })
      );
    }
  };

  // Establish initial connection
  await connect();

  return {
    async unsubscribe(): Promise<void> {
      isActive = false;
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.close();
      }
    },
    isActive(): boolean {
      return isActive;
    },
  };
}

/**
 * Maps a raw Soroban event to a normalized ContractEvent.
 * This is internal and used by subscribeToDirectLedgerEvents.
 */
function mapRawEvent(rawEvent: any): ContractEvent | null {
  try {
    const topic = rawEvent.topics?.[0];
    if (!topic) return null;

    return {
      topic: topic as EventTopic,
      ledger: rawEvent.ledger || 0,
      txHash: rawEvent.txHash || "",
      data: rawEvent.value || {},
      timestamp: new Date(),
      raw: {
        id: rawEvent.id || "",
        type: rawEvent.type || "contract",
        ledger: rawEvent.ledger || 0,
        txHash: rawEvent.txHash || "",
        topics: rawEvent.topics || [],
        data: rawEvent.value || {},
        timestamp: new Date(),
      },
    };
  } catch {
    return null;
  }
}
