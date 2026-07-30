/**
 * Scheduled reconciliation: sample indexed attestations and diff them against
 * live contract state fetched via Soroban RPC. Emits Prometheus metrics (and
 * logs) when drift is detected so silent indexing bugs surface quickly.
 */

import { PrismaClient } from "@prisma/client";
import {
  Contract,
  TransactionBuilder,
  BASE_FEE,
  Networks,
  nativeToScVal,
  rpc as SorobanRpc,
  scValToNative,
} from "@stellar/stellar-sdk";
import { Counter, Gauge } from "prom-client";

/** Indexed attestation fields used for reconciliation comparisons. */
type IndexedAttestation = {
  id: string;
  issuer: string;
  subject: string;
  claimType: string;
  timestamp: bigint;
  expiration: bigint | null;
  isRevoked: boolean;
  metadata: string | null;
  updatedAt?: Date;
};

/** Attestations compared during reconciliation runs */
export const reconciliationCheckedTotal = new Counter({
  name: "trustlink_reconciliation_checked_total",
  help: "Total attestations compared against live contract state",
});

/** Indexed rows that diverged from on-chain state */
export const reconciliationDriftTotal = new Counter({
  name: "trustlink_reconciliation_drift_total",
  help: "Total attestations where indexed state differs from on-chain state",
  labelNames: ["field"],
});

/** Unix timestamp of the last completed reconciliation run */
export const reconciliationLastRunTimestamp = new Gauge({
  name: "trustlink_reconciliation_last_run_timestamp",
  help: "Unix timestamp of the last completed reconciliation run",
});

export type OnChainAttestation = {
  id: string;
  issuer: string;
  subject: string;
  claim_type: string;
  timestamp: bigint | number | string;
  expiration: bigint | number | string | null;
  revoked: boolean;
  metadata: string | null;
  source_chain?: string | null;
  source_tx?: string | null;
};

export type DriftReport = {
  attestationId: string;
  fields: string[];
  indexed: Record<string, unknown>;
  onChain: Record<string, unknown>;
};

export type ReconciliationResult = {
  checked: number;
  missingOnChain: number;
  drifts: DriftReport[];
};

export type ReconciliationOptions = {
  sampleSize?: number;
  /** When set, only these IDs are checked (used by tests). */
  attestationIds?: string[];
};

const COMPARE_FIELDS: Array<{
  indexed: keyof IndexedAttestation;
  onChain: keyof OnChainAttestation;
  normalize?: (v: unknown) => string;
}> = [
  { indexed: "issuer", onChain: "issuer", normalize: String },
  { indexed: "subject", onChain: "subject", normalize: String },
  { indexed: "claimType", onChain: "claim_type", normalize: String },
  {
    indexed: "timestamp",
    onChain: "timestamp",
    normalize: (v) => String(v),
  },
  {
    indexed: "expiration",
    onChain: "expiration",
    normalize: (v) => (v == null ? "" : String(v)),
  },
  {
    indexed: "isRevoked",
    onChain: "revoked",
    normalize: (v) => String(Boolean(v)),
  },
  {
    indexed: "metadata",
    onChain: "metadata",
    normalize: (v) => (v == null ? "" : String(v)),
  },
];

/**
 * Compare a sample of indexed attestations against live on-chain state.
 */
export async function reconcileAttestations(
  db: PrismaClient,
  fetchOnChain: (id: string) => Promise<OnChainAttestation | null>,
  options: ReconciliationOptions = {},
): Promise<ReconciliationResult> {
  const sampleSize = options.sampleSize ?? 50;

  let rows: IndexedAttestation[];
  if (options.attestationIds?.length) {
    rows = await db.attestation.findMany({
      where: { id: { in: options.attestationIds } },
    });
  } else {
    // Prefer recent rows so drift in active traffic is caught first.
    rows = await db.attestation.findMany({
      take: sampleSize,
      orderBy: { updatedAt: "desc" },
    });
  }

  const drifts: DriftReport[] = [];
  let missingOnChain = 0;

  for (const row of rows) {
    reconciliationCheckedTotal.inc();
    const onChain = await fetchOnChain(row.id);

    if (!onChain) {
      missingOnChain += 1;
      reconciliationDriftTotal.inc({ field: "missing_on_chain" });
      drifts.push({
        attestationId: row.id,
        fields: ["missing_on_chain"],
        indexed: summarizeIndexed(row),
        onChain: {},
      });
      console.error(
        `[RECONCILE] drift: attestation ${row.id} present in indexer but missing on-chain`,
      );
      continue;
    }

    const mismatched: string[] = [];
    for (const field of COMPARE_FIELDS) {
      const norm = field.normalize ?? ((v: unknown) => String(v));
      const left = norm(row[field.indexed]);
      const right = norm(onChain[field.onChain]);
      if (left !== right) {
        mismatched.push(String(field.indexed));
        reconciliationDriftTotal.inc({ field: String(field.indexed) });
      }
    }

    if (mismatched.length > 0) {
      const report: DriftReport = {
        attestationId: row.id,
        fields: mismatched,
        indexed: summarizeIndexed(row),
        onChain: {
          id: onChain.id,
          issuer: onChain.issuer,
          subject: onChain.subject,
          claimType: onChain.claim_type,
          timestamp: String(onChain.timestamp),
          expiration:
            onChain.expiration != null ? String(onChain.expiration) : null,
          isRevoked: onChain.revoked,
          metadata: onChain.metadata,
        },
      };
      drifts.push(report);
      console.error(
        `[RECONCILE] drift on ${row.id}: fields=${mismatched.join(",")} indexed=${JSON.stringify(report.indexed)} onChain=${JSON.stringify(report.onChain)}`,
      );
    }
  }

  reconciliationLastRunTimestamp.set(Math.floor(Date.now() / 1000));

  if (drifts.length > 0) {
    console.error(
      `[RECONCILE] ALERT: ${drifts.length} attestation(s) drifted out of ${rows.length} checked`,
    );
  } else {
    console.log(
      `[RECONCILE] ok: checked=${rows.length} missingOnChain=${missingOnChain}`,
    );
  }

  return { checked: rows.length, missingOnChain, drifts };
}

function summarizeIndexed(row: IndexedAttestation): Record<string, unknown> {
  return {
    id: row.id,
    issuer: row.issuer,
    subject: row.subject,
    claimType: row.claimType,
    timestamp: String(row.timestamp),
    expiration: row.expiration != null ? String(row.expiration) : null,
    isRevoked: row.isRevoked,
    metadata: row.metadata,
  };
}

/**
 * Fetch a single attestation from the live TrustLink contract via RPC simulate.
 */
export function createRpcAttestationFetcher(
  contractId: string,
  rpcUrl: string,
  networkPassphrase: string = Networks.TESTNET,
): (id: string) => Promise<OnChainAttestation | null> {
  const rpc = new SorobanRpc.Server(rpcUrl, { allowHttp: true });
  const dummySource =
    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";

  return async (attestationId: string): Promise<OnChainAttestation | null> => {
    try {
      const contract = new Contract(contractId);
      const account = await rpc.getAccount(dummySource).catch(
        () =>
          ({
            accountId: () => dummySource,
            sequenceNumber: () => "0",
            incrementSequenceNumber: () => {},
          }) as never,
      );

      const tx = new TransactionBuilder(account as never, {
        fee: BASE_FEE,
        networkPassphrase,
      })
        .addOperation(
          contract.call(
            "get_attestation",
            nativeToScVal(attestationId, { type: "string" }),
          ),
        )
        .setTimeout(30)
        .build();

      const result = await rpc.simulateTransaction(tx);
      if (SorobanRpc.Api.isSimulationError(result)) {
        return null;
      }
      const success = result as SorobanRpc.Api.SimulateTransactionSuccessResponse;
      if (!success.result?.retval) return null;

      const native = scValToNative(success.result.retval) as OnChainAttestation;
      return {
        ...native,
        issuer: String(native.issuer),
        subject: String(native.subject),
      };
    } catch (err) {
      console.warn(`[RECONCILE] RPC fetch failed for ${attestationId}:`, err);
      return null;
    }
  };
}

export type ScheduleReconciliationConfig = {
  intervalMinutes?: number;
  sampleSize?: number;
  contractId: string;
  rpcUrl: string;
  networkPassphrase?: string;
  /** Skip the immediate startup run (useful in tests). */
  runImmediately?: boolean;
};

/**
 * Schedule periodic reconciliation. Default: every 60 minutes, sample 50 rows.
 */
export function scheduleReconciliationJob(
  db: PrismaClient,
  config: ScheduleReconciliationConfig,
): ReturnType<typeof setInterval> {
  const intervalMinutes = config.intervalMinutes ?? 60;
  const sampleSize = config.sampleSize ?? 50;
  const fetchOnChain = createRpcAttestationFetcher(
    config.contractId,
    config.rpcUrl,
    config.networkPassphrase,
  );

  const run = () =>
    reconcileAttestations(db, fetchOnChain, { sampleSize }).catch((err) => {
      console.error("[RECONCILE] Scheduled run failed:", err);
    });

  if (config.runImmediately !== false) {
    run();
  }

  const intervalMs = intervalMinutes * 60 * 1000;
  console.log(
    `[RECONCILE] Scheduled reconciliation every ${intervalMinutes}m (sampleSize=${sampleSize})`,
  );

  return setInterval(run, intervalMs);
}
