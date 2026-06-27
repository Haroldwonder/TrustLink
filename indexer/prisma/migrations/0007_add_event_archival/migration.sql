-- CreateTable for ArchivalConfig (singleton)
CREATE TABLE "ArchivalConfig" (
    "id" INTEGER NOT NULL DEFAULT 1,
    "retentionDaysRaw" INTEGER NOT NULL DEFAULT 30,
    "archiveEnabled" BOOLEAN NOT NULL DEFAULT false,
    "archivePath" TEXT NOT NULL DEFAULT 's3://trustlink-events-archive',
    "lastArchivedAt" TIMESTAMP(3),
    "lastArchivedAttemptAt" TIMESTAMP(3),
    "lastArchivalError" TEXT,
    "compressionEnabled" BOOLEAN NOT NULL DEFAULT true,

    CONSTRAINT "ArchivalConfig_pkey" PRIMARY KEY ("id")
);

-- CreateTable for ArchivedEventBatch (audit trail of archived batches)
CREATE TABLE "ArchivedEventBatch" (
    "id" TEXT NOT NULL,
    "fromLedger" INTEGER NOT NULL,
    "toLedger" INTEGER NOT NULL,
    "recordCount" INTEGER NOT NULL,
    "archivePath" TEXT NOT NULL,
    "compressedSize" BIGINT NOT NULL,
    "uncompressedSize" BIGINT NOT NULL,
    "archivedAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "checksum" TEXT NOT NULL,

    CONSTRAINT "ArchivedEventBatch_pkey" PRIMARY KEY ("id")
);

-- CreateTable for RawEvent (transient event storage)
CREATE TABLE "RawEvent" (
    "id" TEXT NOT NULL,
    "ledger" INTEGER NOT NULL,
    "eventType" TEXT NOT NULL,
    "contractId" TEXT NOT NULL,
    "topic0" TEXT,
    "topic1" TEXT,
    "topic2" TEXT,
    "topic3" TEXT,
    "dataJson" TEXT NOT NULL,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "RawEvent_pkey" PRIMARY KEY ("id")
);

-- CreateIndex
CREATE INDEX "ArchivedEventBatch_archivedAt_idx" ON "ArchivedEventBatch"("archivedAt");

-- CreateIndex
CREATE INDEX "ArchivedEventBatch_fromLedger_toLedger_idx" ON "ArchivedEventBatch"("fromLedger", "toLedger");

-- CreateIndex
CREATE INDEX "RawEvent_ledger_idx" ON "RawEvent"("ledger");

-- CreateIndex
CREATE INDEX "RawEvent_eventType_idx" ON "RawEvent"("eventType");

-- CreateIndex
CREATE INDEX "RawEvent_createdAt_idx" ON "RawEvent"("createdAt");

-- CreateIndex
CREATE INDEX "RawEvent_contractId_idx" ON "RawEvent"("contractId");
