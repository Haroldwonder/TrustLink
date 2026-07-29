jest.mock("./metrics", () => ({
  getMetrics: jest.fn().mockResolvedValue(""),
}));

import { EXPORT_BATCH_SIZE } from "./export";
import { buildRestServer } from "./rest";

function makeAttestation(index: number) {
  return {
    id: `att-${index}`,
    issuer: "GISSUER",
    subject: `GSUBJECT${index}`,
    claimType: "KYC_PASSED",
    timestamp: BigInt(1_700_000_000 + index),
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

function makeExportDb(totalRows: number) {
  const rows = Array.from({ length: totalRows }, (_, index) => makeAttestation(index));
  const findMany = jest.fn(async (args: { take: number; skip?: number; cursor?: { id: string } }) => {
    let start = 0;
    if (args.cursor) {
      const cursorIndex = rows.findIndex((row) => row.id === args.cursor?.id);
      start = cursorIndex + 1;
    }
    return rows.slice(start, start + args.take);
  });

  return { attestation: { findMany }, rows };
}

describe("GET /export/attestations", () => {
  const originalApiKeys = process.env.GRAPHQL_API_KEYS;

  afterEach(() => {
    if (originalApiKeys === undefined) {
      delete process.env.GRAPHQL_API_KEYS;
    } else {
      process.env.GRAPHQL_API_KEYS = originalApiKeys;
    }
  });

  it("requires authorization when API keys are configured", async () => {
    process.env.GRAPHQL_API_KEYS = "secret-key";
    const db = makeExportDb(1);
    const app = buildRestServer(db as never);

    const unauthorized = await app.inject({
      method: "GET",
      url: "/export/attestations?format=csv",
    });
    expect(unauthorized.statusCode).toBe(401);

    const authorized = await app.inject({
      method: "GET",
      url: "/export/attestations?format=csv",
      headers: { "x-api-key": "secret-key" },
    });
    expect(authorized.statusCode).toBe(200);
  });

  it("streams a filtered CSV export in batches", async () => {
    const db = makeExportDb(EXPORT_BATCH_SIZE + 25);
    const app = buildRestServer(db as never);

    const response = await app.inject({
      method: "GET",
      url: "/export/attestations?format=csv&issuer=GISSUER&claim_type=KYC_PASSED&from=1700000000&to=1700999999",
    });

    expect(response.statusCode).toBe(200);
    expect(response.headers["content-type"]).toContain("text/csv");
    expect(db.attestation.findMany).toHaveBeenCalledTimes(2);

    const lines = response.body.trim().split("\n");
    expect(lines[0]).toBe(
      "id,issuer,subject,claimType,timestamp,expiration,isRevoked,revocationReason,metadata,imported,bridged,sourceChain,sourceTx,createdAt,updatedAt",
    );
    expect(lines).toHaveLength(EXPORT_BATCH_SIZE + 26);
    expect(lines[1]).toContain("att-0");
    expect(lines.at(-1)).toContain(`att-${EXPORT_BATCH_SIZE + 24}`);
  });
});
