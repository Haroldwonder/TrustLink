import { useState } from "react";
import { createAttestation, revokeAttestation, getSubjectAttestations, Attestation } from "../contract";
import IssuerDashboard from "./IssuerDashboard";

interface Props { address: string; }

export default function IssuerPanel({ address }: Props) {
  const [tab, setTab] = useState<"dashboard" | "create" | "revoke" | "lookup">("dashboard");
  const [subject, setSubject] = useState("");
  const [claimType, setClaimType] = useState("");
  const [metadata, setMetadata] = useState("");
  const [status, setStatus] = useState<{ type: "success" | "error"; msg: string } | null>(null);
  const [loading, setLoading] = useState(false);

  const [revokeId, setRevokeId] = useState("");
  const [revokeReason, setRevokeReason] = useState("");

  const [lookupAddr, setLookupAddr] = useState("");
  const [attestations, setAttestations] = useState<Attestation[]>([]);

  async function handleCreate() {
    if (!subject || !claimType) return;
    setLoading(true);
    setStatus(null);
    try {
      await createAttestation(address, subject.trim(), claimType.trim(), null, metadata || null);
      setStatus({ type: "success", msg: "Attestation created." });
      setSubject(""); setClaimType(""); setMetadata("");
    } catch (e: unknown) {
      setStatus({ type: "error", msg: (e as Error).message });
    } finally {
      setLoading(false);
    }
  }

  async function handleRevoke() {
    if (!revokeId) return;
    setLoading(true);
    setStatus(null);
    try {
      await revokeAttestation(address, revokeId.trim(), revokeReason || null);
      setStatus({ type: "success", msg: "Attestation revoked." });
      setRevokeId(""); setRevokeReason("");
    } catch (e: unknown) {
      setStatus({ type: "error", msg: (e as Error).message });
    } finally {
      setLoading(false);
    }
  }

  async function handleLookup() {
    if (!lookupAddr) return;
    setLoading(true);
    try {
      const list = await getSubjectAttestations(lookupAddr.trim());
      setAttestations(list.filter((a) => a.issuer === address));
    } catch (e: unknown) {
      setStatus({ type: "error", msg: (e as Error).message });
    } finally {
      setLoading(false);
    }
  }

  if (tab === "dashboard") {
    return (
      <div>
        <div style={{ display: "flex", gap: "0.5rem", marginBottom: "1rem", borderBottom: "1px solid #2d3148", paddingBottom: "0.5rem" }}>
          <button
            className={`tab ${tab === "dashboard" ? "active" : ""}`}
            onClick={() => setTab("dashboard")}
            style={{ flex: 1, textAlign: "center", padding: "0.5rem" }}
          >
            Dashboard
          </button>
          <button
            className={`tab ${tab === "create" ? "active" : ""}`}
            onClick={() => setTab("create")}
            style={{ flex: 1, textAlign: "center", padding: "0.5rem" }}
          >
            Create
          </button>
          <button
            className={`tab ${tab === "revoke" ? "active" : ""}`}
            onClick={() => setTab("revoke")}
            style={{ flex: 1, textAlign: "center", padding: "0.5rem" }}
          >
            Revoke
          </button>
          <button
            className={`tab ${tab === "lookup" ? "active" : ""}`}
            onClick={() => setTab("lookup")}
            style={{ flex: 1, textAlign: "center", padding: "0.5rem" }}
          >
            Lookup
          </button>
        </div>
        <IssuerDashboard address={address} />
      </div>
    );
  }

  return (
    <div className="panel">
      <h2>Issuer Panel</h2>
      <div style={{ display: "flex", gap: "0.5rem", marginBottom: "1rem", borderBottom: "1px solid #2d3148", paddingBottom: "0.5rem" }}>
        <button
          className={`tab ${tab === "dashboard" ? "active" : ""}`}
          onClick={() => setTab("dashboard")}
          style={{ flex: 1, textAlign: "center", padding: "0.5rem" }}
        >
          Dashboard
        </button>
        <button
          className={`tab ${tab === "create" ? "active" : ""}`}
          onClick={() => setTab("create")}
          style={{ flex: 1, textAlign: "center", padding: "0.5rem" }}
        >
          Create
        </button>
        <button
          className={`tab ${tab === "revoke" ? "active" : ""}`}
          onClick={() => setTab("revoke")}
          style={{ flex: 1, textAlign: "center", padding: "0.5rem" }}
        >
          Revoke
        </button>
        <button
          className={`tab ${tab === "lookup" ? "active" : ""}`}
          onClick={() => setTab("lookup")}
          style={{ flex: 1, textAlign: "center", padding: "0.5rem" }}
        >
          Lookup
        </button>
      </div>

      {status && <div className={`alert alert-${status.type}`}>{status.msg}</div>}

      {tab === "create" && (
        <div className="card">
          <h3>Create Attestation</h3>
          <div className="field"><label>Subject Address</label>
            <input value={subject} onChange={(e) => setSubject(e.target.value)} placeholder="G..." />
          </div>
          <div className="field"><label>Claim Type</label>
            <input value={claimType} onChange={(e) => setClaimType(e.target.value)} placeholder="KYC, AML, accredited-investor…" />
          </div>
          <div className="field"><label>Metadata (optional)</label>
            <input value={metadata} onChange={(e) => setMetadata(e.target.value)} placeholder="optional note" />
          </div>
          <button className="btn btn-primary" disabled={loading || !subject || !claimType} onClick={handleCreate}>
            Create
          </button>
        </div>
      )}

      {tab === "revoke" && (
        <div className="card">
          <h3>Revoke Attestation</h3>
          <div className="field"><label>Attestation ID</label>
            <input value={revokeId} onChange={(e) => setRevokeId(e.target.value)} placeholder="attestation hash" />
          </div>
          <div className="field"><label>Reason (optional)</label>
            <input value={revokeReason} onChange={(e) => setRevokeReason(e.target.value)} placeholder="reason for revocation" />
          </div>
          <button className="btn btn-danger" disabled={loading || !revokeId} onClick={handleRevoke}>
            Revoke
          </button>
        </div>
      )}

      {tab === "lookup" && (
        <div className="card">
          <h3>My Issued Attestations</h3>
          <div style={{ display: "flex", gap: "0.5rem", marginBottom: "0.75rem" }}>
            <input
              className="field"
              style={{ flex: 1, background: "#0f1117", border: "1px solid #2d3148", borderRadius: "0.5rem", padding: "0.5rem 0.75rem", color: "#e2e8f0" }}
              value={lookupAddr}
              onChange={(e) => setLookupAddr(e.target.value)}
              placeholder="Subject address G..."
            />
            <button className="btn btn-outline" disabled={loading || !lookupAddr} onClick={handleLookup}>
              Load
            </button>
          </div>
          {attestations.length === 0
            ? <p className="empty">No attestations found.</p>
            : <AttestationList items={attestations} />}
        </div>
      )}
    </div>
  );
}

function AttestationList({ items }: { items: Attestation[] }) {
  return (
    <div className="att-list">
      {items.map((a) => (
        <div key={a.id} className="att-item">
          <div className="row">
            <span className="claim">{a.claim_type}</span>
            <span className={`badge ${a.revoked ? "badge-revoked" : "badge-valid"}`}>
              {a.revoked ? "Revoked" : "Valid"}
            </span>
          </div>
          <span className="meta">Subject: {a.subject}</span>
          <span className="meta">ID: {a.id}</span>
        </div>
      ))}
    </div>
  );
}
