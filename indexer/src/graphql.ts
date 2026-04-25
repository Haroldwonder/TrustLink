import { PubSub } from "graphql-subscriptions";
import { PrismaClient, Attestation } from "@prisma/client";

export const pubsub = new PubSub();
export const ATTESTATION_CREATED = "ATTESTATION_CREATED";
export const ATTESTATION_REVOKED = "ATTESTATION_REVOKED";
export const ISSUER_REGISTERED = "ISSUER_REGISTERED";

type MappedAttestation = Omit<Attestation, "timestamp" | "expiration" | "createdAt" | "updatedAt"> & {
  timestamp: string;
  expiration: string | null;
  createdAt: string;
  updatedAt: string;
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

export function buildResolvers(db: PrismaClient) {
  return {
    Query: {
      attestations: async (
        _: unknown,
        args: { subject?: string; claimType?: string; status?: "ACTIVE" | "REVOKED" }
      ) => {
        const where: Record<string, unknown> = {};
        if (args.subject) where.subject = args.subject;
        if (args.claimType) where.claimType = args.claimType;
        if (args.status === "ACTIVE") where.isRevoked = false;
        if (args.status === "REVOKED") where.isRevoked = true;

        const rows = await db.attestation.findMany({
          where,
          orderBy: { timestamp: "desc" },
        });
        return rows.map(mapAttestation);
      },

      issuerStats: async (_: unknown, args: { issuer: string }) => {
        const rows = await db.attestation.findMany({
          where: { issuer: args.issuer },
          select: { isRevoked: true, claimType: true },
        });

        const claimTypes = [...new Set(rows.map((r) => r.claimType))];
        const revoked = rows.filter((r) => r.isRevoked).length;

        return {
          issuer: args.issuer,
          total: rows.length,
          active: rows.length - revoked,
          revoked,
          claimTypes,
        };
      },
    },

    Subscription: {
      onAttestationCreated: {
        subscribe: (_: unknown, args: { subject?: string }) => {
          const iter = pubsub.asyncIterableIterator<{
            onAttestationCreated: ReturnType<typeof mapAttestation>;
          }>(ATTESTATION_CREATED);

          if (!args.subject) return iter;

          // Filter by subject when provided
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
