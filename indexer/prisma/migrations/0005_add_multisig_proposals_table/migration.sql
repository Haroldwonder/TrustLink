-- CreateTable
CREATE TABLE "multisig_proposals" (
    "id" TEXT NOT NULL,
    "subject" TEXT NOT NULL,
    "proposer" TEXT NOT NULL,
    "claim_type" TEXT NOT NULL,
    "threshold" INTEGER NOT NULL,
    "signers" TEXT[],
    "signature_count" INTEGER NOT NULL DEFAULT 1,
    "finalized" BOOLEAN NOT NULL DEFAULT false,
    "expires_at" BIGINT NOT NULL,
    "created_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "multisig_proposals_pkey" PRIMARY KEY ("id")
);

-- CreateIndex
CREATE INDEX "multisig_proposals_subject_idx" ON "multisig_proposals"("subject");

-- CreateIndex
CREATE INDEX "multisig_proposals_proposer_idx" ON "multisig_proposals"("proposer");

-- CreateIndex
CREATE INDEX "multisig_proposals_finalized_idx" ON "multisig_proposals"("finalized");

-- CreateIndex
CREATE INDEX "multisig_proposals_subject_finalized_idx" ON "multisig_proposals"("subject", "finalized");
