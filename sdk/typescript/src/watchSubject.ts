/**
 * Subscription helper for monitoring a subject's attestation changes.
 *
 * Implements real-time GraphQL WebSocket subscriptions with automatic
 * reconnection and graceful fallback to RPC polling.
 */

import { validateAddress } from "./validation.js";
import type { TrustLinkClient } from "./client.js";
import type { Attestation } from "./types.js";

// ── Types ───────────────────────────────────────────────────────────────────

export type SubjectEventType = "created" | "revoked" | "renewed";

export interface SubjectAttestationEvent {
  type: SubjectEventType;
  subject: string;
  attestationId?: string;
  issuer?: string;
  claimType?: string;
  timestamp?: number;
  raw?: any;
}

export type SubjectWatchCallback = (event: SubjectAttestationEvent) => void | Promise<void>;

export interface WatchSubjectOptions {
  /**
   * Indexer GraphQL WebSocket URL (e.g. 'ws://localhost:4000/graphql' or 'wss://...').
   * If not provided or if WebSocket connections fail, falls back to polling RPC events.
   */
  graphqlUrl?: string;

  /**
   * Polling interval in milliseconds when using or falling back to RPC polling.
   * Default: 5000ms.
   */
  pollingIntervalMs?: number;

  /**
   * Maximum number of reconnect attempts before falling back to RPC polling.
   * Default: 5.
   */
  reconnectAttempts?: number;

  /**
   * Base delay between reconnect attempts in milliseconds (uses exponential backoff).
   * Default: 1000ms.
   */
  reconnectDelayMs?: number;

  /**
   * Optional custom WebSocket factory / mock transport.
   * If omitted, uses globalThis.WebSocket if available.
   */
  webSocketFactory?: (url: string, protocols?: string | string[]) => any;

  /**
   * Optional event types filter. Defaults to all types: ['created', 'revoked', 'renewed'].
   */
  eventTypes?: SubjectEventType[];
}

export type UnsubscribeFunction = () => void;

// ── Internal Helper Classes ─────────────────────────────────────────────────

interface WebSocketLike {
  send(data: string): void;
  close(code?: number, reason?: string): void;
  onopen?: ((event: any) => void) | null;
  onmessage?: ((event: { data: any }) => void) | null;
  onerror?: ((event: any) => void) | null;
  onclose?: ((event: { code: number; reason?: string }) => void) | null;
  readyState?: number;
}

/**
 * Subscribes to attestation changes for a subject address.
 *
 * Tries the indexer GraphQL WebSocket subscription first. If unavailable,
 * or upon exhausting reconnect attempts, gracefully falls back to polling
 * the contract over Soroban RPC.
 *
 * @param client - TrustLinkClient instance
 * @param subject - Stellar account address of the subject (G...)
 * @param callback - Function called whenever an attestation event occurs
 * @param options - Transport, timing, and reconnect options
 * @returns An unsubscribe function that cancels all subscriptions and polling
 */
export function watchSubject(
  client: TrustLinkClient,
  subject: string,
  callback: SubjectWatchCallback,
  options: WatchSubjectOptions = {}
): UnsubscribeFunction {
  validateAddress(subject);

  const pollingIntervalMs = options.pollingIntervalMs ?? 5000;
  const maxReconnectAttempts = options.reconnectAttempts ?? 5;
  const baseReconnectDelayMs = options.reconnectDelayMs ?? 1000;
  const allowedTypes = new Set<SubjectEventType>(
    options.eventTypes ?? ["created", "revoked", "renewed"]
  );

  let isActive = true;
  let activeWs: WebSocketLike | null = null;
  let reconnectTimeout: ReturnType<typeof setTimeout> | null = null;
  let pollingTimer: ReturnType<typeof setInterval> | null = null;
  let reconnectCount = 0;
  let isPollingActive = false;

  // Track known attestations for RPC polling diffs: id -> { revoked, expiration }
  let knownAttestations = new Map<string, { revoked: boolean; expiration: bigint | null }>();
  let hasInitializedRpcState = false;

  function emitEvent(event: SubjectAttestationEvent) {
    if (!isActive) return;
    if (allowedTypes.has(event.type)) {
      try {
        const result = callback(event);
        if (result && typeof (result as Promise<void>).catch === "function") {
          (result as Promise<void>).catch((err) => {
            console.error("Error in watchSubject callback:", err);
          });
        }
      } catch (err) {
        console.error("Error in watchSubject callback:", err);
      }
    }
  }

  // ── RPC Polling Fallback ──────────────────────────────────────────────────

  async function pollRpcOnce() {
    if (!isActive) return;
    try {
      const attestations: Attestation[] = await client.getSubjectAttestations(
        subject,
        0,
        100
      );

      if (!hasInitializedRpcState) {
        // First poll: record initial baseline without firing false 'created' events
        for (const att of attestations) {
          knownAttestations.set(att.id, {
            revoked: att.revoked,
            expiration: att.expiration,
          });
        }
        hasInitializedRpcState = true;
        return;
      }

      const currentIds = new Set<string>();

      for (const att of attestations) {
        currentIds.add(att.id);
        const previous = knownAttestations.get(att.id);

        if (!previous) {
          // New attestation created
          knownAttestations.set(att.id, {
            revoked: att.revoked,
            expiration: att.expiration,
          });
          emitEvent({
            type: "created",
            subject,
            attestationId: att.id,
            issuer: att.issuer,
            claimType: att.claim_type,
            timestamp: Number(att.timestamp),
            raw: att,
          });
        } else {
          // Check for revocation
          if (!previous.revoked && att.revoked) {
            previous.revoked = true;
            emitEvent({
              type: "revoked",
              subject,
              attestationId: att.id,
              issuer: att.issuer,
              claimType: att.claim_type,
              timestamp: Date.now(),
              raw: att,
            });
          }

          // Check for renewal (expiration changed to a later date)
          if (
            att.expiration != null &&
            previous.expiration != null &&
            att.expiration > previous.expiration
          ) {
            previous.expiration = att.expiration;
            emitEvent({
              type: "renewed",
              subject,
              attestationId: att.id,
              issuer: att.issuer,
              claimType: att.claim_type,
              timestamp: Date.now(),
              raw: att,
            });
          }
        }
      }
    } catch (err) {
      // Do not crash polling loop on transient network failure
      console.warn("RPC polling check failed:", err);
    }
  }

  function startRpcPolling() {
    if (!isActive || isPollingActive) return;
    isPollingActive = true;

    // Run first poll immediately
    void pollRpcOnce();

    pollingTimer = setInterval(() => {
      void pollRpcOnce();
    }, pollingIntervalMs);
    pollingTimer.unref?.();
  }

  // ── GraphQL WebSocket Transport ──────────────────────────────────────────

  function connectWebSocket(): void {
    if (!isActive) return;

    const wsUrl = options.graphqlUrl;
    const wsFactory =
      options.webSocketFactory ??
      (typeof globalThis !== "undefined" ? (globalThis as any).WebSocket : null);

    if (!wsUrl || !wsFactory) {
      // No WebSocket URL or factory available -> start RPC polling directly
      startRpcPolling();
      return;
    }

    try {
      const ws: WebSocketLike = wsFactory(wsUrl, "graphql-transport-ws");
      activeWs = ws;

      ws.onopen = () => {
        if (!isActive) {
          ws.close(1000, "Unsubscribed");
          return;
        }
        reconnectCount = 0; // Reset reconnect counter on successful connect
        // Send connection_init
        ws.send(JSON.stringify({ type: "connection_init", payload: {} }));
      };

      ws.onmessage = (event: { data: any }) => {
        if (!isActive) return;
        try {
          const rawText = typeof event.data === "string" ? event.data : event.data?.toString?.();
          if (!rawText) return;
          const msg = JSON.parse(rawText);

          if (msg.type === "connection_ack") {
            // Subscribe to onAttestationCreated
            ws.send(
              JSON.stringify({
                id: "sub_created",
                type: "subscribe",
                payload: {
                  query: `
                    subscription OnCreated($subject: String) {
                      onAttestationCreated(subject: $subject) {
                        id
                        issuer
                        subject
                        claimType
                        timestamp
                      }
                    }
                  `,
                  variables: { subject },
                },
              })
            );

            // Subscribe to onAttestationRevoked
            ws.send(
              JSON.stringify({
                id: "sub_revoked",
                type: "subscribe",
                payload: {
                  query: `
                    subscription OnRevoked($subject: String) {
                      onAttestationRevoked(subject: $subject) {
                        id
                        issuer
                        subject
                        claimType
                        revokedAt
                      }
                    }
                  `,
                  variables: { subject },
                },
              })
            );
            return;
          }

          if (msg.type === "next") {
            const data = msg.payload?.data;
            if (data?.onAttestationCreated) {
              const item = data.onAttestationCreated;
              emitEvent({
                type: "created",
                subject,
                attestationId: item.id,
                issuer: item.issuer,
                claimType: item.claimType,
                timestamp: item.timestamp ? Number(item.timestamp) : undefined,
                raw: item,
              });
            } else if (data?.onAttestationRevoked) {
              const item = data.onAttestationRevoked;
              emitEvent({
                type: "revoked",
                subject,
                attestationId: item.id,
                issuer: item.issuer,
                claimType: item.claimType,
                timestamp: item.revokedAt ? Number(item.revokedAt) : undefined,
                raw: item,
              });
            }
          }
        } catch (e) {
          console.warn("Failed to parse incoming WebSocket message:", e);
        }
      };

      ws.onerror = (_err: any) => {
        // Will be followed by onclose
      };

      ws.onclose = (event: { code: number; reason?: string }) => {
        activeWs = null;
        if (!isActive) return;

        // Code 1000 is normal closure
        if (event.code !== 1000) {
          if (reconnectCount < maxReconnectAttempts) {
            const delay = baseReconnectDelayMs * Math.pow(2, reconnectCount);
            reconnectCount++;
            reconnectTimeout = setTimeout(() => {
              connectWebSocket();
            }, delay);
            reconnectTimeout.unref?.();
          } else {
            // Reconnect attempts exhausted -> fall back to RPC polling
            startRpcPolling();
          }
        }
      };
    } catch (_err) {
      // Immediate failure constructing WebSocket -> fallback
      startRpcPolling();
    }
  }

  // Initialize
  if (options.graphqlUrl) {
    connectWebSocket();
  } else {
    startRpcPolling();
  }

  // Return unsubscribe handler
  return function unsubscribe(): void {
    isActive = false;

    if (reconnectTimeout != null) {
      clearTimeout(reconnectTimeout);
      reconnectTimeout = null;
    }

    if (pollingTimer != null) {
      clearInterval(pollingTimer);
      pollingTimer = null;
    }

    if (activeWs) {
      try {
        activeWs.close(1000, "Normal closure");
      } catch (_e) {
        // Ignore errors during closure
      }
      activeWs = null;
    }
  };
}
