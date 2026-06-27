import { useState } from "react";
import { hasValidClaim, getSubjectAttestations, Attestation } from "../contract";

export default function VerifierPanel() {
  const [subject, setSubject] = useState("");
  const [claimType, setClaimType] = useState("");
  const [checkResult, setCheckResult] = useState<boolean | null>(null);
  const [attestations, setAttestations] = useState<Attestation[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleCheck() {
    if (!subject || !claimType) return;
    setLoading(true);
    setError(null);
    setCheckResult(null);
    try {
      const result = await hasValidClaim(subject.trim(), claimType.trim());
      setCheckResult(result);
    } catch (e: unknown) {
      setError((e as Error).message);
    } finally {
      setLoading(false);
    }
  }

  async function handleLoadAll() {
    if (!subject) return;
    setLoading(true);
    setError(null);
    try {
      const list = await getSubjectAttestations(subject.trim());
      setAttestations(list);
    } catch (e: unknown) {
      setError((e as Error).message);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="panel">
      <h2>Verifier Panel</h2>
      {error && <div role="alert" className="alert alert-error">{error}</div>}

      <div className="card">
        <h3>Check Claim</h3>
        <div className="field">
          <label htmlFor="verifier-subject">Subject Address</label>
          <input
            id="verifier-subject"
            value={subject}
            onChange={(e) => { setSubject(e.target.value); setCheckResult(null); }}
            placeholder="G..."
          />
        </div>
        <div className="field">
          <label htmlFor="verifier-claim-type">Claim Type</label>
          <input
            id="verifier-claim-type"
            value={claimType}
            onChange={(e) => { setClaimType(e.target.value); setCheckResult(null); }}
            placeholder="KYC_PASSED, AML_CLEARED…"
          />
        </div>
        <div style={{ display: "flex", gap: "0.5rem" }}>
          <button
            className="btn btn-primary"
            disabled={loading || !subject || !claimType}
            onClick={handleCheck}
            aria-disabled={loading || !subject || !claimType}
          >
            Verify Claim
          </button>
          <button
            className="btn btn-outline"
            disabled={loading || !subject}
            onClick={handleLoadAll}
            aria-disabled={loading || !subject}
          >
            Load All Attestations
          </button>
        </div>

        {checkResult !== null && (
          <div
            role="status"
            aria-live="polite"
            className={`alert ${checkResult ? "alert-success" : "alert-error"}`}
            style={{ marginTop: "1rem" }}
          >
            {checkResult
              ? `✓ ${subject.slice(0, 8)}… holds a valid "${claimType}" claim.`
              : `✗ No valid "${claimType}" claim found for this address.`}
          </div>
        )}
      </div>

      {attestations.length > 0 && (
        <div className="card">
          <h3>All Attestations for {subject.slice(0, 12)}…</h3>
          <ul className="att-list" aria-label="All attestations">
            {attestations.map((a) => (
              <li key={a.id} className="att-item">
                <div className="row">
                  <span className="claim">{a.claim_type}</span>
                  <span className={`badge ${a.revoked ? "badge-revoked" : "badge-valid"}`}>
                    {a.revoked ? "Revoked" : "Valid"}
                  </span>
                </div>
                <span className="meta">Issuer: {a.issuer}</span>
                <span className="meta">ID: {a.id}</span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
