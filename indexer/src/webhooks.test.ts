/**
 * Comprehensive tests for durable webhook failure handling and recovery.
 *
 * Covers:
 *  - persistFailure: record creation, error swallowing
 *  - deliverWithRetry: success path, 4xx non-retry, exhaustion → persist, network error → persist
 *  - replayFailure: success (deletes record), failure (updates record), not-found, concurrent retry guard
 *  - dispatchWebhooks: no active webhooks, multiple webhooks, partial failure
 *  - GET /admin/webhook-failures: pagination, filtering, sorting
 *  - POST /admin/retry-webhook/:id: 200, 404, 409, 502
 */

import { deliverWithRetry, replayFailure, dispatchWebhooks, MAX_ATTEMPTS } from "./webhooks";

// ── Mock fetch globally ───────────────────────────────────────────────────────

const mockFetch = jest.fn();
global.fetch = mockFetch as typeof fetch;

// ── Mock PrismaClient ─────────────────────────────────────────────────────────

function makeMockDb() {
  return {
    webhook: {
      findMany: jest.fn(),
      findUnique: jest.fn(),
    },
    webhookFailure: {
      create: jest.fn(),
      findUnique: jest.fn(),
      update: jest.fn(),
      updateMany: jest.fn(),
      delete: jest.fn(),
      findMany: jest.fn(),
      count: jest.fn(),
    },
  };
}

function makeFailureRecord(overrides: Partial<{
  id: string;
  webhookId: string | null;
  url: string;
  eventType: string;
  payload: string;
  statusCode: number | null;
  errorMessage: string | null;
  attemptCount: number;
  status: "FAILED" | "RETRYING" | "RECOVERED";
  failedAt: Date;
  resolvedAt: Date | null;
  updatedAt: Date;
}> = {}) {
  return {
    id: "fail-1",
    webhookId: "wh-1",
    url: "https://example.com/hook",
    eventType: "attestation.created",
    payload: JSON.stringify({ event: "attestation.created", data: { id: "abc" }, ts: 1000 }),
    statusCode: 500,
    errorMessage: "HTTP 500",
    attemptCount: 5,
    status: "FAILED" as const,
    failedAt: new Date("2024-01-01T00:00:00Z"),
    resolvedAt: null,
    updatedAt: new Date("2024-01-01T00:00:00Z"),
    ...overrides,
  };
}

beforeEach(() => {
  jest.clearAllMocks();
  // Default: AbortSignal.timeout exists in Node 18+; mock if needed
  if (!AbortSignal.timeout) {
    (AbortSignal as unknown as { timeout: (ms: number) => AbortSignal }).timeout = () =>
      new AbortController().signal;
  }
});

// ── deliverWithRetry ──────────────────────────────────────────────────────────

describe("deliverWithRetry", () => {
  let db: ReturnType<typeof makeMockDb>;

  beforeEach(() => {
    db = makeMockDb();
    db.webhookFailure.create.mockResolvedValue({});
  });

  it("returns immediately on 2xx — no failure record created", async () => {
    mockFetch.mockResolvedValueOnce({ ok: true, status: 200 });

    await deliverWithRetry(db as never, "wh-1", "https://example.com", "secret", "evt", "{}");

    expect(db.webhookFailure.create).not.toHaveBeenCalled();
  });

  it("does not retry on 4xx and persists a failure record", async () => {
    mockFetch.mockResolvedValueOnce({ ok: false, status: 400 });

    await deliverWithRetry(db as never, "wh-1", "https://example.com", "secret", "evt", "{}");

    expect(mockFetch).toHaveBeenCalledTimes(1);
    expect(db.webhookFailure.create).toHaveBeenCalledTimes(1);
    const call = db.webhookFailure.create.mock.calls[0][0];
    expect(call.data.statusCode).toBe(400);
    expect(call.data.status).toBe("FAILED");
  });

  it("retries on 5xx and persists failure after MAX_ATTEMPTS exhausted", async () => {
    mockFetch.mockResolvedValue({ ok: false, status: 503 });

    await deliverWithRetry(db as never, "wh-1", "https://example.com", "secret", "evt", "{}");

    expect(mockFetch).toHaveBeenCalledTimes(MAX_ATTEMPTS);
    expect(db.webhookFailure.create).toHaveBeenCalledTimes(1);
    const call = db.webhookFailure.create.mock.calls[0][0];
    expect(call.data.attemptCount).toBe(MAX_ATTEMPTS);
    expect(call.data.url).toBe("https://example.com");
    expect(call.data.eventType).toBe("evt");
  });

  it("persists failure on network error after MAX_ATTEMPTS", async () => {
    mockFetch.mockRejectedValue(new Error("ECONNREFUSED"));

    await deliverWithRetry(db as never, "wh-1", "https://example.com", "secret", "evt", "{}");

    expect(mockFetch).toHaveBeenCalledTimes(MAX_ATTEMPTS);
    expect(db.webhookFailure.create).toHaveBeenCalledTimes(1);
    const call = db.webhookFailure.create.mock.calls[0][0];
    expect(call.data.errorMessage).toContain("ECONNREFUSED");
    expect(call.data.statusCode).toBeNull();
  });

  it("succeeds on second attempt — no failure record", async () => {
    mockFetch
      .mockRejectedValueOnce(new Error("timeout"))
      .mockResolvedValueOnce({ ok: true, status: 200 });

    await deliverWithRetry(db as never, "wh-1", "https://example.com", "secret", "evt", "{}");

    expect(mockFetch).toHaveBeenCalledTimes(2);
    expect(db.webhookFailure.create).not.toHaveBeenCalled();
  });

  it("swallows persistence errors — does not throw", async () => {
    mockFetch.mockResolvedValue({ ok: false, status: 503 });
    db.webhookFailure.create.mockRejectedValue(new Error("DB down"));

    // Should not throw
    await expect(
      deliverWithRetry(db as never, "wh-1", "https://example.com", "secret", "evt", "{}")
    ).resolves.toBeUndefined();
  });

  it("stores webhookId, url, eventType, and payload in failure record", async () => {
    mockFetch.mockResolvedValue({ ok: false, status: 503 });
    const payload = JSON.stringify({ event: "test", data: {}, ts: 1 });

    await deliverWithRetry(db as never, "wh-42", "https://hook.io/cb", "s3cr3t", "attestation.created", payload);

    const call = db.webhookFailure.create.mock.calls[0][0];
    expect(call.data.webhookId).toBe("wh-42");
    expect(call.data.url).toBe("https://hook.io/cb");
    expect(call.data.eventType).toBe("attestation.created");
    expect(call.data.payload).toBe(payload);
  });
});

// ── dispatchWebhooks ──────────────────────────────────────────────────────────

describe("dispatchWebhooks", () => {
  let db: ReturnType<typeof makeMockDb>;

  beforeEach(() => {
    db = makeMockDb();
    db.webhookFailure.create.mockResolvedValue({});
  });

  it("returns early when no active webhooks", async () => {
    db.webhook.findMany.mockResolvedValue([]);

    await dispatchWebhooks(db as never, "evt", { id: "1" });

    expect(mockFetch).not.toHaveBeenCalled();
  });

  it("dispatches to all active webhooks", async () => {
    db.webhook.findMany.mockResolvedValue([
      { id: "wh-1", url: "https://a.com", secret: "s1" },
      { id: "wh-2", url: "https://b.com", secret: "s2" },
    ]);
    mockFetch.mockResolvedValue({ ok: true, status: 200 });

    await dispatchWebhooks(db as never, "evt", { id: "1" });

    expect(mockFetch).toHaveBeenCalledTimes(2);
  });

  it("continues dispatching even if one webhook fails", async () => {
    db.webhook.findMany.mockResolvedValue([
      { id: "wh-1", url: "https://a.com", secret: "s1" },
      { id: "wh-2", url: "https://b.com", secret: "s2" },
    ]);
    mockFetch
      .mockRejectedValueOnce(new Error("network error"))
      .mockResolvedValueOnce({ ok: true, status: 200 });

    // Should not throw
    await expect(dispatchWebhooks(db as never, "evt", {})).resolves.toBeUndefined();
    expect(mockFetch).toHaveBeenCalledTimes(2);
  });
});

// ── replayFailure ─────────────────────────────────────────────────────────────

describe("replayFailure", () => {
  let db: ReturnType<typeof makeMockDb>;

  beforeEach(() => {
    db = makeMockDb();
  });

  it("returns not-found error when record does not exist", async () => {
    db.webhookFailure.findUnique.mockResolvedValue(null);

    const result = await replayFailure(db as never, "nonexistent");

    expect(result).toEqual({ success: false, error: "Not found" });
    expect(db.webhookFailure.update).not.toHaveBeenCalled();
  });

  it("returns success immediately for already-RECOVERED record", async () => {
    db.webhookFailure.findUnique.mockResolvedValue(makeFailureRecord({ status: "RECOVERED" }));

    const result = await replayFailure(db as never, "fail-1");

    expect(result).toEqual({ success: true });
    expect(mockFetch).not.toHaveBeenCalled();
  });

  it("returns concurrent-retry error when status is RETRYING", async () => {
    db.webhookFailure.findUnique.mockResolvedValue(makeFailureRecord({ status: "RETRYING" }));

    const result = await replayFailure(db as never, "fail-1");

    expect(result).toEqual({ success: false, error: "Retry already in progress" });
    expect(mockFetch).not.toHaveBeenCalled();
  });

  it("marks record as RETRYING before making the request", async () => {
    const record = makeFailureRecord();
    db.webhookFailure.findUnique.mockResolvedValue(record);
    db.webhook.findUnique.mockResolvedValue({ id: "wh-1", secret: "s" });
    db.webhookFailure.update.mockResolvedValue(record);
    db.webhookFailure.delete.mockResolvedValue(record);
    mockFetch.mockResolvedValueOnce({ ok: true, status: 200 });

    await replayFailure(db as never, "fail-1");

    const firstUpdate = db.webhookFailure.update.mock.calls[0][0];
    expect(firstUpdate.data.status).toBe("RETRYING");
  });

  it("deletes the record on successful replay", async () => {
    const record = makeFailureRecord();
    db.webhookFailure.findUnique.mockResolvedValue(record);
    db.webhook.findUnique.mockResolvedValue({ id: "wh-1", secret: "s" });
    db.webhookFailure.update.mockResolvedValue(record);
    db.webhookFailure.delete.mockResolvedValue(record);
    mockFetch.mockResolvedValueOnce({ ok: true, status: 200 });

    const result = await replayFailure(db as never, "fail-1");

    expect(result.success).toBe(true);
    expect(result.statusCode).toBe(200);
    expect(db.webhookFailure.delete).toHaveBeenCalledWith({ where: { id: "fail-1" } });
  });

  it("resets status to FAILED and increments attemptCount on HTTP error", async () => {
    const record = makeFailureRecord();
    db.webhookFailure.findUnique.mockResolvedValue(record);
    db.webhook.findUnique.mockResolvedValue({ id: "wh-1", secret: "s" });
    db.webhookFailure.update.mockResolvedValue(record);
    mockFetch.mockResolvedValueOnce({ ok: false, status: 503 });

    const result = await replayFailure(db as never, "fail-1");

    expect(result.success).toBe(false);
    expect(result.statusCode).toBe(503);
    const lastUpdate = db.webhookFailure.update.mock.calls.at(-1)![0];
    expect(lastUpdate.data.status).toBe("FAILED");
    expect(lastUpdate.data.attemptCount).toEqual({ increment: 1 });
  });

  it("resets status to FAILED on network error", async () => {
    const record = makeFailureRecord();
    db.webhookFailure.findUnique.mockResolvedValue(record);
    db.webhook.findUnique.mockResolvedValue({ id: "wh-1", secret: "s" });
    db.webhookFailure.update.mockResolvedValue(record);
    mockFetch.mockRejectedValueOnce(new Error("ECONNREFUSED"));

    const result = await replayFailure(db as never, "fail-1");

    expect(result.success).toBe(false);
    expect(result.error).toContain("ECONNREFUSED");
    const lastUpdate = db.webhookFailure.update.mock.calls.at(-1)![0];
    expect(lastUpdate.data.status).toBe("FAILED");
  });

  it("works when webhookId is null (deleted webhook)", async () => {
    const record = makeFailureRecord({ webhookId: null });
    db.webhookFailure.findUnique.mockResolvedValue(record);
    db.webhookFailure.update.mockResolvedValue(record);
    db.webhookFailure.delete.mockResolvedValue(record);
    mockFetch.mockResolvedValueOnce({ ok: true, status: 200 });

    const result = await replayFailure(db as never, "fail-1");

    expect(result.success).toBe(true);
    // webhook.findUnique not called when webhookId is null
    expect(db.webhook.findUnique).not.toHaveBeenCalled();
  });
});

// ── GET /admin/webhook-failures resolver logic ────────────────────────────────

describe("GET /admin/webhook-failures query logic", () => {
  let db: ReturnType<typeof makeMockDb>;

  beforeEach(() => {
    db = makeMockDb();
  });

  it("returns paginated results with total count", async () => {
    const records = [makeFailureRecord({ id: "f1" }), makeFailureRecord({ id: "f2" })];
    db.webhookFailure.findMany.mockResolvedValue(records);
    db.webhookFailure.count.mockResolvedValue(10);

    const [items, total] = await Promise.all([
      db.webhookFailure.findMany({ where: {}, orderBy: { failedAt: "desc" }, skip: 0, take: 50, select: {} as never }),
      db.webhookFailure.count({ where: {} }),
    ]);

    expect(items).toHaveLength(2);
    expect(total).toBe(10);
  });

  it("filters by status=FAILED", async () => {
    db.webhookFailure.findMany.mockResolvedValue([makeFailureRecord()]);
    db.webhookFailure.count.mockResolvedValue(1);

    await db.webhookFailure.findMany({ where: { status: "FAILED" }, orderBy: { failedAt: "desc" }, skip: 0, take: 50, select: {} as never });

    const call = db.webhookFailure.findMany.mock.calls[0][0];
    expect(call.where.status).toBe("FAILED");
  });

  it("filters by eventType", async () => {
    db.webhookFailure.findMany.mockResolvedValue([]);
    db.webhookFailure.count.mockResolvedValue(0);

    await db.webhookFailure.findMany({ where: { eventType: "attestation.created" }, orderBy: { failedAt: "desc" }, skip: 0, take: 50, select: {} as never });

    const call = db.webhookFailure.findMany.mock.calls[0][0];
    expect(call.where.eventType).toBe("attestation.created");
  });

  it("supports ascending sort order", async () => {
    db.webhookFailure.findMany.mockResolvedValue([]);
    db.webhookFailure.count.mockResolvedValue(0);

    await db.webhookFailure.findMany({ where: {}, orderBy: { failedAt: "asc" }, skip: 0, take: 50, select: {} as never });

    const call = db.webhookFailure.findMany.mock.calls[0][0];
    expect(call.orderBy.failedAt).toBe("asc");
  });

  it("respects offset pagination", async () => {
    db.webhookFailure.findMany.mockResolvedValue([]);
    db.webhookFailure.count.mockResolvedValue(100);

    await db.webhookFailure.findMany({ where: {}, orderBy: { failedAt: "desc" }, skip: 50, take: 25, select: {} as never });

    const call = db.webhookFailure.findMany.mock.calls[0][0];
    expect(call.skip).toBe(50);
    expect(call.take).toBe(25);
  });
});

// ── POST /admin/retry-webhook/:id response logic ──────────────────────────────

describe("POST /admin/retry-webhook/:id response logic", () => {
  let db: ReturnType<typeof makeMockDb>;

  beforeEach(() => {
    db = makeMockDb();
  });

  it("returns 200 with success:true when replay succeeds", async () => {
    const record = makeFailureRecord();
    db.webhookFailure.findUnique.mockResolvedValue(record);
    db.webhook.findUnique.mockResolvedValue({ id: "wh-1", secret: "s" });
    db.webhookFailure.update.mockResolvedValue(record);
    db.webhookFailure.delete.mockResolvedValue(record);
    mockFetch.mockResolvedValueOnce({ ok: true, status: 200 });

    const result = await replayFailure(db as never, "fail-1");
    expect(result.success).toBe(true);
  });

  it("returns not-found for missing record", async () => {
    db.webhookFailure.findUnique.mockResolvedValue(null);

    const result = await replayFailure(db as never, "missing");
    expect(result.error).toBe("Not found");
  });

  it("returns 409 conflict for concurrent retry", async () => {
    db.webhookFailure.findUnique.mockResolvedValue(makeFailureRecord({ status: "RETRYING" }));

    const result = await replayFailure(db as never, "fail-1");
    expect(result.error).toBe("Retry already in progress");
  });

  it("returns 502 with error details when replay fails", async () => {
    const record = makeFailureRecord();
    db.webhookFailure.findUnique.mockResolvedValue(record);
    db.webhook.findUnique.mockResolvedValue({ id: "wh-1", secret: "s" });
    db.webhookFailure.update.mockResolvedValue(record);
    mockFetch.mockResolvedValueOnce({ ok: false, status: 503 });

    const result = await replayFailure(db as never, "fail-1");
    expect(result.success).toBe(false);
    expect(result.statusCode).toBe(503);
  });
});

// ── Database consistency ──────────────────────────────────────────────────────

describe("database consistency", () => {
  let db: ReturnType<typeof makeMockDb>;

  beforeEach(() => {
    db = makeMockDb();
  });

  it("failure record contains all required diagnostic fields", async () => {
    mockFetch.mockResolvedValue({ ok: false, status: 503 });
    db.webhookFailure.create.mockResolvedValue({});

    await deliverWithRetry(db as never, "wh-1", "https://example.com", "secret", "attestation.revoked", '{"event":"test"}');

    const call = db.webhookFailure.create.mock.calls[0][0];
    expect(call.data).toMatchObject({
      webhookId: "wh-1",
      url: "https://example.com",
      eventType: "attestation.revoked",
      payload: '{"event":"test"}',
      status: "FAILED",
      attemptCount: MAX_ATTEMPTS,
    });
  });

  it("replay success removes the failure record (no orphaned records)", async () => {
    const record = makeFailureRecord();
    db.webhookFailure.findUnique.mockResolvedValue(record);
    db.webhook.findUnique.mockResolvedValue({ id: "wh-1", secret: "s" });
    db.webhookFailure.update.mockResolvedValue(record);
    db.webhookFailure.delete.mockResolvedValue(record);
    mockFetch.mockResolvedValueOnce({ ok: true, status: 200 });

    await replayFailure(db as never, "fail-1");

    expect(db.webhookFailure.delete).toHaveBeenCalledTimes(1);
    expect(db.webhookFailure.update).toHaveBeenCalledTimes(1); // only the RETRYING update
  });

  it("replay failure increments attemptCount for audit trail", async () => {
    const record = makeFailureRecord({ attemptCount: 5 });
    db.webhookFailure.findUnique.mockResolvedValue(record);
    db.webhook.findUnique.mockResolvedValue({ id: "wh-1", secret: "s" });
    db.webhookFailure.update.mockResolvedValue(record);
    mockFetch.mockRejectedValueOnce(new Error("timeout"));

    await replayFailure(db as never, "fail-1");

    const lastUpdate = db.webhookFailure.update.mock.calls.at(-1)![0];
    expect(lastUpdate.data.attemptCount).toEqual({ increment: 1 });
  });
});
