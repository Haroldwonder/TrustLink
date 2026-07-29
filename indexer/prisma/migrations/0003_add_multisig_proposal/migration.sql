-- CreateTable
CREATE TABLE "MultisigProposal" (
    "id" TEXT NOT NULL,
    "subject" TEXT NOT NULL,
    "proposer" TEXT NOT NULL,
    "claimType" TEXT NOT NULL,
    "threshold" INTEGER NOT NULL,
    "signers" TEXT[],
    "signatureCount" INTEGER NOT NULL DEFAULT 1,
    "finalized" BOOLEAN NOT NULL DEFAULT false,
    "expiresAt" BIGINT NOT NULL,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "MultisigProposal_pkey" PRIMARY KEY ("id")
);

-- CreateIndex
CREATE INDEX "MultisigProposal_subject_idx" ON "MultisigProposal"("subject");

-- CreateIndex
CREATE INDEX "MultisigProposal_proposer_idx" ON "MultisigProposal"("proposer");

-- CreateIndex
CREATE INDEX "MultisigProposal_finalized_idx" ON "MultisigProposal"("finalized");

-- CreateIndex
CREATE INDEX "MultisigProposal_subject_finalized_idx" ON "MultisigProposal"("subject", "finalized");
