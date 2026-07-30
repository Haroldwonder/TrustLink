/**
 * ms_sign event handler — kept in its own module so unit tests can exercise
 * the real production logic without importing the full indexer loop.
 */

import type { PrismaClient } from "@prisma/client";

/**
 * Apply a multisig co-sign event to the indexed proposal row.
 *
 * Event shape (from contract):
 *   topics = [ms_sign, signer]
 *   data   = [proposal_id, signatures_so_far, threshold]
 */
export async function handleMsSign(
  db: PrismaClient,
  proposalId: string,
  signatureCount: number,
  signer: string,
): Promise<void> {
  const existing = await db.multisigProposal.findUnique({
    where: { id: proposalId },
    select: { signers: true },
  });
  if (!existing) return; // out-of-order: wait for ms_prop

  const updatedSigners = existing.signers.includes(signer)
    ? existing.signers
    : [...existing.signers, signer];

  await db.multisigProposal.update({
    where: { id: proposalId },
    data: { signatureCount, signers: updatedSigners },
  });
}
