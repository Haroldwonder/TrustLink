import { jest } from "@jest/globals";
import { Keypair } from "@stellar/stellar-sdk";
import { TrustLinkClient } from "../src/client.js";
import { watchSubject } from "../src/watchSubject.js";
import { InvalidAddressError } from "../src/validation.js";
import type { Attestation } from "../src/types.js";
import type { SubjectWatchCallback } from "../src/watchSubject.js";

describe("watchSubject", () => {
  const CONTRACT_ID = "CAAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQC526";
  const SUBJECT = Keypair.random().publicKey();

  let client: TrustLinkClient;

  beforeEach(() => {
    client = new TrustLinkClient({ contractId: CONTRACT_ID, network: "testnet" });
  });

  afterEach(() => {
    jest.restoreAllMocks();
    jest.useRealTimers();
  });

  test("validates subject address format", () => {
    expect(() => {
      client.watchSubject("invalid_addr", () => {});
    }).toThrow(InvalidAddressError);
  });

  test("works as standalone watchSubject function", async () => {
    jest.spyOn(client, "getSubjectAttestations").mockResolvedValue([]);
    const unsub = watchSubject(client, SUBJECT, () => {});
    expect(typeof unsub).toBe("function");
    unsub();
  });

  describe("GraphQL WebSocket Transport", () => {
    class MockWebSocket {
      url: string;
      protocols?: string | string[];
      sentMessages: string[] = [];
      readyState = 1; // OPEN
      onopen: ((event: any) => void) | null = null;
      onmessage: ((event: { data: any }) => void) | null = null;
      onerror: ((event: any) => void) | null = null;
      onclose: ((event: { code: number; reason?: string }) => void) | null = null;

      constructor(url: string, protocols?: string | string[]) {
        this.url = url;
        this.protocols = protocols;
        // Trigger onopen asynchronously
        const t = setTimeout(() => {
          if (this.onopen) this.onopen({});
        }, 10);
        t.unref?.();
      }

      send(data: string) {
        this.sentMessages.push(data);
      }

      close(code = 1000, reason = "") {
        this.readyState = 3; // CLOSED
        if (this.onclose) this.onclose({ code, reason });
      }

      // Test helper to simulate incoming server message
      receive(payload: any) {
        if (this.onmessage) {
          this.onmessage({ data: JSON.stringify(payload) });
        }
      }
    }

    test("connects, negotiates GraphQL subscription protocol, and receives created and revoked events", async () => {
      let activeMockWs: MockWebSocket | null = null;
      const callback = jest.fn<SubjectWatchCallback>();

      const unsubscribe = client.watchSubject(SUBJECT, callback, {
        graphqlUrl: "ws://localhost:4000/graphql",
        webSocketFactory: (url, protocols) => {
          activeMockWs = new MockWebSocket(url, protocols);
          return activeMockWs;
        },
      });

      // Wait for onopen
      await new Promise((r) => setTimeout(r, 20));

      expect(activeMockWs).not.toBeNull();
      // Verifies connection_init was sent
      expect(activeMockWs!.sentMessages).toContain(
        JSON.stringify({ type: "connection_init", payload: {} })
      );

      // Simulate connection_ack
      activeMockWs!.receive({ type: "connection_ack" });

      // Verifies subscribe payloads were sent for created and revoked
      expect(
        activeMockWs!.sentMessages.some(
          (m) => m.includes("subscribe") && m.includes("onAttestationCreated")
        )
      ).toBe(true);
      expect(
        activeMockWs!.sentMessages.some(
          (m) => m.includes("subscribe") && m.includes("onAttestationRevoked")
        )
      ).toBe(true);

      // Simulate onAttestationCreated next payload
      activeMockWs!.receive({
        type: "next",
        id: "sub_created",
        payload: {
          data: {
            onAttestationCreated: {
              id: "att_001",
              issuer: "G_ISSUER",
              subject: SUBJECT,
              claimType: "KYC_PASSED",
              timestamp: 1700000000,
            },
          },
        },
      });

      expect(callback).toHaveBeenCalledWith(
        expect.objectContaining({
          type: "created",
          subject: SUBJECT,
          attestationId: "att_001",
          issuer: "G_ISSUER",
          claimType: "KYC_PASSED",
        })
      );

      // Simulate onAttestationRevoked next payload
      activeMockWs!.receive({
        type: "next",
        id: "sub_revoked",
        payload: {
          data: {
            onAttestationRevoked: {
              id: "att_001",
              issuer: "G_ISSUER",
              subject: SUBJECT,
              claimType: "KYC_PASSED",
              revokedAt: 1700000500,
            },
          },
        },
      });

      expect(callback).toHaveBeenCalledWith(
        expect.objectContaining({
          type: "revoked",
          subject: SUBJECT,
          attestationId: "att_001",
        })
      );

      unsubscribe();
      expect(activeMockWs!.readyState).toBe(3); // CLOSED
    });

    test("handles reconnection on abnormal close", async () => {
      const createdSockets: MockWebSocket[] = [];

      const unsubscribe = client.watchSubject(SUBJECT, () => {}, {
        graphqlUrl: "ws://localhost:4000/graphql",
        reconnectAttempts: 2,
        reconnectDelayMs: 20,
        webSocketFactory: (url, protocols) => {
          const ws = new MockWebSocket(url, protocols);
          createdSockets.push(ws);
          return ws;
        },
      });

      // Wait for first socket to open
      await new Promise((r) => setTimeout(r, 20));
      expect(createdSockets.length).toBe(1);

      // Simulate abnormal server drop
      createdSockets[0].close(1006, "Abnormal closure");

      // Wait for backoff reconnect delay
      await new Promise((r) => setTimeout(r, 50));
      expect(createdSockets.length).toBe(2);

      unsubscribe();
    });
  });

  describe("RPC Polling Fallback", () => {
    test("polls RPC and detects created, revoked, and renewed attestations", async () => {
      const getSubjectAttestationsSpy = jest.spyOn(client, "getSubjectAttestations");
      const callback = jest.fn<SubjectWatchCallback>();

      const baseAttestation: Attestation = {
        id: "att_01",
        issuer: "G_ISSUER",
        subject: SUBJECT,
        claim_type: "KYC_PASSED",
        timestamp: BigInt(1700000000),
        expiration: BigInt(1700500000),
        revoked: false,
        metadata: null,
        valid_from: null,
        origin: "Native",
        source_chain: null,
        source_tx: null,
        tags: null,
        jurisdiction: null,
        revocation_reason: null,
        deleted: false,
      };

      // 1st call: Initial baseline
      getSubjectAttestationsSpy.mockResolvedValueOnce([baseAttestation]);

      const unsubscribe = client.watchSubject(SUBJECT, callback, {
        pollingIntervalMs: 20,
      });

      // Wait for initial poll
      await new Promise((r) => setTimeout(r, 30));
      expect(callback).not.toHaveBeenCalled();

      // 2nd call: New attestation created
      const newAttestation: Attestation = {
        ...baseAttestation,
        id: "att_02",
        claim_type: "ACCREDITED_INVESTOR",
      };
      getSubjectAttestationsSpy.mockResolvedValueOnce([baseAttestation, newAttestation]);

      // Wait for next interval
      await new Promise((r) => setTimeout(r, 30));
      expect(callback).toHaveBeenCalledWith(
        expect.objectContaining({
          type: "created",
          attestationId: "att_02",
          claimType: "ACCREDITED_INVESTOR",
        })
      );

      // 3rd call: att_01 revoked
      const revokedAttestation: Attestation = {
        ...baseAttestation,
        revoked: true,
      };
      getSubjectAttestationsSpy.mockResolvedValueOnce([revokedAttestation, newAttestation]);

      await new Promise((r) => setTimeout(r, 30));
      expect(callback).toHaveBeenCalledWith(
        expect.objectContaining({
          type: "revoked",
          attestationId: "att_01",
        })
      );

      // 4th call: att_02 renewed (expiration extended)
      const renewedAttestation: Attestation = {
        ...newAttestation,
        expiration: BigInt(1700900000), // Extended expiration
      };
      getSubjectAttestationsSpy.mockResolvedValueOnce([revokedAttestation, renewedAttestation]);

      await new Promise((r) => setTimeout(r, 30));
      expect(callback).toHaveBeenCalledWith(
        expect.objectContaining({
          type: "renewed",
          attestationId: "att_02",
        })
      );

      unsubscribe();
    });
  });
});
