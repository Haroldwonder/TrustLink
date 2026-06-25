import { useState, useEffect } from "react";
import {
  proposeAttestation,
  cosignAttestation,
  getMultiSigProposal,
  getMultisigTtl,
  MultiSigProposal,
  isIssuer,
} from "../contract";

interface Props { address: string; }

export default function MultiSigPanel({ address }: Props) {
  const [tab, setTab] = useState<"propose" | "cosign">("propose");
  const [subject, setSubject] = useState("");
  const [claimType, setClaimType] = useState("");
  const [signers, setSigners] = useState("");
  const [threshold, setThreshold] = useState("2");
  const [status, setStatus] = useState<{ type: "success" | "error"; msg: string } | null>(null);
  const [loading, setLoading] = useState(false);

  const [proposalId, setProposalId] = useState("");
  const [proposal, setProposal] = useState<MultiSigProposal | null>(null);
  const [isUserIssuer, setIsUserIssuer] = useState(false);
  const [multisigTtl, setMultisigTtl] = useState<bigint | null>(null);

  useEffect(() => {
    isIssuer(address).then(setIsUserIssuer);
    getMultisigTtl().then(setMultisigTtl).catch(() => null);
  }, [address]);

  function formatCountdown(expiresAt: bigint): string {
    const nowSec = BigInt(Math.floor(Date.now() / 1000));
    const remaining = expiresAt - nowSec;
    if (remaining <= 0n) return "Expired";
    const h = remaining / 3600n;
    const m = (remaining % 3600n) / 60n;
    const s = remaining % 60n;
    return `${h}h ${m}m ${s}s remaining`;
  }

  async function handlePropose() {
    if (!subject || !claimType || !signers || !threshold) return;
    setLoading(true);
    setStatus(null);
    try {
      const signerList = signers
        .split(",")
        .map((s) => s.trim())
        .filter((s) => s);
      if (signerList.length === 0) throw new Error("At least one signer required");

      const id = await proposeAttestation(
        address,
        subject.trim(),
        claimType.trim(),
        signerList,
        parseInt(threshold)
      );
      setStatus({ type: "success", msg: `Proposal created: ${id}` });
      setSubject("");
      setClaimType("");
      setSigners("");
      setThreshold("2");
    } catch (e: unknown) {
      setStatus({ type: "error", msg: (e as Error).message });
    } finally {
      setLoading(false);
    }
  }

  async function handleLoadProposal() {
    if (!proposalId) return;
    setLoading(true);
    setStatus(null);
    try {
      const p = await getMultiSigProposal(proposalId.trim());
      setProposal(p);
    } catch (e: unknown) {
      setStatus({ type: "error", msg: (e as Error).message });
    } finally {
      setLoading(false);
    }
  }

  async function handleCosign() {
    if (!proposalId) return;
    setLoading(true);
    setStatus(null);
    try {
      await cosignAttestation(address, proposalId.trim());
      setStatus({ type: "success", msg: "Proposal co-signed." });
      await handleLoadProposal();
    } catch (e: unknown) {
      setStatus({ type: "error", msg: (e as Error).message });
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="panel">
      <h2>Multi-Sig Attestations</h2>
      {status && <div className={`alert alert-${status.type}`}>{status.msg}</div>}

      <div style={{ display: "flex", gap: "0.5rem", marginBottom: "1rem", borderBottom: "1px solid #2d3148", paddingBottom: "0.5rem" }}>
        {isUserIssuer && (
          <button
            className={`tab ${tab === "propose" ? "active" : ""}`}
            onClick={() => setTab("propose")}
            style={{ flex: 1, textAlign: "center", padding: "0.5rem" }}
          >
            Propose
          </button>
        )}
        <button
          className={`tab ${tab === "cosign" ? "active" : ""}`}
          onClick={() => setTab("cosign")}
          style={{ flex: 1, textAlign: "center", padding: "0.5rem" }}
        >
          Co-Sign
        </button>
      </div>

      {tab === "propose" && isUserIssuer && (
        <div className="card">
          <h3>Propose Multi-Sig Attestation</h3>
          <p style={{ fontSize: "0.8rem", color: "#94a3b8", marginBottom: "1rem" }}>
            Create a multi-signature proposal requiring multiple issuers to co-sign before the attestation is finalized.
            {multisigTtl != null && (
              <span style={{ display: "block", marginTop: "0.25rem", color: "#7c6af7" }}>
                Proposals expire after {multisigTtl.toString()} ledgers (~{Math.round(Number(multisigTtl) * 5 / 60)} min).
              </span>
            )}
          </p>
          <div className="field">
            <label>Subject Address</label>
            <input
              value={subject}
              onChange={(e) => setSubject(e.target.value)}
              placeholder="G..."
            />
          </div>
          <div className="field">
            <label>Claim Type</label>
            <input
              value={claimType}
              onChange={(e) => setClaimType(e.target.value)}
              placeholder="ACCREDITED_INVESTOR, etc."
            />
          </div>
          <div className="field">
            <label>Required Signers (comma-separated)</label>
            <textarea
              value={signers}
              onChange={(e) => setSigners(e.target.value)}
              placeholder="G..., G..., G..."
              rows={3}
            />
          </div>
          <div className="field">
            <label>Signature Threshold</label>
            <input
              type="number"
              value={threshold}
              onChange={(e) => setThreshold(e.target.value)}
              min="1"
              placeholder="2"
            />
          </div>
          <button
            className="btn btn-primary"
            disabled={loading || !subject || !claimType || !signers || !threshold}
            onClick={handlePropose}
          >
            Propose
          </button>
        </div>
      )}

      {tab === "cosign" && (
        <div>
          <div className="card">
            <h3>Co-Sign Proposal</h3>
            <p style={{ fontSize: "0.8rem", color: "#94a3b8", marginBottom: "1rem" }}>
              Review and co-sign pending multi-signature proposals.
            </p>
            <div style={{ display: "flex", gap: "0.5rem", marginBottom: "1rem" }}>
              <input
                className="field"
                style={{ flex: 1, background: "#0f1117", border: "1px solid #2d3148", borderRadius: "0.5rem", padding: "0.5rem 0.75rem", color: "#e2e8f0" }}
                value={proposalId}
                onChange={(e) => setProposalId(e.target.value)}
                placeholder="Proposal ID"
              />
              <button
                className="btn btn-outline"
                disabled={loading || !proposalId}
                onClick={handleLoadProposal}
              >
                Load
              </button>
            </div>

            {proposal && (
              <div style={{ background: "#0f1117", border: "1px solid #2d3148", borderRadius: "0.5rem", padding: "1rem", marginBottom: "1rem" }}>
                <div style={{ marginBottom: "0.75rem" }}>
                  <span style={{ color: "#94a3b8", fontSize: "0.8rem" }}>Claim Type:</span>
                  <span style={{ marginLeft: "0.5rem", fontWeight: "bold" }}>{proposal.claim_type}</span>
                </div>
                <div style={{ marginBottom: "0.75rem" }}>
                  <span style={{ color: "#94a3b8", fontSize: "0.8rem" }}>Subject:</span>
                  <span style={{ marginLeft: "0.5rem", fontFamily: "monospace", fontSize: "0.8rem" }}>{proposal.subject}</span>
                </div>
                <div style={{ marginBottom: "0.75rem" }}>
                  <span style={{ color: "#94a3b8", fontSize: "0.8rem" }}>Signatures:</span>
                  <span style={{ marginLeft: "0.5rem" }}>
                    {proposal.signers.length} / {proposal.threshold}
                  </span>
                  <div style={{ width: "100%", background: "#1e293b", borderRadius: "0.25rem", height: "0.5rem", marginTop: "0.25rem", overflow: "hidden" }}>
                    <div
                      style={{
                        width: `${(proposal.signers.length / proposal.threshold) * 100}%`,
                        background: proposal.finalized ? "#10b981" : "#7c6af7",
                        height: "100%",
                      }}
                    />
                  </div>
                </div>
                <div style={{ marginBottom: "0.75rem" }}>
                  <span style={{ color: "#94a3b8", fontSize: "0.8rem" }}>Status:</span>
                  <span style={{ marginLeft: "0.5rem" }}>
                    {proposal.finalized ? (
                      <span className="badge badge-valid">Finalized</span>
                    ) : (
                      <span className="badge">Pending</span>
                    )}
                  </span>
                </div>
                {!proposal.finalized && proposal.expires_at != null && (
                  <div style={{ marginBottom: "0.75rem" }}>
                    <span style={{ color: "#94a3b8", fontSize: "0.8rem" }}>Expires:</span>
                    <span style={{ marginLeft: "0.5rem", color: proposal.expires_at - BigInt(Math.floor(Date.now() / 1000)) < 3600n ? "#f87171" : "#e2e8f0", fontSize: "0.85rem" }}>
                      {formatCountdown(proposal.expires_at)}
                    </span>
                  </div>
                )}
                <div style={{ marginBottom: "0.75rem" }}>
                  <span style={{ color: "#94a3b8", fontSize: "0.8rem" }}>Signers:</span>
                  <div style={{ marginTop: "0.5rem", fontSize: "0.8rem", fontFamily: "monospace" }}>
                    {proposal.signers.map((s) => (
                      <div key={s} style={{ color: "#10b981", marginBottom: "0.25rem" }}>
                        ✓ {s}
                      </div>
                    ))}
                    {proposal.required_signers
                      .filter((s) => !proposal.signers.includes(s))
                      .map((s) => (
                        <div key={s} style={{ color: "#64748b", marginBottom: "0.25rem" }}>
                          ○ {s}
                        </div>
                      ))}
                  </div>
                </div>
              </div>
            )}

            {proposal && !proposal.finalized && (
              <button
                className="btn btn-primary"
                disabled={loading || proposal.finalized || proposal.signers.includes(address)}
                onClick={handleCosign}
              >
                {proposal.signers.includes(address) ? "Already Signed" : "Co-Sign"}
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
