import { PrismaClient } from "@prisma/client";
import Fastify, { FastifyInstance } from "fastify";
import { requireApiKey } from "./auth";
import {
  buildAttestationExportWhere,
  createAttestationExportStream,
  parseExportFormat,
  parseTimestampQuery,
} from "./export";
import { getLastLedger } from "./indexer-state";
import { getMetrics } from "./metrics";

export function buildRestServer(db: PrismaClient): FastifyInstance {
  const fastify = Fastify({ logger: false });

  fastify.get("/health", async (_request, reply) => {
    let dbConnected = false;
    try {
      await db.$queryRaw`SELECT 1`;
      dbConnected = true;
    } catch {
      dbConnected = false;
    }

    if (!dbConnected) {
      reply.code(503);
      return {
        status: "error",
        db: "disconnected",
        lastLedger: getLastLedger(),
      };
    }

    return {
      status: "ok",
      db: "connected",
      lastLedger: getLastLedger(),
    };
  });

  fastify.get("/ready", async () => {
    const checkpoint = await db.checkpoint.findUnique({ where: { id: 1 } });
    const rpc = new (await import("@stellar/stellar-sdk")).rpc.Server(
      process.env.RPC_URL ?? "https://soroban-testnet.stellar.org",
      { allowHttp: true },
    );
    const { sequence: tip } = await rpc.getLatestLedger();
    const lag = tip - (checkpoint?.ledger ?? 0);
    if (lag <= 10) {
      return { status: 200 };
    }
    return { status: 503 };
  });

  fastify.get("/metrics", async () => getMetrics());

  fastify.get<{ Params: { subject: string } }>(
    "/attestations/:subject",
    async (req) =>
      db.attestation.findMany({
        where: { subject: req.params.subject },
        orderBy: { timestamp: "desc" },
      }),
  );

  fastify.get<{ Params: { address: string; claim_type: string } }>(
    "/subjects/:address/claims/:claim_type/valid",
    async (req) => {
      const attestation = await db.attestation.findFirst({
        where: {
          subject: req.params.address,
          claimType: req.params.claim_type,
          isRevoked: false,
        },
      });
      return { valid: !!attestation };
    },
  );

  fastify.get<{ Params: { address: string } }>(
    "/issuers/:address/attestations",
    async (req) =>
      db.attestation.findMany({
        where: { issuer: req.params.address },
        orderBy: { timestamp: "desc" },
      }),
  );

  fastify.get<{
    Querystring: {
      format?: string;
      issuer?: string;
      subject?: string;
      claim_type?: string;
      from?: string;
      to?: string;
    };
  }>("/export/attestations", async (req, reply) => {
    if (!requireApiKey(req, reply)) {
      return;
    }

    const format = parseExportFormat(req.query.format);
    if (!format) {
      reply.code(400);
      return { error: "Invalid format; use csv or json" };
    }

    const fromTimestamp = parseTimestampQuery(req.query.from);
    const toTimestamp = parseTimestampQuery(req.query.to);
    if ((req.query.from && fromTimestamp === undefined) || (req.query.to && toTimestamp === undefined)) {
      reply.code(400);
      return { error: "Invalid from/to timestamp" };
    }

    const where = buildAttestationExportWhere({
      issuer: req.query.issuer,
      subject: req.query.subject,
      claimType: req.query.claim_type,
      fromTimestamp,
      toTimestamp,
    });

    const contentType = format === "csv" ? "text/csv; charset=utf-8" : "application/json; charset=utf-8";
    const extension = format === "csv" ? "csv" : "json";

    reply
      .header("Content-Type", contentType)
      .header("Content-Disposition", `attachment; filename="attestations.${extension}"`)
      .header("Transfer-Encoding", "chunked");

    return reply.send(createAttestationExportStream(db, where, format));
  });

  fastify.get("/stats", async () => {
    const [total, revoked, issuers] = await Promise.all([
      db.attestation.count(),
      db.attestation.count({ where: { isRevoked: true } }),
      db.attestation.findMany({
        distinct: ["issuer"],
        select: { issuer: true },
      }),
    ]);
    return {
      total_attestations: total,
      total_revocations: revoked,
      total_issuers: issuers.length,
    };
  });

  fastify.get("/webhooks", async () =>
    db.webhook.findMany({
      select: { id: true, url: true, active: true, createdAt: true },
      orderBy: { createdAt: "desc" },
    }),
  );

  fastify.post<{ Body: { url: string; secret: string } }>(
    "/webhooks",
    async (req, reply) => {
      const { url, secret } = req.body ?? {};
      if (!url || !secret) {
        reply.code(400);
        return { error: "url and secret are required" };
      }
      const webhook = await db.webhook.create({ data: { url, secret } });
      reply.code(201);
      return { id: webhook.id, url: webhook.url, active: webhook.active };
    },
  );

  fastify.delete<{ Params: { id: string } }>(
    "/webhooks/:id",
    async (req, reply) => {
      try {
        await db.webhook.delete({ where: { id: req.params.id } });
        reply.code(204);
        return;
      } catch {
        reply.code(404);
        return { error: "Webhook not found" };
      }
    },
  );

  fastify.post<{ Querystring: { from?: string } }>(
    "/admin/reindex",
    async (req, reply) => {
      const from = req.query.from ? parseInt(req.query.from, 10) : getLastLedger();
      if (isNaN(from) || from < 0) {
        reply.code(400);
        return { error: "Invalid 'from' ledger number" };
      }
      reply.code(202);
      return { message: `Reindex started from ledger ${from}` };
    },
  );

  fastify.get<{
    Querystring: {
      status?: string;
      eventType?: string;
      limit?: string;
      offset?: string;
      sort?: string;
    };
  }>("/admin/webhook-failures", async (req, reply) => {
    const { status, eventType, limit: limitStr, offset: offsetStr, sort } = req.query;
    const limit = Math.min(parseInt(limitStr ?? "50", 10) || 50, 200);
    const offset = parseInt(offsetStr ?? "0", 10) || 0;
    const orderBy = sort === "asc" ? "asc" : "desc";

    const where: Record<string, unknown> = {};
    if (status) {
      if (!["FAILED", "RETRYING", "RECOVERED"].includes(status)) {
        reply.code(400);
        return { error: "Invalid status filter" };
      }
      where.status = status;
    }
    if (eventType) {
      where.eventType = eventType;
    }

    const [items, total] = await Promise.all([
      db.webhookFailure.findMany({
        where,
        orderBy: { failedAt: orderBy },
        skip: offset,
        take: limit,
        select: {
          id: true,
          webhookId: true,
          url: true,
          eventType: true,
          statusCode: true,
          errorMessage: true,
          attemptCount: true,
          status: true,
          failedAt: true,
          resolvedAt: true,
          updatedAt: true,
        },
      }),
      db.webhookFailure.count({ where }),
    ]);

    return { items, total, limit, offset };
  });

  fastify.post<{ Params: { id: string } }>(
    "/admin/retry-webhook/:id",
    async (req, reply) => {
      const { id } = req.params;
      if (!id) {
        reply.code(400);
        return { error: "Missing failure id" };
      }
      const { replayFailure } = await import("./webhooks");
      const result = await replayFailure(db, id);
      if (result.error === "Not found") {
        reply.code(404);
        return { error: "Webhook failure record not found" };
      }
      if (result.error === "Retry already in progress") {
        reply.code(409);
        return { error: result.error };
      }
      if (result.success) {
        return { success: true, statusCode: result.statusCode };
      }
      reply.code(502);
      return { success: false, statusCode: result.statusCode, error: result.error };
    },
  );

  return fastify;
}
