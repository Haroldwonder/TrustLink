import { useState, useEffect, useMemo } from "react";
import {
  getIssuerAttestations,
  getExpiringAttestations,
  getIssuerStats,
  renewAttestation,
  Attestation,
} from "../contract";
import { useIssuerStats } from "../../../../sdk/react/src";

interface Props { address: string; }

type StatusFilter = "all" | "valid" | "revoked" | "expired";

function deriveStatus(a: Attestation): "valid" | "revoked" | "expired" {
  if (a.revoked) return "revoked";
  if (a.expiration && a.expiration < BigInt(Math.floor(Date.now() / 1000))) return "expired";
  return "valid";
}

export default function IssuerDashboard({ address }: Props) {
  const { data: stats, loading: statsLoading, error: statsError } = useIssuerStats(address, getIssuerStats);
  const [recent, setRecent] = useState<Attestation[]>([]);
  const [expiring, setExpiring] = useState<Attestation[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [renewing, setRenewing] = useState<Set<string>>(new Set());

  const [filterText, setFilterText] = useState("");
  const [filterClaimType, setFilterClaimType] = useState("");
  const [filterStatus, setFilterStatus] = useState<StatusFilter>("all");

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    Promise.all([
      getIssuerAttestations(address, 0, 10),
      getExpiringAttestations(address, 30),
    ])
      .then(([r, e]) => {
        if (!cancelled) {
          setRecent(r);
          setExpiring(e);
          setLoading(false);
        }
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setError((err as Error).message);
          setLoading(false);
        }
      });
    return () => { cancelled = true; };
  }, [address]);

  function handleRefresh() {
    setLoading(true);
    setError(null);
    Promise.all([
      getIssuerAttestations(address, 0, 10),
      getExpiringAttestations(address, 30),
    ])
      .then(([r, e]) => {
        setRecent(r);
        setExpiring(e);
        setLoading(false);
      })
      .catch((err: unknown) => {
        setError((err as Error).message);
        setLoading(false);
      });
  }

  async function handleRenew(id: string, expiration: bigint) {
    setRenewing((prev) => new Set(prev).add(id));
    try {
      const newExpiration = expiration + BigInt(30 * 24 * 3600);
      await renewAttestation(address, id, newExpiration);
      handleRefresh();
    } catch {
      // errors surfaced via the parent status
    } finally {
      setRenewing((prev) => { const next = new Set(prev); next.delete(id); return next; });
    }
  }

  const claimTypes = useMemo(
    () => Array.from(new Set(recent.map((a) => a.claim_type))).sort(),
    [recent]
  );

  const filteredRecent = useMemo(() => {
    const text = filterText.toLowerCase();
    return recent.filter((a) => {
      if (filterClaimType && a.claim_type !== filterClaimType) return false;
      if (filterStatus !== "all" && deriveStatus(a) !== filterStatus) return false;
      if (text && !a.subject.toLowerCase().includes(text) && !a.id.toLowerCase().includes(text)) return false;
      return true;
    });
  }, [recent, filterText, filterClaimType, filterStatus]);

  if (statsLoading || loading) {
    return (
      <div className="panel">
        <h2>Issuer Dashboard</h2>
        <p className="empty">Loading…</p>
      </div>
    );
  }

  if (statsError || error) {
    return (
      <div className="panel">
        <h2>Issuer Dashboard</h2>
        <div className="alert alert-error">{statsError?.message ?? error}</div>
      </div>
    );
  }

  return (
    <div className="panel">
      <h2>Issuer Dashboard</h2>
      <p style={{ fontSize: "0.8rem", color: "#64748b", marginBottom: "1.5rem", fontFamily: "monospace" }}>
        {address}
      </p>

      {stats && (
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(150px, 1fr))", gap: "1rem", marginBottom: "2rem" }}>
          <div style={{ background: "#0f1117", border: "1px solid #2d3148", borderRadius: "0.5rem", padding: "1rem" }}>
            <div style={{ fontSize: "0.8rem", color: "#94a3b8", marginBottom: "0.5rem" }}>Total Issued</div>
            <div style={{ fontSize: "2rem", fontWeight: "bold", color: "#7c6af7" }}>{stats.total_issued}</div>
          </div>
          <div style={{ background: "#0f1117", border: "1px solid #2d3148", borderRadius: "0.5rem", padding: "1rem" }}>
            <div style={{ fontSize: "0.8rem", color: "#94a3b8", marginBottom: "0.5rem" }}>Active</div>
            <div style={{ fontSize: "2rem", fontWeight: "bold", color: "#10b981" }}>{stats.active}</div>
          </div>
          <div style={{ background: "#0f1117", border: "1px solid #2d3148", borderRadius: "0.5rem", padding: "1rem" }}>
            <div style={{ fontSize: "0.8rem", color: "#94a3b8", marginBottom: "0.5rem" }}>Revoked</div>
            <div style={{ fontSize: "2rem", fontWeight: "bold", color: "#ef4444" }}>{stats.revoked}</div>
          </div>
          <div style={{ background: "#0f1117", border: "1px solid #2d3148", borderRadius: "0.5rem", padding: "1rem" }}>
            <div style={{ fontSize: "0.8rem", color: "#94a3b8", marginBottom: "0.5rem" }}>Expired</div>
            <div style={{ fontSize: "2rem", fontWeight: "bold", color: "#f59e0b" }}>{stats.expired}</div>
          </div>
        </div>
      )}

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "2rem" }}>
        <div className="card">
          <h3>Recent Issuances</h3>

          <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap", marginBottom: "0.75rem", alignItems: "center" }}>
            <input
              style={{ flex: "1 1 120px", background: "#0f1117", border: "1px solid #2d3148", borderRadius: "0.5rem", padding: "0.35rem 0.6rem", color: "#e2e8f0", fontSize: "0.8rem" }}
              placeholder="Search subject or ID…"
              value={filterText}
              onChange={(e) => setFilterText(e.target.value)}
            />
            <select
              style={{ background: "#0f1117", border: "1px solid #2d3148", borderRadius: "0.5rem", padding: "0.35rem 0.4rem", color: "#e2e8f0", fontSize: "0.8rem" }}
              value={filterClaimType}
              onChange={(e) => setFilterClaimType(e.target.value)}
            >
              <option value="">All types</option>
              {claimTypes.map((ct) => <option key={ct} value={ct}>{ct}</option>)}
            </select>
            <select
              style={{ background: "#0f1117", border: "1px solid #2d3148", borderRadius: "0.5rem", padding: "0.35rem 0.4rem", color: "#e2e8f0", fontSize: "0.8rem" }}
              value={filterStatus}
              onChange={(e) => setFilterStatus(e.target.value as StatusFilter)}
            >
              <option value="all">All</option>
              <option value="valid">Valid</option>
              <option value="revoked">Revoked</option>
              <option value="expired">Expired</option>
            </select>
            {(filterText || filterClaimType || filterStatus !== "all") && (
              <button
                className="btn btn-outline"
                style={{ fontSize: "0.75rem", padding: "0.25rem 0.5rem" }}
                onClick={() => { setFilterText(""); setFilterClaimType(""); setFilterStatus("all"); }}
              >
                Clear
              </button>
            )}
            <span style={{ fontSize: "0.75rem", color: "#64748b", whiteSpace: "nowrap" }}>
              {filteredRecent.length}/{recent.length}
            </span>
          </div>

          {filteredRecent.length === 0 ? (
            <p className="empty">
              {recent.length === 0 ? "No attestations issued yet." : "No attestations match the current filters."}
            </p>
          ) : (
            <div className="att-list">
              {filteredRecent.map((a) => (
                <div key={a.id} className="att-item">
                  <div className="row">
                    <span className="claim">{a.claim_type}</span>
                    {a.revoked ? (
                      <span className="badge badge-revoked">Revoked</span>
                    ) : a.expiration && a.expiration < BigInt(Math.floor(Date.now() / 1000)) ? (
                      <span className="badge badge-expired">Expired</span>
                    ) : (
                      <span className="badge badge-valid">Valid</span>
                    )}
                  </div>
                  <span className="meta">Subject: {a.subject}</span>
                  <span className="meta">
                    Issued: {new Date(Number(a.timestamp) * 1000).toLocaleDateString()}
                  </span>
                  {a.expiration && (
                    <span className="meta">
                      Expires: {new Date(Number(a.expiration) * 1000).toLocaleDateString()}
                    </span>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="card">
          <h3>Expiring Soon (30 days)</h3>
          {expiring.length === 0 ? (
            <p className="empty">No attestations expiring soon.</p>
          ) : (
            <div className="att-list">
              {expiring.map((a) => (
                <div key={a.id} className="att-item">
                  <div className="row">
                    <span className="claim">{a.claim_type}</span>
                    <span className="badge badge-expired">Expiring</span>
                  </div>
                  <span className="meta">Subject: {a.subject}</span>
                  {a.expiration && (
                    <span className="meta">
                      Expires: {new Date(Number(a.expiration) * 1000).toLocaleDateString()}
                    </span>
                  )}
                  <span className="meta">
                    Days left:{" "}
                    {Math.ceil(
                      (Number(a.expiration || 0) - Math.floor(Date.now() / 1000)) / 86400
                    )}
                  </span>
                  <button
                    className="btn btn-primary"
                    style={{ marginTop: "0.5rem", width: "100%" }}
                    onClick={() => handleRenew(a.id, a.expiration!)}
                    disabled={renewing.has(a.id)}
                  >
                    {renewing.has(a.id) ? "Renewing..." : "Renew"}
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      <div style={{ marginTop: "2rem", textAlign: "center" }}>
        <button className="btn btn-outline" onClick={handleRefresh}>
          Refresh
        </button>
      </div>
    </div>
  );
}
