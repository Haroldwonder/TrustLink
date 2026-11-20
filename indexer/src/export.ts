import type { PrismaClient } from "@prisma/client";
import { Readable } from "stream";

export const EXPORT_BATCH_SIZE = 500;
export const DEFAULT_EXPORT_MAX_ROWS = 10_000;
export const DEFAULT_EXPORT_MAX_RANGE_SECONDS = 30 * 24 * 60 * 60; // 30 days

export function getExportMaxRows(): number {
  const raw = process.env.EXPORT_MAX_ROWS;
  if (!raw) {
    return DEFAULT_EXPORT_MAX_ROWS;
  }
  const parsed = Number.parseInt(raw, 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : DEFAULT_EXPORT_MAX_ROWS;
}

export function getExportMaxRangeSeconds(): bigint {
  const raw = process.env.EXPORT_MAX_RANGE_SECONDS;
  if (!raw) {
    return BigInt(DEFAULT_EXPORT_MAX_RANGE_SECONDS);
  }
  try {
    const parsed = BigInt(raw);
    return parsed > 0n ? parsed : BigInt(DEFAULT_EXPORT_MAX_RANGE_SECONDS);
  } catch {
    return BigInt(DEFAULT_EXPORT_MAX_RANGE_SECONDS);
  }
}

export function validateExportDateRange(
  fromTimestamp?: bigint,
  toTimestamp?: bigint,
  maxRangeSeconds = getExportMaxRangeSeconds(),
): string | null {
  if (fromTimestamp !== undefined && toTimestamp !== undefined) {
    if (fromTimestamp > toTimestamp) {
      return "'from' timestamp must be less than or equal to 'to' timestamp";
    }
    if (toTimestamp - fromTimestamp > maxRangeSeconds) {
      return `Date range exceeds maximum allowed window of ${maxRangeSeconds} seconds`;
    }
  }
  return null;
}

export type ExportFormat = "csv" | "json";

export interface AttestationExportFilters {
  issuer?: string;
  subject?: string;
  claimType?: string;
  fromTimestamp?: bigint;
  toTimestamp?: bigint;
}

export interface AttestationExportWhere {
  issuer?: string;
  subject?: string;
  claimType?: string;
  timestamp?: {
    gte?: bigint;
    lte?: bigint;
  };
}

export interface ExportAttestationRow {
  id: string;
  issuer: string;
  subject: string;
  claimType: string;
  timestamp: bigint;
  expiration: bigint | null;
  isRevoked: boolean;
  revocationReason: string | null;
  metadata: string | null;
  imported: boolean;
  bridged: boolean;
  sourceChain: string | null;
  sourceTx: string | null;
  createdAt: Date;
  updatedAt: Date;
}

export function parseExportFormat(value: string | undefined): ExportFormat | null {
  if (!value || value === "json") {
    return "json";
  }
  if (value === "csv") {
    return "csv";
  }
  return null;
}

export function buildAttestationExportWhere(
  filters: AttestationExportFilters,
): AttestationExportWhere {
  const where: AttestationExportWhere = {};
  if (filters.issuer) {
    where.issuer = filters.issuer;
  }
  if (filters.subject) {
    where.subject = filters.subject;
  }
  if (filters.claimType) {
    where.claimType = filters.claimType;
  }
  if (filters.fromTimestamp !== undefined || filters.toTimestamp !== undefined) {
    where.timestamp = {};
    if (filters.fromTimestamp !== undefined) {
      where.timestamp.gte = filters.fromTimestamp;
    }
    if (filters.toTimestamp !== undefined) {
      where.timestamp.lte = filters.toTimestamp;
    }
  }
  return where;
}

export function parseTimestampQuery(value: string | undefined): bigint | undefined {
  if (!value) {
    return undefined;
  }
  try {
    return BigInt(value);
  } catch {
    return undefined;
  }
}

const CSV_COLUMNS = [
  "id",
  "issuer",
  "subject",
  "claimType",
  "timestamp",
  "expiration",
  "isRevoked",
  "revocationReason",
  "metadata",
  "imported",
  "bridged",
  "sourceChain",
  "sourceTx",
  "createdAt",
  "updatedAt",
] as const;

function escapeCsvField(value: unknown): string {
  if (value === null || value === undefined) {
    return "";
  }
  const text =
    value instanceof Date
      ? value.toISOString()
      : typeof value === "bigint"
        ? value.toString()
        : String(value);
  if (/[",\n\r]/.test(text)) {
    return `"${text.replace(/"/g, '""')}"`;
  }
  return text;
}

export function attestationToCsvRow(
  row: Record<(typeof CSV_COLUMNS)[number], unknown>,
): string {
  return `${CSV_COLUMNS.map((column) => escapeCsvField(row[column])).join(",")}\n`;
}

export const CSV_HEADER = `${CSV_COLUMNS.join(",")}\n`;

export async function* iterateAttestationsForExport(
  db: PrismaClient,
  where: AttestationExportWhere,
  batchSize = EXPORT_BATCH_SIZE,
  maxRows = getExportMaxRows(),
): AsyncGenerator<ExportAttestationRow> {
  let cursor: string | undefined;
  let yielded = 0;

  while (yielded < maxRows) {
    const take = Math.min(batchSize, maxRows - yielded);
    const batch = await db.attestation.findMany({
      where,
      orderBy: { id: "asc" },
      take,
      ...(cursor ? { skip: 1, cursor: { id: cursor } } : {}),
    });

    if (batch.length === 0) {
      break;
    }

    for (const row of batch) {
      yield row as ExportAttestationRow;
      yielded += 1;
      if (yielded >= maxRows) {
        break;
      }
    }

    cursor = batch[batch.length - 1]?.id;
    if (batch.length < take) {
      break;
    }
  }
}

export function createAttestationExportStream(
  db: PrismaClient,
  where: AttestationExportWhere,
  format: ExportFormat,
  maxRows = getExportMaxRows(),
): Readable {
  const iterator = iterateAttestationsForExport(db, where, EXPORT_BATCH_SIZE, maxRows);

  return Readable.from(
    (async function* () {
      if (format === "csv") {
        yield CSV_HEADER;
        for await (const row of iterator) {
          yield attestationToCsvRow(row as Record<(typeof CSV_COLUMNS)[number], unknown>);
        }
        return;
      }

      yield "[";
      let first = true;
      for await (const row of iterator) {
        const serialized = JSON.stringify({
          ...row,
          timestamp: row.timestamp.toString(),
          expiration: row.expiration?.toString() ?? null,
          createdAt: row.createdAt.toISOString(),
          updatedAt: row.updatedAt.toISOString(),
        });
        yield first ? serialized : `,${serialized}`;
        first = false;
      }
      yield "]";
    })(),
  );
}
