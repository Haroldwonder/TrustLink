import { PrismaClient } from "@prisma/client";
import * as fs from "fs";
import * as path from "path";
import * as zlib from "zlib";
import { createReadStream, createWriteStream } from "fs";
import { createHash } from "crypto";
import { pipeline } from "stream/promises";

/**
 * Archival service for TrustLink indexer
 *
 * Responsibility: Archive old raw events to cold storage based on configurable retention window.
 *
 * Archival flow:
 * 1. Query ArchivalConfig to determine retention window and target archive path
 * 2. Find RawEvent rows older than retention window
 * 3. Batch export events to compressed JSON files
 * 4. Write metadata to ArchivedEventBatch table
 * 5. Delete RawEvent rows after successful archival
 *
 * Archive format:
 * - S3/GCS compatible paths: s3://bucket/trustlink-events/YYYY/MM/DD/ledger_X-Y.json.gz
 * - Local filesystem: /archive/trustlink-events/YYYY/MM/DD/ledger_X-Y.json.gz
 */

interface ArchiveOptions {
  batchSize?: number; // Events per archived file (default: 10000)
  maxAge?: number; // Days to retain in hot storage (default: from config)
  dryRun?: boolean; // Simulate archival without deleting
}

export class EventArchivalService {
  constructor(private db: PrismaClient) {}

  /**
   * Run the archival job
   *
   * This is called periodically (e.g., daily or every 6 hours) to:
   * 1. Fetch events older than retention window
   * 2. Export to compressed archive files
   * 3. Clean up archival metadata
   * 4. Update ArchivalConfig with last execution timestamp
   */
  async runArchivalJob(
    options: ArchiveOptions = {},
  ): Promise<ArchivalJobResult> {
    const startTime = Date.now();
    const result: ArchivalJobResult = {
      success: false,
      archivedCount: 0,
      deletedCount: 0,
      batchesCreated: 0,
      duration: 0,
      error: undefined,
    };

    try {
      // 1. Fetch archival configuration
      const config = await this.db.archivalConfig.findUnique({
        where: { id: 1 },
      });
      if (!config || !config.archiveEnabled) {
        result.error = "Archival not enabled or config not found";
        return result;
      }

      const maxAgeDays = options.maxAge ?? config.retentionDaysRaw;
      const cutoffDate = new Date();
      cutoffDate.setDate(cutoffDate.getDate() - maxAgeDays);

      console.log(
        `[ARCHIVAL] Starting job: retention window = ${maxAgeDays} days (before ${cutoffDate.toISOString()})`,
      );

      // 2. Find events older than retention window
      const oldEvents = await this.db.rawEvent.findMany({
        where: { createdAt: { lt: cutoffDate } },
        orderBy: { ledger: "asc" },
        take: options.batchSize ?? 10000,
      });

      if (oldEvents.length === 0) {
        console.log("[ARCHIVAL] No events to archive");
        result.success = true;
        await this.updateArchivalConfig(config, false);
        return result;
      }

      // 3. Group events by ledger range and export
      const ledgerGroups = this.groupEventsByLedger(
        oldEvents,
        options.batchSize ?? 10000,
      );

      for (const [groupIdx, events] of ledgerGroups.entries()) {
        try {
          const minLedger = Math.min(...events.map((e) => e.ledger));
          const maxLedger = Math.max(...events.map((e) => e.ledger));

          const batch = await this.archiveBatch(
            config.archivePath,
            minLedger,
            maxLedger,
            events,
            config.compressionEnabled,
          );

          // 4. Record metadata
          await this.db.archivedEventBatch.create({
            data: {
              fromLedger: batch.fromLedger,
              toLedger: batch.toLedger,
              recordCount: batch.recordCount,
              archivePath: batch.archivePath,
              compressedSize: batch.compressedSize,
              uncompressedSize: batch.uncompressedSize,
              checksum: batch.checksum,
            },
          });

          result.archivedCount += events.length;
          result.batchesCreated += 1;

          console.log(
            `[ARCHIVAL] Batch ${groupIdx + 1}: archived ledgers ${minLedger}–${maxLedger} ` +
              `(${events.length} events, compressed: ${(batch.compressedSize / 1024 / 1024).toFixed(2)} MB)`,
          );
        } catch (err) {
          console.error(
            `[ARCHIVAL] Error archiving batch ${groupIdx + 1}:`,
            err instanceof Error ? err.message : err,
          );
          throw err;
        }
      }

      // 5. Delete archived events (only if not dry-run)
      if (!options.dryRun) {
        const { count: deletedCount } = await this.db.rawEvent.deleteMany({
          where: { createdAt: { lt: cutoffDate } },
        });
        result.deletedCount = deletedCount;
        console.log(
          `[ARCHIVAL] Deleted ${deletedCount} old events from hot storage`,
        );
      }

      result.success = true;
      await this.updateArchivalConfig(config, true);
    } catch (err) {
      result.error = err instanceof Error ? err.message : String(err);
      console.error("[ARCHIVAL] Job failed:", result.error);
      await this.updateArchivalConfig(undefined, false, result.error);
    }

    result.duration = Date.now() - startTime;
    console.log(
      `[ARCHIVAL] Job complete: ${result.archivedCount} events, ` +
        `${result.batchesCreated} batches, ${result.duration}ms`,
    );

    return result;
  }

  /**
   * Restore events from archive (for recovery/auditing)
   *
   * WARNING: This is for recovery only. Restored events are re-inserted into RawEvent.
   * Use sparingly to avoid event duplication.
   */
  async restoreFromArchive(
    fromLedger: number,
    toLedger: number,
  ): Promise<number> {
    const batches = await this.db.archivedEventBatch.findMany({
      where: {
        fromLedger: { gte: fromLedger },
        toLedger: { lte: toLedger },
      },
    });

    let restoredCount = 0;

    for (const batch of batches) {
      try {
        const events = await this.readArchivedBatch(
          batch.archivePath,
          batch.checksum,
        );

        // Upsert to avoid duplicates (idempotent)
        for (const event of events) {
          await this.db.rawEvent.upsert({
            where: { id: event.id },
            update: {},
            create: event,
          });
        }

        restoredCount += events.length;
        console.log(
          `[RESTORE] Restored batch from ${batch.archivePath}: ${events.length} events`,
        );
      } catch (err) {
        console.error(
          `[RESTORE] Error restoring batch ${batch.archivePath}:`,
          err,
        );
        throw err;
      }
    }

    return restoredCount;
  }

  /**
   * Archive a batch of events to cold storage
   *
   * Returns metadata for tracking in ArchivedEventBatch table
   */
  private async archiveBatch(
    archivePath: string,
    fromLedger: number,
    toLedger: number,
    events: Array<{
      id: string;
      ledger: number;
      eventType: string;
      contractId: string;
      topic0: string | null;
      topic1: string | null;
      topic2: string | null;
      topic3: string | null;
      dataJson: string;
      createdAt: Date;
    }>,
    compress: boolean,
  ): Promise<{
    fromLedger: number;
    toLedger: number;
    recordCount: number;
    archivePath: string;
    compressedSize: bigint;
    uncompressedSize: bigint;
    checksum: string;
  }> {
    // Generate archive filename
    const now = new Date();
    const yyyy = now.getFullYear();
    const mm = String(now.getMonth() + 1).padStart(2, "0");
    const dd = String(now.getDate()).padStart(2, "0");
    const filename = `ledger_${fromLedger}-${toLedger}.json${compress ? ".gz" : ""}`;
    const fullPath = path.join(archivePath, yyyy.toString(), mm, dd, filename);

    // Ensure directory exists (local filesystem)
    if (archivePath.startsWith("/") || archivePath.includes(":\\")) {
      const dir = path.dirname(fullPath);
      fs.mkdirSync(dir, { recursive: true });
    }

    // Write events as JSONL (one JSON object per line)
    const tempFile = `/tmp/archive_${Date.now()}_${Math.random().toString(36).slice(2)}.jsonl`;
    const writeStream = createWriteStream(tempFile);
    const hash = createHash("sha256");

    let uncompressedSize = 0;

    for (const event of events) {
      const json = JSON.stringify({
        id: event.id,
        ledger: event.ledger,
        eventType: event.eventType,
        contractId: event.contractId,
        topics: [event.topic0, event.topic1, event.topic2, event.topic3].filter(
          (t) => t != null,
        ),
        data: event.dataJson,
        createdAt: event.createdAt.toISOString(),
      });
      const line = json + "\n";
      writeStream.write(line);
      hash.update(line);
      uncompressedSize += line.length;
    }

    await new Promise<void>((resolve, reject) => {
      writeStream.on("finish", resolve);
      writeStream.on("error", reject);
      writeStream.end();
    });

    const checksum = hash.digest("hex");

    // Compress if enabled
    let finalFile = tempFile;
    if (compress) {
      finalFile = `${tempFile}.gz`;
      await pipeline(
        createReadStream(tempFile),
        zlib.createGzip({ level: 9 }),
        createWriteStream(finalFile),
      );
      fs.unlinkSync(tempFile);
    }

    const compressedSize = BigInt(fs.statSync(finalFile).size);

    // Move to final destination (local filesystem only for now)
    // For S3/GCS, integrate with AWS SDK or google-cloud-storage
    if (!archivePath.startsWith("s3://") && !archivePath.startsWith("gs://")) {
      fs.renameSync(finalFile, fullPath);
    } else {
      // TODO: Implement S3/GCS upload
      console.warn(
        `[ARCHIVAL] S3/GCS upload not yet implemented; archived to temp: ${finalFile}`,
      );
    }

    return {
      fromLedger,
      toLedger,
      recordCount: events.length,
      archivePath: fullPath,
      compressedSize,
      uncompressedSize: BigInt(uncompressedSize),
      checksum,
    };
  }

  /**
   * Read a batch of events from an archived file
   *
   * Verifies checksum before returning
   */
  private async readArchivedBatch(
    archivePath: string,
    expectedChecksum: string,
  ): Promise<
    Array<{
      id: string;
      ledger: number;
      eventType: string;
      contractId: string;
      topic0: string | null;
      topic1: string | null;
      topic2: string | null;
      topic3: string | null;
      dataJson: string;
      createdAt: Date;
    }>
  > {
    // TODO: Implement S3/GCS download
    if (archivePath.startsWith("s3://") || archivePath.startsWith("gs://")) {
      throw new Error("S3/GCS restore not yet implemented");
    }

    // Read local file
    let content = fs.readFileSync(archivePath);

    // Decompress if gzipped
    if (archivePath.endsWith(".gz")) {
      content = zlib.gunzipSync(content);
    }

    // Verify checksum
    const hash = createHash("sha256").update(content).digest("hex");
    if (hash !== expectedChecksum) {
      throw new Error(
        `Checksum mismatch: expected ${expectedChecksum}, got ${hash} (file may be corrupted)`,
      );
    }

    // Parse JSONL
    const lines = content
      .toString("utf-8")
      .split("\n")
      .filter((l) => l.length > 0);
    return lines.map((line) => {
      const parsed = JSON.parse(line);
      return {
        id: parsed.id,
        ledger: parsed.ledger,
        eventType: parsed.eventType,
        contractId: parsed.contractId,
        topic0: parsed.topics[0] ?? null,
        topic1: parsed.topics[1] ?? null,
        topic2: parsed.topics[2] ?? null,
        topic3: parsed.topics[3] ?? null,
        dataJson: parsed.data,
        createdAt: new Date(parsed.createdAt),
      };
    });
  }

  /**
   * Group events by ledger range for archival
   */
  private groupEventsByLedger(
    events: Array<{ ledger: number; [key: string]: unknown }>,
    batchSize: number,
  ): Array<Array<(typeof events)[0]>> {
    const groups: Array<Array<(typeof events)[0]>> = [];
    let currentGroup: Array<(typeof events)[0]> = [];

    for (const event of events) {
      currentGroup.push(event);
      if (currentGroup.length >= batchSize) {
        groups.push(currentGroup);
        currentGroup = [];
      }
    }

    if (currentGroup.length > 0) {
      groups.push(currentGroup);
    }

    return groups;
  }

  /**
   * Update archival configuration with job status
   */
  private async updateArchivalConfig(
    config: { id: number } | undefined,
    success: boolean,
    error?: string,
  ): Promise<void> {
    const now = new Date();
    const update: Record<string, unknown> = {
      lastArchivedAttemptAt: now,
    };

    if (success) {
      update.lastArchivedAt = now;
      update.lastArchivalError = null;
    } else if (error) {
      update.lastArchivalError = error;
    }

    await this.db.archivalConfig.upsert({
      where: { id: 1 },
      update,
      create: {
        id: 1,
        ...update,
      },
    });
  }
}

export interface ArchivalJobResult {
  success: boolean;
  archivedCount: number;
  deletedCount: number;
  batchesCreated: number;
  duration: number;
  error?: string;
}

/**
 * Initialize archival job scheduler
 *
 * Runs archival every N hours (default: 6)
 */
export function scheduleArchivalJob(
  db: PrismaClient,
  intervalHours: number = 6,
): NodeJS.Timer {
  const service = new EventArchivalService(db);

  // Run on startup
  service.runArchivalJob().catch((err) => {
    console.error("[ARCHIVAL] Initial run failed:", err);
  });

  // Schedule periodic runs
  const intervalMs = intervalHours * 60 * 60 * 1000;
  console.log(`[ARCHIVAL] Scheduled archival job every ${intervalHours} hours`);

  return setInterval(() => {
    service.runArchivalJob().catch((err) => {
      console.error("[ARCHIVAL] Scheduled run failed:", err);
    });
  }, intervalMs);
}
