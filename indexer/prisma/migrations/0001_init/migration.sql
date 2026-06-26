-- CreateTable
CREATE TABLE "Attestation" (
    "id" TEXT NOT NULL,
    "issuer" TEXT NOT NULL,
    "subject" TEXT NOT NULL,
    "claimType" TEXT NOT NULL,
    "timestamp" BIGINT NOT NULL,
    "expiration" BIGINT,
    "isRevoked" BOOLEAN NOT NULL DEFAULT false,
    "metadata" TEXT,
    "imported" BOOLEAN NOT NULL DEFAULT false,
    "bridged" BOOLEAN NOT NULL DEFAULT false,
    "sourceChain" TEXT,
    "sourceTx" TEXT,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "Attestation_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "Checkpoint" (
    "id" INTEGER NOT NULL DEFAULT 1,
    "ledger" INTEGER NOT NULL,

    CONSTRAINT "Checkpoint_pkey" PRIMARY KEY ("id")
);

-- CreateIndex
CREATE INDEX "Attestation_subject_idx" ON "Attestation"("subject");

-- CreateIndex
CREATE INDEX "Attestation_issuer_idx" ON "Attestation"("issuer");
