import { PubSub } from "graphql-subscriptions";
import { PrismaClient, Attestation, MultisigProposal } from "@prisma/client";

export const pubsub = new PubSub();
export const ATTESTATION_CREATED = "ATTESTATION_CREATED";
export const ATTESTATION_REVOKED = "ATTESTATION_REVOKED";
export const ISSUER_REGISTERED = "ISSUER_REGISTERED";

type MappedAttestation = Omit<
  Attestation,
  "timestamp" | "expiration" | "createdAt" | "updatedAt"
> & {
  timestamp: string;
  expiration: string | null;
  createdAt: string;
  updatedAt: string;
};

type MappedProposal = Omit<
  MultisigProposal,
  "expiresAt" | "createdAt" | "updatedAt"
> & {
  expiresAt: string;
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

function mapProposal(p: MultisigProposal): MappedProposal {
  return {
    ...p,
    expiresAt: String(p.expiresAt),
    createdAt: p.createdAt.toISOString(),
    updatedAt: p.updatedAt.toISOString(),
  };
}

// ── Cursor-based pagination helper ────────────────────────────────────────────

interface AttestationConnection {
  edges: { node: MappedAttestation; cursor: string }[];
  pageInfo: {
    hasNextPage: boolean;
    hasPreviousPage: boolean;
    startCursor: string | null;
    endCursor: string | null;
  };
  totalCount: number;
}

async function buildAttestationConnection(
  db: PrismaClient,
  where: Record<string, unknown>,
  first?: number,
  after?: string
): Promise<AttestationConnection> {
  const take = Math.min(first ?? 20, 100);
  const cursor = after ? { id: after } : undefined;
  const skip = cursor ? 1 : 0;

  const [rows, totalCount] = await Promise.all([
    db.attestation.findMany({
      where,
      take,
      skip,
      cursor,
      orderBy: { timestamp: "desc" },
    }),
    db.attestation.count({ where }),
  ]);

  const edges = rows.map((row) => ({
    node: mapAttestation(row),
    cursor: row.id,
  }));

  return {
    edges,
    pageInfo: {
      hasNextPage: edges.length === take,
      hasPreviousPage: !!after,
      startCursor: edges[0]?.cursor ?? null,
      endCursor: edges[edges.length - 1]?.cursor ?? null,
    },
    totalCount,
  };
}

// ── Resolvers ─────────────────────────────────────────────────────────────────

export function buildResolvers(db: PrismaClient) {
  return {
    Query: {
      /**
       * Query attestations with optional filters.
       *
       * The `contractId` argument restricts results to a single tracked
       * contract instance. Omitting it returns data across all tracked
       * contracts (useful for platform-wide dashboards).
       */
      attestations: async (
        _: unknown,
        args: {
          contractId?: string;
          subject?: string;
          claimType?: string;
          status?: "ACTIVE" | "REVOKED";
          first?: number;
          after?: string;
        }
      ): Promise<AttestationConnection> => {
        const where: Record<string, unknown> = {};
        if (args.contractId) where.contractId = args.contractId;
        if (args.subject) where.subject = args.subject;
        if (args.claimType) where.claimType = args.claimType;
        if (args.status === "ACTIVE") where.isRevoked = false;
        if (args.status === "REVOKED") where.isRevoked = true;

        return buildAttestationConnection(db, where, args.first, args.after);
      },

      /**
       * Query attestations by issuer with optional contractId scope.
       */
      attestationsByIssuer: async (
        _: unknown,
        args: {
          issuer: string;
          contractId?: string;
          first?: number;
          after?: string;
        }
      ): Promise<AttestationConnection> => {
        const where: Record<string, unknown> = { issuer: args.issuer };
        if (args.contractId) where.contractId = args.contractId;
        return buildAttestationConnection(db, where, args.first, args.after);
      },

      /**
       * Issuer statistics, optionally scoped to a contract instance.
       */
      issuerStats: async (
        _: unknown,
        args: { issuer: string; contractId?: string }
      ) => {
        const where: Record<string, unknown> = { issuer: args.issuer };
        if (args.contractId) where.contractId = args.contractId;

        const rows = await db.attestation.findMany({
          where,
          select: { isRevoked: true, claimType: true },
        });

        const claimTypes = [...new Set(rows.map((r) => r.claimType))];
        const revoked = rows.filter((r) => r.isRevoked).length;

        return {
          issuer: args.issuer,
          contractId: args.contractId ?? null,
          total: rows.length,
          active: rows.length - revoked,
          revoked,
          claimTypes,
        };
      },

      /**
       * Fetch a multi-sig proposal by ID.
       */
      proposal: async (_: unknown, args: { id: string }) => {
        const proposal = await db.multisigProposal.findUnique({
          where: { id: args.id },
        });
        return proposal ? mapProposal(proposal) : null;
      },

      /**
       * List multi-sig proposals, optionally scoped to a contract instance.
       */
      proposals: async (
        _: unknown,
        args: { contractId?: string; subject?: string; finalized?: boolean }
      ) => {
        const where: Record<string, unknown> = {};
        if (args.contractId) where.contractId = args.contractId;
        if (args.subject) where.subject = args.subject;
        if (args.finalized !== undefined) where.finalized = args.finalized;

        const rows = await db.multisigProposal.findMany({
          where,
          orderBy: { createdAt: "desc" },
        });
        return rows.map(mapProposal);
      },

      /**
       * Return the list of contract IDs currently being tracked.
       */
      trackedContracts: async () => {
        const checkpoints = await db.checkpoint.findMany();
        return checkpoints.map((c) => c.contractId);
      },
    },

    Subscription: {
      onAttestationCreated: {
        subscribe: (
          _: unknown,
          args: { subject?: string; contractId?: string }
        ) => {
          const iter = pubsub.asyncIterableIterator<{
            onAttestationCreated: MappedAttestation;
          }>(ATTESTATION_CREATED);

          const { subject, contractId } = args;

          // If no filters, return the raw iterator.
          if (!subject && !contractId) return iter;

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
                if (contractId && att.contractId !== contractId) continue;
                return result;
              }
            },
            async return() {
              return (
                iter.return?.() ?? { done: true as const, value: undefined }
              );
            },
          };
        },
        resolve: (payload: { onAttestationCreated: MappedAttestation }) =>
          payload.onAttestationCreated,
      },

      onAttestationRevoked: {
        subscribe: (
          _: unknown,
          args: { issuer?: string; contractId?: string }
        ) => {
          const iter = pubsub.asyncIterableIterator<{
            onAttestationRevoked: {
              id: string;
              issuer: string;
              contractId: string;
              revokedAt: string;
            };
          }>(ATTESTATION_REVOKED);

          const { issuer, contractId } = args;
          if (!issuer && !contractId) return iter;

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
                if (issuer && data.issuer !== issuer) continue;
                if (contractId && data.contractId !== contractId) continue;
                return result;
              }
            },
            async return() {
              return (
                iter.return?.() ?? { done: true as const, value: undefined }
              );
            },
          };
        },
        resolve: (payload: {
          onAttestationRevoked: {
            id: string;
            issuer: string;
            contractId: string;
            revokedAt: string;
          };
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
