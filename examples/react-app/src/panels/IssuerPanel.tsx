import { useState } from "react";
import { createAttestation, revokeAttestation, getSubjectAttestations, Attestation } from "../contract";
import { SkeletonAttestationList } from "../SkeletonList";
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
  const [lookupLoading, setLookupLoading] = useState(false);

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
    setLookupLoading(true);
    try {
      const list = await getSubjectAttestations(lookupAddr.trim());
      setAttestations(list.filter((a) => a.issuer === address));
    } catch (e: unknown) {
      setStatus({ type: "error", msg: (e as Error).message });
    } finally {
      setLoading(false);
      setLookupLoading(false);
    }
  }

  const TabNav = () => (
    <nav
      role="tablist"
      aria-label="Issuer panel tabs"
      style={{ display: "flex", gap: "0.5rem", marginBottom: "1rem", borderBottom: "1px solid #2d3148", paddingBottom: "0.5rem" }}
    >
      {(["dashboard", "create", "revoke", "lookup"] as const).map((t) => (
        <button
          key={t}
          role="tab"
          aria-selected={tab === t}
          className={`tab ${tab === t ? "active" : ""}`}
          onClick={() => setTab(t)}
          style={{ flex: 1, textAlign: "center", padding: "0.5rem", textTransform: "capitalize" }}
        >
          {t.charAt(0).toUpperCase() + t.slice(1)}
        </button>
      ))}
    </nav>
  );

  if (tab === "dashboard") {
    return (
      <div>
        <TabNav />
        <IssuerDashboard address={address} />
      </div>
    );
  }

  return (
    <div className="panel">
      <h2>Issuer Panel</h2>
      <TabNav />

      {status && (
        <div role="alert" className={`alert alert-${status.type}`}>
          {status.msg}
        </div>
      )}

      {tab === "create" && (
        <div className="card">
          <h3>Create Attestation</h3>
          <div className="field">
            <label htmlFor="issuer-subject">Subject Address</label>
            <input
              id="issuer-subject"
              value={subject}
              onChange={(e) => setSubject(e.target.value)}
              placeholder="G..."
            />
          </div>
          <div className="field">
            <label htmlFor="issuer-claim-type">Claim Type</label>
            <input
              id="issuer-claim-type"
              value={claimType}
              onChange={(e) => setClaimType(e.target.value)}
              placeholder="KYC_PASSED, AML_CLEARED…"
            />
          </div>
          <div className="field">
            <label htmlFor="issuer-metadata">Metadata (optional)</label>
            <input
              id="issuer-metadata"
              value={metadata}
              onChange={(e) => setMetadata(e.target.value)}
              placeholder="optional note"
            />
          </div>
          <button
            className="btn btn-primary"
            disabled={loading || !subject || !claimType}
            onClick={handleCreate}
            aria-disabled={loading || !subject || !claimType}
          >
            Create
          </button>
        </div>
      )}

      {tab === "revoke" && (
        <div className="card">
          <h3>Revoke Attestation</h3>
          <div className="field">
            <label htmlFor="revoke-id">Attestation ID</label>
            <input
              id="revoke-id"
              value={revokeId}
              onChange={(e) => setRevokeId(e.target.value)}
              placeholder="attestation hash"
            />
          </div>
          <div className="field">
            <label htmlFor="revoke-reason">Reason (optional)</label>
            <input
              id="revoke-reason"
              value={revokeReason}
              onChange={(e) => setRevokeReason(e.target.value)}
              placeholder="reason for revocation"
            />
          </div>
          <button
            className="btn btn-danger"
            disabled={loading || !revokeId}
            onClick={handleRevoke}
            aria-disabled={loading || !revokeId}
          >
            Revoke
          </button>
        </div>
      )}

      {tab === "lookup" && (
        <div className="card">
          <h3>My Issued Attestations</h3>
          <div style={{ display: "flex", gap: "0.5rem", marginBottom: "0.75rem" }}>
            <label htmlFor="issuer-lookup-addr" className="visually-hidden">
              Subject address to look up
            </label>
            <input
              id="issuer-lookup-addr"
              className="field"
              style={{ flex: 1, background: "#0f1117", border: "1px solid #2d3148", borderRadius: "0.5rem", padding: "0.5rem 0.75rem", color: "#e2e8f0" }}
              value={lookupAddr}
              onChange={(e) => setLookupAddr(e.target.value)}
              placeholder="Subject address G..."
              aria-label="Subject address"
            />
            <button
              className="btn btn-outline"
              disabled={loading || !lookupAddr}
              onClick={handleLookup}
              aria-disabled={loading || !lookupAddr}
            >
              Load
            </button>
          </div>
          {lookupLoading
            ? <SkeletonAttestationList />
            : attestations.length === 0
              ? <p className="empty">No attestations found.</p>
              : <AttestationList items={attestations} />}
        </div>
      )}
    </div>
  );
}

function AttestationList({ items }: { items: Attestation[] }) {
  return (
    <ul className="att-list" aria-label="Attestation list">
      {items.map((a) => (
        <li key={a.id} className="att-item">
          <div className="row">
            <span className="claim">{a.claim_type}</span>
            <span className={`badge ${a.revoked ? "badge-revoked" : "badge-valid"}`}>
              {a.revoked ? "Revoked" : "Valid"}
            </span>
          </div>
          <span className="meta">Subject: {a.subject}</span>
          <span className="meta">ID: {a.id}</span>
        </li>
      ))}
    </ul>
  );
}
