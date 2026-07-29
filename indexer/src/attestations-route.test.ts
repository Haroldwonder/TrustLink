jest.mock("./metrics", () => ({
  getMetrics: jest.fn().mockResolvedValue(""),
}));

import { buildRestServer } from "./rest";

function makeAttestation(subject: string, id = "att-1") {
  return {
    id,
    issuer: "GISSUER",
    subject,
    claimType: "KYC_PASSED",
    timestamp: 1_700_000_000,
    expiration: null,
    isRevoked: false,
    revocationReason: null,
    metadata: null,
    imported: false,
    bridged: false,
    sourceChain: null,
    sourceTx: null,
    createdAt: new Date("2024-01-01T00:00:00.000Z"),
    updatedAt: new Date("2024-01-01T00:00:00.000Z"),
  };
}

describe("GET /attestations/:subject route regression", () => {
  const subject = "GSUBJECT123";

  it("returns attestations for the requested subject via real HTTP route", async () => {
    const findMany = jest.fn().mockResolvedValue([makeAttestation(subject)]);
    const app = buildRestServer({ attestation: { findMany } } as never);

    const response = await app.inject({
      method: "GET",
      url: `/attestations/${subject}`,
    });

    expect(response.statusCode).toBe(200);
    expect(findMany).toHaveBeenCalledWith({
      where: { subject },
      orderBy: { timestamp: "desc" },
    });

    const body = JSON.parse(response.body) as Array<{ subject: string }>;
    expect(body).toHaveLength(1);
    expect(body[0]?.subject).toBe(subject);
  });

  it("uses req.params.subject, not req.params.address", async () => {
    const findMany = jest.fn().mockResolvedValue([]);
    const app = buildRestServer({ attestation: { findMany } } as never);

    await app.inject({
      method: "GET",
      url: `/attestations/${subject}`,
    });

    const call = findMany.mock.calls[0]?.[0] as { where: { subject?: string; address?: string } };
    expect(call.where.subject).toBe(subject);
    expect(call.where).not.toHaveProperty("address");
  });
});
