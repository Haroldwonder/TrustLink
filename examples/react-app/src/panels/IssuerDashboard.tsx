import { useState, useEffect } from "react";
import {
  getIssuerStats,
  getIssuerAttestations,
  getExpiringAttestations,
  Attestation,
  IssuerStats,
} from "../contract";

interface Props { address: string; }

export default function IssuerDashboard({ address }: Props) {
  const [stats, setStats] = useState<IssuerStats | null>(null);
  const [recent, setRecent] = useState<Attestation[]>([]);
  const [expiring, setExpiring] = useState<Attestation[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadDashboard();
  }, [address]);

  async function loadDashboard() {
    setLoading(true);
    setError(null);
    try {
      const [s, r, e] = await Promise.all([
        getIssuerStats(address),
        getIssuerAttestations(address, 0, 10),
        getExpiringAttestations(address, 30),
      ]);
      setStats(s);
      setRecent(r);
      setExpiring(e);
    } catch (err: unknown) {
      setError((err as Error).message);
    } finally {
      setLoading(false);
    }
  }

  if (loading) {
    return (
      <div className="panel">
        <h2>Issuer Dashboard</h2>
        <p className="empty">Loading…</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="panel">
        <h2>Issuer Dashboard</h2>
        <div className="alert alert-error">{error}</div>
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
          {recent.length === 0 ? (
            <p className="empty">No attestations issued yet.</p>
          ) : (
            <div className="att-list">
              {recent.map((a) => (
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
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      <div style={{ marginTop: "2rem", textAlign: "center" }}>
        <button className="btn btn-outline" onClick={loadDashboard}>
          Refresh
        </button>
      </div>
    </div>
  );
}
