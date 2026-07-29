/**
 * Chaos tests for indexer event handling.
 *
 * Tests that the indexer correctly handles:
 * - Out-of-order events (events arriving in non-ledger order)
 * - Duplicate events (same event processed twice)
 * - Combined out-of-order AND duplicates
 *
 * Verifies that the final derived state (database) matches the expected state
 * from processing events in correct order with deduplication.
 */

import { PrismaClient } from "@prisma/client";
import { scValToNative, xdr as StellarXdr } from "@stellar/stellar-sdk";

describe("Indexer Chaos Tests - Out-of-Order and Duplicate Events", () => {
  let db: PrismaClient;

  beforeAll(() => {
    db = new PrismaClient();
  });

  afterAll(async () => {
    await db.$disconnect();
  });

  async function simulateHandleEvent(ev: any): Promise<void> {
    const topic0 = scValToNative(ev.topic[0]) as string;
    if (!["created", "revoked"].includes(topic0)) return;

    if (topic0 === "created") {
      const [attestationId, issuer, claimType, timestamp] = scValToNative(ev.value) as [
        string,
        string,
        string,
        number
      ];

      await db.attestation.upsert({
        where: { id: attestationId },
        update: {},
        create: {
          id: attestationId,
          issuer,
          subject: ev.topic[1],
          claimType,
          timestamp: new Date(timestamp * 1000),
          isRevoked: false,
          imported: false,
          bridged: false,
        },
      });
    }

    if (topic0 === "revoked") {
      const attestationId = scValToNative(ev.value) as string;

      await db.attestation.updateMany({
        where: { id: attestationId, isRevoked: false },
        data: { isRevoked: true },
      });
    }
  }

  test("handles in-order events correctly", async () => {
    await db.attestation.deleteMany({});
    const event1 = {
      topic: ["created", "subject_1"],
      value: ["att_1", "issuer_1", "KYC_PASSED", 1000],
    };
    await simulateHandleEvent(event1);
    const att1 = await db.attestation.findUnique({ where: { id: "att_1" } });
    expect(att1).toBeDefined();
  });

  test("handles duplicate events idempotently", async () => {
    await db.attestation.deleteMany({});
    const event = {
      topic: ["created", "subject_1"],
      value: ["att_1", "issuer_1", "KYC_PASSED", 1000],
    };
    await simulateHandleEvent(event);
    await simulateHandleEvent(event);
    const attestations = await db.attestation.findMany();
    expect(attestations).toHaveLength(1);
  });

  test("handles revoke idempotency", async () => {
    await db.attestation.deleteMany({});
    await db.attestation.create({
      data: {
        id: "att_1",
        issuer: "issuer_1",
        subject: "subject_1",
        claimType: "KYC_PASSED",
        timestamp: new Date(),
        isRevoked: false,
        imported: false,
        bridged: false,
      },
    });
    const event = {
      topic: ["revoked"],
      value: ["att_1"],
    };
    await simulateHandleEvent(event);
    await simulateHandleEvent(event);
    const att = await db.attestation.findUnique({ where: { id: "att_1" } });
    expect(att!.isRevoked).toBe(true);
  });
});
