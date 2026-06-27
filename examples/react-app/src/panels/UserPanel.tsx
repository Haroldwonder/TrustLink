import { useState, useEffect, useMemo } from "react";
import { QRCodeSVG } from "qrcode.react";
import { getSubjectAttestations, getAuditLog, Attestation, AuditEntry } from "../contract";
import { SkeletonAttestationList } from "../SkeletonList";

interface Props { address: string; }

type StatusFilter = "all" | "valid" | "revoked" | "expired";

function deriveStatus(a: Attestation): "valid" | "revoked" | "expired" {
  if (a.revoked) return "revoked";
  if (a.expiration && a.expiration < BigInt(Math.floor(Date.now() / 1000))) return "expired";
  return "valid";
}

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
  const [expandedQR, setExpandedQR] = useState<string | null>(null);

  const [filterText, setFilterText] = useState("");
  const [filterClaimType, setFilterClaimType] = useState("");
  const [filterStatus, setFilterStatus] = useState<StatusFilter>("all");

  useEffect(() => {
    setLoading(true);
    getSubjectAttestations(address)
      .then(setAttestations)
      .catch((e: unknown) => setError((e as Error).message))
      .finally(() => setLoading(false));
  }, [address]);

  const claimTypes = useMemo(
    () => Array.from(new Set(attestations.map((a) => a.claim_type))).sort(),
    [attestations]
  );

  const filtered = useMemo(() => {
    const text = filterText.toLowerCase();
    return attestations.filter((a) => {
      if (filterClaimType && a.claim_type !== filterClaimType) return false;
      if (filterStatus !== "all" && deriveStatus(a) !== filterStatus) return false;
      if (text && !a.issuer.toLowerCase().includes(text) && !a.id.toLowerCase().includes(text)) return false;
      return true;
    });
  }, [attestations, filterText, filterClaimType, filterStatus]);

  function statusBadge(a: Attestation) {
    const s = deriveStatus(a);
    if (s === "revoked") return <span className="badge badge-revoked">Revoked</span>;
    if (s === "expired") return <span className="badge badge-expired">Expired</span>;
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

      {!loading && (
        <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap", marginBottom: "1rem", alignItems: "center" }}>
          <input
            style={{ flex: "1 1 160px", background: "#0f1117", border: "1px solid #2d3148", borderRadius: "0.5rem", padding: "0.4rem 0.75rem", color: "#e2e8f0", fontSize: "0.85rem" }}
            placeholder="Search issuer or ID…"
            value={filterText}
            onChange={(e) => setFilterText(e.target.value)}
          />
          <select
            style={{ background: "#0f1117", border: "1px solid #2d3148", borderRadius: "0.5rem", padding: "0.4rem 0.5rem", color: "#e2e8f0", fontSize: "0.85rem" }}
            value={filterClaimType}
            onChange={(e) => setFilterClaimType(e.target.value)}
          >
            <option value="">All claim types</option>
            {claimTypes.map((ct) => <option key={ct} value={ct}>{ct}</option>)}
          </select>
          <select
            style={{ background: "#0f1117", border: "1px solid #2d3148", borderRadius: "0.5rem", padding: "0.4rem 0.5rem", color: "#e2e8f0", fontSize: "0.85rem" }}
            value={filterStatus}
            onChange={(e) => setFilterStatus(e.target.value as StatusFilter)}
          >
            <option value="all">All statuses</option>
            <option value="valid">Valid</option>
            <option value="revoked">Revoked</option>
            <option value="expired">Expired</option>
          </select>
          {(filterText || filterClaimType || filterStatus !== "all") && (
            <button
              className="btn btn-outline"
              style={{ fontSize: "0.8rem", padding: "0.3rem 0.6rem" }}
              onClick={() => { setFilterText(""); setFilterClaimType(""); setFilterStatus("all"); }}
            >
              Clear
            </button>
          )}
          <span style={{ fontSize: "0.8rem", color: "#64748b", whiteSpace: "nowrap" }}>
            {filtered.length} / {attestations.length}
          </span>
        </div>
      )}

      {!loading && attestations.length === 0 && (
        <p className="empty">No attestations found for your address.</p>
      )}

      {!loading && attestations.length > 0 && filtered.length === 0 && (
        <p className="empty">No attestations match the current filters.</p>
      )}

      <div className="att-list">
        {filtered.map((a) => (
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
            <button
              className="btn btn-outline"
              style={{ marginTop: "0.4rem", fontSize: "0.78rem", padding: "0.25rem 0.6rem" }}
              onClick={() => setExpandedQR(expandedQR === a.id ? null : a.id)}
            >
              {expandedQR === a.id ? "Hide QR" : "Show QR"}
            </button>
            {expandedQR === a.id && (
              <div style={{ marginTop: "0.75rem", display: "flex", flexDirection: "column", alignItems: "center", gap: "0.5rem" }}>
                <QRCodeSVG value={a.id} size={160} bgColor="#ffffff" fgColor="#0f1117" level="M" />
                <span style={{ fontSize: "0.72rem", color: "#64748b", wordBreak: "break-all", textAlign: "center" }}>
                  {a.id}
                </span>
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
