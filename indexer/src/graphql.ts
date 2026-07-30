import { RedisPubSub } from "graphql-redis-subscriptions";
import Redis from "ioredis";
import { PrismaClient, Attestation, MultisigProposal, AuditEntry } from "@prisma/client";

const REDIS_URL = process.env.REDIS_URL ?? "redis://localhost:6379";

function createRedisClient() {
  return new Redis(REDIS_URL, {
    maxRetriesPerRequest: null,
    retryStrategy(times: number) {
      return Math.min(times * 50, 2000);
    },
  });
}

export const pubsub = new RedisPubSub({
  publisher: createRedisClient(),
  subscriber: createRedisClient(),
});
export const ATTESTATION_CREATED = "ATTESTATION_CREATED";
export const ATTESTATION_REVOKED = "ATTESTATION_REVOKED";
export const ISSUER_REGISTERED = "ISSUER_REGISTERED";

// Schema version — increment on any change requiring client adaptation.
// See ADR-011 for change-governance rules.
export const SCHEMA_VERSION = "1.1.0";

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
          schemaVersion: SCHEMA_VERSION,
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
      /**
       * Subscribe to attestation creation events.
       * 
       * @param subject - Optional: filter by subject address
       * @param topics - Optional: allowlist of event topics to receive. 
       *                 If provided, only 'created' is relevant for this subscription.
       *                 Useful for consolidated topic-filtered subscriptions.
       */
      onAttestationCreated: {
        subscribe: (_: unknown, args: { subject?: string; issuer?: string; claimType?: string; topics?: string[] }) => {
          const iter = pubsub.asyncIterableIterator<{
            onAttestationCreated: ReturnType<typeof mapAttestation>;
          }>(ATTESTATION_CREATED);

          // If topics filter is provided and "created" is not in the list, return empty
          if (args.topics && !args.topics.includes("created")) {
            return {
              [Symbol.asyncIterator]() {
                return this;
              },
              async next(): Promise<IteratorResult<unknown>> {
                return { done: true, value: undefined };
              },
              async return() {
                return iter.return?.() ?? { done: true as const, value: undefined };
              },
            };
          }

          // If no filters, pass the iterator through unchanged
          if (!args.subject && !args.issuer && !args.claimType) return iter;

          const { subject, issuer, claimType } = args;
          return {
            [Symbol.asyncIterator]() {
              return this;
            },
            async next(): Promise<IteratorResult<unknown>> {
              while (true) {
                const result = await iter.next();
                if (result.done) return result;
                const att = result.value?.onAttestationCreated;
                if (!att) return result;
                if (subject && att.subject !== subject) continue;
                if (issuer && att.issuer !== issuer) continue;
                if (claimType && att.claimType !== claimType) continue;
                return result;
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

      /**
       * Subscribe to attestation revocation events.
       * 
       * @param issuer - Optional: filter by issuer address
       * @param topics - Optional: allowlist of event topics to receive.
       *                 If provided, only 'revoked' is relevant for this subscription.
       *                 Useful for consolidated topic-filtered subscriptions.
       */
      onAttestationRevoked: {
        subscribe: (_: unknown, args: { subject?: string; issuer?: string; claimType?: string; topics?: string[] }) => {
          const iter = pubsub.asyncIterableIterator<{
            onAttestationRevoked: { id: string; issuer: string; subject: string; claimType: string; revokedAt: string };
          }>(ATTESTATION_REVOKED);

          // If topics filter is provided and "revoked" is not in the list, return empty
          if (args.topics && !args.topics.includes("revoked")) {
            return {
              [Symbol.asyncIterator]() {
                return this;
              },
              async next(): Promise<IteratorResult<unknown>> {
                return { done: true, value: undefined };
              },
              async return() {
                return iter.return?.() ?? { done: true as const, value: undefined };
              },
            };
          }

          // If no filters, pass the iterator through unchanged
          if (!args.subject && !args.issuer && !args.claimType) return iter;

          const { subject, issuer, claimType } = args;
          return {
            [Symbol.asyncIterator]() {
              return this;
            },
            async next(): Promise<IteratorResult<unknown>> {
              while (true) {
                const result = await iter.next();
                if (result.done) return result;
                const data = result.value?.onAttestationRevoked;
                if (!data) return result;
                if (subject && data.subject !== subject) continue;
                if (issuer && data.issuer !== issuer) continue;
                if (claimType && data.claimType !== claimType) continue;
                return result;
              }
            },
            async return() {
              return iter.return?.() ?? { done: true as const, value: undefined };
            },
          };
        },
        resolve: (payload: {
          onAttestationRevoked: { id: string; issuer: string; subject: string; claimType: string; revokedAt: string };
        }) => payload.onAttestationRevoked,
      },

      /**
       * Subscribe to issuer registration events.
       * 
       * @param topics - Optional: allowlist of event topics to receive.
       *                 If provided, only 'iss_reg' is relevant for this subscription.
       *                 Useful for consolidated topic-filtered subscriptions.
       */
      onIssuerRegistered: {
        subscribe: (_: unknown, args: { topics?: string[] }) => {
          const iter = pubsub.asyncIterableIterator<{
            onIssuerRegistered: { issuer: string; registeredAt: string };
          }>(ISSUER_REGISTERED);

          // If topics filter is provided and "iss_reg" is not in the list, return empty
          if (args.topics && !args.topics.includes("iss_reg")) {
            return {
              [Symbol.asyncIterator]() {
                return this;
              },
              async next(): Promise<IteratorResult<unknown>> {
                return { done: true, value: undefined };
              },
              async return() {
                return iter.return?.() ?? { done: true as const, value: undefined };
              },
            };
          }

          return iter;
        },
        resolve: (payload: {
          onIssuerRegistered: { issuer: string; registeredAt: string };
        }) => payload.onIssuerRegistered,
      },
    },
  };
}
