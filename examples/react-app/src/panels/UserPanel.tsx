import { useState, useEffect } from "react";
import { getSubjectAttestations, Attestation } from "../contract";

interface Props { address: string; }

export default function UserPanel({ address }: Props) {
  const [attestations, setAttestations] = useState<Attestation[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

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
      {loading && <p className="empty">Loading…</p>}

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
          </div>
        ))}
      </div>
    </div>
  );
}
