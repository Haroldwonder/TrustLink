-- CreateTable
CREATE TABLE "Template" (
    "id" TEXT NOT NULL,
    "templateId" TEXT NOT NULL,
    "issuer" TEXT NOT NULL,
    "claimType" TEXT NOT NULL,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "Template_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "Delegation" (
    "id" TEXT NOT NULL,
    "delegator" TEXT NOT NULL,
    "delegate" TEXT NOT NULL,
    "claimType" TEXT NOT NULL,
    "expiresAt" BIGINT NOT NULL,
    "revoked" BOOLEAN NOT NULL DEFAULT false,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "Delegation_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "WhitelistEntry" (
    "id" TEXT NOT NULL,
    "issuer" TEXT NOT NULL,
    "subject" TEXT NOT NULL,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "WhitelistEntry_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "CouncilAction" (
    "id" TEXT NOT NULL,
    "actionId" TEXT NOT NULL,
    "proposer" TEXT NOT NULL,
    "approvals" TEXT[],
    "executed" BOOLEAN NOT NULL DEFAULT false,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "CouncilAction_pkey" PRIMARY KEY ("id")
);

-- CreateIndex
CREATE UNIQUE INDEX "Template_templateId_key" ON "Template"("templateId");
CREATE INDEX "Template_issuer_idx" ON "Template"("issuer");

-- CreateIndex
CREATE UNIQUE INDEX "Delegation_delegator_delegate_claimType_key" ON "Delegation"("delegator", "delegate", "claimType");
CREATE INDEX "Delegation_delegator_idx" ON "Delegation"("delegator");
CREATE INDEX "Delegation_delegate_idx" ON "Delegation"("delegate");

-- CreateIndex
CREATE UNIQUE INDEX "WhitelistEntry_issuer_subject_key" ON "WhitelistEntry"("issuer", "subject");
CREATE INDEX "WhitelistEntry_issuer_idx" ON "WhitelistEntry"("issuer");
CREATE INDEX "WhitelistEntry_subject_idx" ON "WhitelistEntry"("subject");

-- CreateIndex
CREATE UNIQUE INDEX "CouncilAction_actionId_key" ON "CouncilAction"("actionId");
CREATE INDEX "CouncilAction_executed_idx" ON "CouncilAction"("executed");
CREATE INDEX "CouncilAction_proposer_idx" ON "CouncilAction"("proposer");
