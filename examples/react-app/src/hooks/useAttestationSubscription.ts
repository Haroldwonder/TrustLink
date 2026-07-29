import { useEffect, useRef } from "react";

export interface AttestationEvent {
  type: "attestation_created" | "attestation_revoked";
  id: string;
  subject: string;
  issuer: string;
  claim_type: string;
}

const GQL_SUBSCRIPTION_CREATED = (address: string) => JSON.stringify({
  id: "sub_created",
  type: "subscribe",
  payload: {
    query: `subscription { onAttestationCreated(subject: "${address}") { id subject issuer claimType } }`,
  },
});

const GQL_SUBSCRIPTION_REVOKED = (address: string) => JSON.stringify({
  id: "sub_revoked",
  type: "subscribe",
  payload: {
    query: `subscription { onAttestationRevoked(subject: "${address}") { id subject issuer claimType } }`,
  },
});

export function useAttestationSubscription(
  address: string | null,
  onEvent: (event: AttestationEvent) => void
) {
  const onEventRef = useRef(onEvent);
  onEventRef.current = onEvent;

  useEffect(() => {
    if (!address) return;
    const indexerUrl = (import.meta as { env: Record<string, string> }).env.VITE_INDEXER_WS_URL;
    if (!indexerUrl) return;

    const ws = new WebSocket(indexerUrl, "graphql-transport-ws");

    ws.onopen = () => {
      ws.send(JSON.stringify({ type: "connection_init", payload: {} }));
    };

    ws.onmessage = (msg: MessageEvent) => {
      let data: { type: string; id?: string; payload?: { data?: Record<string, unknown> } };
      try {
        data = JSON.parse(msg.data as string) as typeof data;
      } catch {
        return;
      }

      if (data.type === "connection_ack") {
        ws.send(GQL_SUBSCRIPTION_CREATED(address));
        ws.send(GQL_SUBSCRIPTION_REVOKED(address));
        return;
      }

      if (data.type === "next" && data.payload?.data) {
        const payload = data.payload.data;

        if (data.id === "sub_created" && payload.onAttestationCreated) {
          const raw = payload.onAttestationCreated as { id: string; subject: string; issuer: string; claimType: string };
          onEventRef.current({
            type: "attestation_created",
            id: raw.id,
            subject: raw.subject,
            issuer: raw.issuer,
            claim_type: raw.claimType,
          });
        }

        if (data.id === "sub_revoked" && payload.onAttestationRevoked) {
          const raw = payload.onAttestationRevoked as { id: string; subject: string; issuer: string; claimType: string };
          onEventRef.current({
            type: "attestation_revoked",
            id: raw.id,
            subject: raw.subject,
            issuer: raw.issuer,
            claim_type: raw.claimType,
          });
        }
      }
    };

    return () => {
      if (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING) {
        ws.close();
      }
    };
  }, [address]);
}
