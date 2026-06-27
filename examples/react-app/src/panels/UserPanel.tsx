import { useState, useEffect } from "react";
import { getSubjectAttestations, getAuditLog, Attestation, AuditEntry } from "../contract";
import { SkeletonAttestationList } from "../SkeletonList";

interface Props { address: string; }

function AttestationTimeline({ attestationId }: { attestationId: string }) {
  const [log, setLog] = useState<AuditEntry[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getAuditLog(attestationId)
      .then(setLog)
      .catch((e: unknown) => setError((e as Error).message))
      .finally(() => setLoading(false));
  }, [attestationId]);

  if (loading) return <p className="empty" style={{ fontSize: "0.75rem" }}>Loading history…</p>;
  if (error) return <p className="empty" style={{ fontSize: "0.75rem", color: "#ef4444" }}>Failed to load history.</p>;
  if (!log || log.length === 0) return <p className="empty" style={{ fontSize: "0.75rem" }}>No history available.</p>;

  return (
    <div style={{ marginTop: "0.5rem", borderLeft: "2px solid #2d3148", paddingLeft: "0.75rem" }}>
      {log.map((entry, i) => (
        <div key={i} style={{ marginBottom: "0.5rem", position: "relative" }}>
          <div style={{ position: "absolute", left: "-1rem", top: "0.3rem", width: "0.5rem", height: "0.5rem", borderRadius: "50%", background: "#7c6af7" }} />
          <div style={{ fontSize: "0.75rem", fontWeight: 600, color: "#e2e8f0" }}>{entry.action}</div>
          <div style={{ fontSize: "0.7rem", color: "#94a3b8", fontFamily: "monospace" }}>
            {entry.actor.slice(0, 6)}…{entry.actor.slice(-4)}
          </div>
          <div style={{ fontSize: "0.7rem", color: "#64748b" }}>
            {new Date(Number(entry.timestamp) * 1000).toLocaleString()}
          </div>
          {entry.details && (
            <div style={{ fontSize: "0.7rem", color: "#94a3b8" }}>{entry.details}</div>
          )}
        </div>
      ))}
    </div>
  );
}

export default function UserPanel({ address }: Props) {
  const [attestations, setAttestations] = useState<Attestation[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [expandedTimeline, setExpandedTimeline] = useState<string | null>(null);

  useEffect(() => {
    setLoading(true);
    getSubjectAttestations(address)
      .then(setAttestations)
      .catch((e: unknown) => setError((e as Error).message))
      .finally(() => setLoading(false));
  }, [address]);

  function statusBadge(a: Attestation) {
    if (a.revoked) return <span className="badge badge-revoked">Revoked</span>;
    if (a.expiration && a.expiration < BigInt(Math.floor(Date.now() / 1000)))
      return <span className="badge badge-expired">Expired</span>;
    return <span className="badge badge-valid">Valid</span>;
  }

  return (
    <div className="panel">
      <h2>My Attestations</h2>
      <p style={{ fontSize: "0.8rem", color: "#64748b", marginBottom: "1rem", fontFamily: "monospace" }}>
        {address}
      </p>

      {error && <div className="alert alert-error">{error}</div>}
      {loading && <SkeletonAttestationList />}

      {!loading && attestations.length === 0 && (
        <p className="empty">No attestations found for your address.</p>
      )}

      <div className="att-list">
        {attestations.map((a) => (
          <div key={a.id} className="att-item">
            <div className="row">
              <span className="claim">{a.claim_type}</span>
              {statusBadge(a)}
            </div>
            <span className="meta">Issuer: {a.issuer}</span>
            {a.metadata && <span className="meta">Note: {a.metadata}</span>}
            {a.expiration && (
              <span className="meta">
                Expires: {new Date(Number(a.expiration) * 1000).toLocaleDateString()}
              </span>
            )}
            <span className="meta">ID: {a.id}</span>
            <button
              className="btn btn-outline"
              style={{ marginTop: "0.4rem", fontSize: "0.75rem", padding: "0.2rem 0.6rem" }}
              onClick={() => setExpandedTimeline(expandedTimeline === a.id ? null : a.id)}
            >
              {expandedTimeline === a.id ? "Hide History" : "View History"}
            </button>
            {expandedTimeline === a.id && <AttestationTimeline attestationId={a.id} />}
          </div>
        ))}
      </div>
    </div>
  );
}
