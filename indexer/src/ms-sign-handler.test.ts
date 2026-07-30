/**
 * Regression tests for the ms_sign handler.
 *
 * The production handler previously referenced an undefined `updatedSigners`
 * variable, which crashed at runtime. These tests process a realistic ms_sign
 * payload through the actual `handleMsSign` function (the same code path
 * `handleEvent` calls) so that class of regression cannot silently reappear.
 *
 * Acceptance: fails against the buggy handler; passes once the fix lands.
 */

import { Keypair, nativeToScVal, Address, xdr, scValToNative } from "@stellar/stellar-sdk";
import { handleMsSign } from "./ms-sign";

function makeMockDb() {
  return {
    multisigProposal: {
      findUnique: jest.fn(),
      update: jest.fn(),
    },
  };
}

/**
 * Build a realistic ms_sign event payload matching the contract's publish shape:
 *   topics = [ms_sign, signer]
 *   data   = [proposal_id, signatures_so_far, threshold]
 * then parse it the same way handleEvent does before calling handleMsSign.
 */
function parseMsSignEvent(proposalId: string, signatureCount: number, threshold: number, signer: string) {
  const topic = [
    nativeToScVal("ms_sign", { type: "symbol" }),
    Address.fromString(signer).toScVal(),
  ];
  const value = xdr.ScVal.scvVec([
    nativeToScVal(proposalId, { type: "string" }),
    nativeToScVal(signatureCount, { type: "u32" }),
    nativeToScVal(threshold, { type: "u32" }),
  ]);

  const topicStr = scValToNative(topic[0]) as string;
  const data = scValToNative(value) as unknown[];
  const parsedSigner = topic[1] ? String(scValToNative(topic[1])) : "";

  return {
    topicStr,
    proposalId: String(data[0]),
    signatureCount: Number(data[1]),
    threshold: Number(data[2]),
    signer: parsedSigner,
  };
}

describe("ms_sign handleMsSign regression (undefined updatedSigners)", () => {
  it("parses a realistic ms_sign payload and updates signer count without throwing", async () => {
    const proposer = Keypair.random().publicKey();
    const signerB = Keypair.random().publicKey();
    const parsed = parseMsSignEvent("prop-abc", 2, 2, signerB);

    expect(parsed.topicStr).toBe("ms_sign");
    expect(parsed.proposalId).toBe("prop-abc");
    expect(parsed.signatureCount).toBe(2);
    expect(parsed.signer).toBe(signerB);

    const db = makeMockDb();
    db.multisigProposal.findUnique.mockResolvedValue({ signers: [proposer] });
    db.multisigProposal.update.mockResolvedValue({});

    await expect(
      handleMsSign(db as never, parsed.proposalId, parsed.signatureCount, parsed.signer),
    ).resolves.toBeUndefined();

    expect(db.multisigProposal.findUnique).toHaveBeenCalledWith({
      where: { id: "prop-abc" },
      select: { signers: true },
    });
    expect(db.multisigProposal.update).toHaveBeenCalledWith({
      where: { id: "prop-abc" },
      data: {
        signatureCount: 2,
        signers: [proposer, signerB],
      },
    });
  });

  it("is idempotent when the same signer co-signs again", async () => {
    const signer = Keypair.random().publicKey();
    const parsed = parseMsSignEvent("prop-abc", 1, 2, signer);
    const db = makeMockDb();
    db.multisigProposal.findUnique.mockResolvedValue({ signers: [signer] });
    db.multisigProposal.update.mockResolvedValue({});

    await expect(
      handleMsSign(db as never, parsed.proposalId, parsed.signatureCount, parsed.signer),
    ).resolves.toBeUndefined();

    expect(db.multisigProposal.update).toHaveBeenCalledWith({
      where: { id: "prop-abc" },
      data: {
        signatureCount: 1,
        signers: [signer],
      },
    });
  });

  it("skips update when proposal is missing (out-of-order ms_sign)", async () => {
    const parsed = parseMsSignEvent(
      "missing-prop",
      2,
      2,
      Keypair.random().publicKey(),
    );
    const db = makeMockDb();
    db.multisigProposal.findUnique.mockResolvedValue(null);

    await expect(
      handleMsSign(db as never, parsed.proposalId, parsed.signatureCount, parsed.signer),
    ).resolves.toBeUndefined();

    expect(db.multisigProposal.update).not.toHaveBeenCalled();
  });
});
