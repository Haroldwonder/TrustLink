import { PubSub } from "graphql-subscriptions";
import { PrismaClient, Attestation, MultisigProposal, AuditEntry } from "@prisma/client";
import type { Redis } from "ioredis";

export const pubsub = new PubSub();
export const ATTESTATION_CREATED = "ATTESTATION_CREATED";
export const ATTESTATION_REVOKED = "ATTESTATION_REVOKED";
export const ISSUER_REGISTERED = "ISSUER_REGISTERED";

// Cache TTL in seconds
const CACHE_TTL = 30;

type MappedAttestation = Omit<Attestation, "timestamp" | "expiration" | "createdAt" | "updatedAt"> & {
  timestamp: string;
  expiration: string | null;
  createdAt: string;
  updatedAt: string;
};

type MappedProposal = Omit<MultisigProposal, "expiresAt" | "createdAt" | "updatedAt"> & {
  expiresAt: string;
  createdAt: string;
  updatedAt: string;
};

type MappedAuditEntry = Omit<AuditEntry, "timestamp" | "createdAt"> & {
  timestamp: string;
  createdAt: string;
};

function mapAttestation(a: Attestation): MappedAttestation {
  return {
    ...a,
    timestamp: String(a.timestamp),
    expiration: a.expiration != null ? String(a.expiration) : null,
    createdAt: a.createdAt.toISOString(),
    updatedAt: a.updatedAt.toISOString(),
  };
}

function mapProposal(p: MultisigProposal): MappedProposal {
  return {
    ...p,
    expiresAt: String(p.expiresAt),
    createdAt: p.createdAt.toISOString(),
    updatedAt: p.updatedAt.toISOString(),
  };
}

function mapAuditEntry(e: AuditEntry): MappedAuditEntry {
  return {
    ...e,
    timestamp: String(e.timestamp),
    createdAt: e.createdAt.toISOString(),
  };
}

// #777: Redis cache helpers (redis may be null when not configured)
async function cacheGet(redis: Redis | null, key: string): Promise<unknown | null> {
  if (!redis) return null;
  try {
    const val = await redis.get(key);
    return val ? JSON.parse(val) : null;
  } catch {
    return null;
  }
}

async function cacheSet(redis: Redis | null, key: string, value: unknown): Promise<void> {
  if (!redis) return;
  try {
    await redis.set(key, JSON.stringify(value), "EX", CACHE_TTL);
  } catch {
    // cache errors are non-fatal
  }
}

export async function cacheInvalidate(redis: Redis | null, pattern: string): Promise<void> {
  if (!redis) return;
  try {
    const keys = await redis.keys(pattern);
    if (keys.length) await redis.del(...keys);
  } catch {
    // non-fatal
  }
}

export function buildResolvers(db: PrismaClient, redis: Redis | null = null) {
  return {
    Query: {
      healthCheck: async () => {
        let dbOk = false;
        try {
          await db.$queryRaw`SELECT 1`;
          dbOk = true;
        } catch {
          dbOk = false;
        }
        return {
          status: dbOk ? "ok" : "degraded",
          lastLedger: getLastLedger ? getLastLedger() : null,
          timestamp: new Date().toISOString(),
        };
      },

      attestations: async (
        _: unknown,
        args: { 
          subject?: string; 
          claimType?: string; 
          status?: "ACTIVE" | "REVOKED";
          first?: number;
          after?: string;
        }
      ): Promise<AttestationConnection> => {
        const where: Record<string, unknown> = {};
        if (args.subject) where.subject = args.subject;
        if (args.claimType) where.claimType = args.claimType;
        if (args.status === "ACTIVE") where.isRevoked = false;
        if (args.status === "REVOKED") where.isRevoked = true;

        return buildAttestationConnection(db, where, args.first, args.after);
      },

      attestationsByIssuer: async (
        _: unknown,
        args: {
          issuer: string;
          first?: number;
          after?: string;
        }
      ): Promise<AttestationConnection> => {
        const where = { issuer: args.issuer };
        return buildAttestationConnection(db, where, args.first, args.after);
      },

      issuer: async (_: unknown, args: { address: string }) => {
        const issuer = await db.issuer.findUnique({
          where: { address: args.address },
        });
        return issuer
          ? {
              ...issuer,
              registeredAt: issuer.registeredAt.toISOString(),
              updatedAt: issuer.updatedAt.toISOString(),
            }
          : null;
      },

      issuers: async (
        _: unknown,
        args: { start?: number; limit?: number }
      ) => {
        const start = args.start ?? 0;
        const limit = args.limit ?? 50;

        const [issuers, total] = await Promise.all([
          db.issuer.findMany({
            skip: start,
            take: limit,
            orderBy: { registeredAt: "desc" },
          }),
          db.issuer.count(),
        ]);

        return {
          items: issuers.map((i) => ({
            ...i,
            registeredAt: i.registeredAt.toISOString(),
            updatedAt: i.updatedAt.toISOString(),
          })),
          total,
        };
      },

      // #775 + #777: issuerStats includes rateLimit; cached in Redis
      issuerStats: async (_: unknown, args: { issuer: string }) => {
        const cacheKey = `issuerStats:${args.issuer}`;
        const cached = await cacheGet(redis, cacheKey);
        if (cached) return cached;

        const [rows, issuerRow] = await Promise.all([
          db.attestation.findMany({
            where: { issuer: args.issuer },
            select: { isRevoked: true, claimType: true },
          }),
          db.issuer.findUnique({ where: { address: args.issuer } }),
        ]);

        const claimTypes = [...new Set(rows.map((r) => r.claimType))];
        const revoked = rows.filter((r) => r.isRevoked).length;

        const result = {
          issuer: args.issuer,
          total: rows.length,
          active: rows.length - revoked,
          revoked,
          claimTypes,
          rateLimit: issuerRow?.rateLimit ?? null,
        };

        await cacheSet(redis, cacheKey, result);
        return result;
      },

      proposal: async (_: unknown, args: { id: string }) => {
        const proposal = await db.multisigProposal.findUnique({
          where: { id: args.id },
        });
        return proposal ? mapProposal(proposal) : null;
      },

      proposals: async (
        _: unknown,
        args: { subject?: string; finalized?: boolean }
      ) => {
        const where: Record<string, unknown> = {};
        if (args.subject) where.subject = args.subject;
        if (args.finalized !== undefined) where.finalized = args.finalized;

        const rows = await db.multisigProposal.findMany({
          where,
          orderBy: { createdAt: "desc" },
        });
        return rows.map(mapProposal);
      },

      // #774: audit log query
      auditLog: async (_: unknown, args: { attestationId: string }) => {
        const rows = await db.auditEntry.findMany({
          where: { attestationId: args.attestationId },
          orderBy: { timestamp: "asc" },
        });
        return rows.map(mapAuditEntry);
      },
    },

    Subscription: {
      onAttestationCreated: {
        subscribe: (_: unknown, args: { subject?: string }) => {
          const iter = pubsub.asyncIterableIterator<{
            onAttestationCreated: ReturnType<typeof mapAttestation>;
          }>(ATTESTATION_CREATED);

          if (!args.subject) return iter;

          const subject = args.subject;
          return {
            [Symbol.asyncIterator]() {
              return this;
            },
            async next(): Promise<IteratorResult<unknown>> {
              while (true) {
                const result = await iter.next();
                if (result.done) return result;
                const att = result.value?.onAttestationCreated;
                if (!att || att.subject === subject) return result;
              }
            },
            async return() {
              return iter.return?.() ?? { done: true as const, value: undefined };
            },
          };
        },
        resolve: (payload: {
          onAttestationCreated: ReturnType<typeof mapAttestation>;
        }) => payload.onAttestationCreated,
      },

      onAttestationRevoked: {
        subscribe: (_: unknown, args: { issuer?: string }) => {
          const iter = pubsub.asyncIterableIterator<{
            onAttestationRevoked: { id: string; issuer: string; revokedAt: string };
          }>(ATTESTATION_REVOKED);

          if (!args.issuer) return iter;

          const issuer = args.issuer;
          return {
            [Symbol.asyncIterator]() {
              return this;
            },
            async next(): Promise<IteratorResult<unknown>> {
              while (true) {
                const result = await iter.next();
                if (result.done) return result;
                const data = result.value?.onAttestationRevoked;
                if (!data || data.issuer === issuer) return result;
              }
            },
            async return() {
              return iter.return?.() ?? { done: true as const, value: undefined };
            },
          };
        },
        resolve: (payload: {
          onAttestationRevoked: { id: string; issuer: string; revokedAt: string };
        }) => payload.onAttestationRevoked,
      },

      onIssuerRegistered: {
        subscribe: () =>
          pubsub.asyncIterableIterator<{
            onIssuerRegistered: { issuer: string; registeredAt: string };
          }>(ISSUER_REGISTERED),
        resolve: (payload: {
          onIssuerRegistered: { issuer: string; registeredAt: string };
        }) => payload.onIssuerRegistered,
      },
    },
  };
}
