import { useState } from "react";
import { hasValidClaim, getSubjectAttestations, getAttestation, Attestation } from "../contract";

type InputMode = "manual" | "scan";

export default function VerifierPanel() {
  const [subject, setSubject] = useState("");
  const [claimType, setClaimType] = useState("");
  const [checkResult, setCheckResult] = useState<boolean | null>(null);
  const [attestations, setAttestations] = useState<Attestation[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [inputMode, setInputMode] = useState<InputMode>("manual");
  const [scannedId, setScannedId] = useState("");
  const [scannedAttestation, setScannedAttestation] = useState<Attestation | null>(null);
  const [scanLoading, setScanLoading] = useState(false);
  const [scanError, setScanError] = useState<string | null>(null);

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

  async function handleScanLookup() {
    if (!scannedId.trim()) return;
    setScanLoading(true);
    setScanError(null);
    setScannedAttestation(null);
    try {
      const att = await getAttestation(scannedId.trim());
      setScannedAttestation(att);
    } catch (e: unknown) {
      setScanError((e as Error).message);
    } finally {
      setScanLoading(false);
    }
  }

  return (
    <div className="panel">
      <h2>Verifier Panel</h2>

      <div style={{ display: "flex", gap: "0.5rem", marginBottom: "1rem", borderBottom: "1px solid #2d3148", paddingBottom: "0.5rem" }}>
        <button
          className={`tab ${inputMode === "manual" ? "active" : ""}`}
          onClick={() => setInputMode("manual")}
          style={{ flex: 1, textAlign: "center", padding: "0.4rem" }}
        >
          Manual
        </button>
        <button
          className={`tab ${inputMode === "scan" ? "active" : ""}`}
          onClick={() => setInputMode("scan")}
          style={{ flex: 1, textAlign: "center", padding: "0.4rem" }}
        >
          Scan / Paste ID
        </button>
      </div>

      {inputMode === "scan" && (
        <div className="card">
          <h3>Look Up by Attestation ID</h3>
          <p style={{ fontSize: "0.82rem", color: "#64748b", marginBottom: "0.75rem" }}>
            Paste or type the attestation ID from a QR code scan.
          </p>
          {scanError && <div className="alert alert-error">{scanError}</div>}
          <div style={{ display: "flex", gap: "0.5rem", marginBottom: "1rem" }}>
            <input
              style={{ flex: 1, background: "#0f1117", border: "1px solid #2d3148", borderRadius: "0.5rem", padding: "0.5rem 0.75rem", color: "#e2e8f0" }}
              value={scannedId}
              onChange={(e) => { setScannedId(e.target.value); setScannedAttestation(null); setScanError(null); }}
              placeholder="Attestation ID…"
            />
            <button className="btn btn-primary" disabled={scanLoading || !scannedId.trim()} onClick={handleScanLookup}>
              {scanLoading ? "Looking up…" : "Look Up"}
            </button>
          </div>
          {scannedAttestation && (
            <div className="att-item">
              <div className="row">
                <span className="claim">{scannedAttestation.claim_type}</span>
                <span className={`badge ${scannedAttestation.revoked ? "badge-revoked" : "badge-valid"}`}>
                  {scannedAttestation.revoked ? "Revoked" : "Valid"}
                </span>
              </div>
              <span className="meta">Issuer: {scannedAttestation.issuer}</span>
              <span className="meta">Subject: {scannedAttestation.subject}</span>
              {scannedAttestation.expiration && (
                <span className="meta">
                  Expires: {new Date(Number(scannedAttestation.expiration) * 1000).toLocaleDateString()}
                </span>
              )}
              <span className="meta">ID: {scannedAttestation.id}</span>
            </div>
          )}
        </div>
      )}

      {inputMode === "manual" && (
        <>
          {error && <div className="alert alert-error">{error}</div>}
          <div className="card">
            <h3>Check Claim</h3>
            <div className="field">
              <label>Subject Address</label>
              <input value={subject} onChange={(e) => { setSubject(e.target.value); setCheckResult(null); }} placeholder="G..." />
            </div>
            <div className="field">
              <label>Claim Type</label>
              <input value={claimType} onChange={(e) => { setClaimType(e.target.value); setCheckResult(null); }} placeholder="KYC, AML…" />
            </div>
            <div style={{ display: "flex", gap: "0.5rem" }}>
              <button className="btn btn-primary" disabled={loading || !subject || !claimType} onClick={handleCheck}>
                Verify Claim
              </button>
              <button className="btn btn-outline" disabled={loading || !subject} onClick={handleLoadAll}>
                Load All Attestations
              </button>
            </div>

            {checkResult !== null && (
              <div className={`alert ${checkResult ? "alert-success" : "alert-error"}`} style={{ marginTop: "1rem" }}>
                {checkResult
                  ? `✓ ${subject.slice(0, 8)}… holds a valid "${claimType}" claim.`
                  : `✗ No valid "${claimType}" claim found for this address.`}
              </div>
            )}
          </div>

          {attestations.length > 0 && (
            <div className="card">
              <h3>All Attestations for {subject.slice(0, 12)}…</h3>
              <div className="att-list">
                {attestations.map((a) => (
                  <div key={a.id} className="att-item">
                    <div className="row">
                      <span className="claim">{a.claim_type}</span>
                      <span className={`badge ${a.revoked ? "badge-revoked" : "badge-valid"}`}>
                        {a.revoked ? "Revoked" : "Valid"}
                      </span>
                    </div>
                    <span className="meta">Issuer: {a.issuer}</span>
                    <span className="meta">ID: {a.id}</span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}
