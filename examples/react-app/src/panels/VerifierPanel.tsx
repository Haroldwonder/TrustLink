import { useState } from "react";
import { useTranslation } from "react-i18next";
import { hasValidClaim, getSubjectAttestations, getAttestation, isWhitelistEnabled, isWhitelisted, Attestation } from "../contract";

type InputMode = "manual" | "scan";

export default function VerifierPanel() {
  const { t } = useTranslation();
  const [subject, setSubject] = useState("");
  const [claimType, setClaimType] = useState("");
  const [issuer, setIssuer] = useState("");
  const [checkResult, setCheckResult] = useState<boolean | null>(null);
  const [whitelistEnabled, setWhitelistEnabled] = useState<boolean | null>(null);
  const [whitelistedResult, setWhitelistedResult] = useState<boolean | null>(null);
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
    setWhitelistEnabled(null);
    setWhitelistedResult(null);
    try {
      const result = await hasValidClaim(subject.trim(), claimType.trim());
      setCheckResult(result);
      if (issuer.trim()) {
        const enabled = await isWhitelistEnabled(issuer.trim());
        setWhitelistEnabled(enabled);
        if (enabled) {
          const listed = await isWhitelisted(issuer.trim(), subject.trim());
          setWhitelistedResult(listed);
        }
      }
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
      <h2>{t("verifier.title")}</h2>

      <div style={{ display: "flex", gap: "0.5rem", marginBottom: "1rem", borderBottom: "1px solid #2d3148", paddingBottom: "0.5rem" }}>
        <button
          className={`tab ${inputMode === "manual" ? "active" : ""}`}
          onClick={() => setInputMode("manual")}
          style={{ flex: 1, textAlign: "center", padding: "0.4rem" }}
        >
          {t("verifier.tab_manual")}
        </button>
        <button
          className={`tab ${inputMode === "scan" ? "active" : ""}`}
          onClick={() => setInputMode("scan")}
          style={{ flex: 1, textAlign: "center", padding: "0.4rem" }}
        >
          {t("verifier.tab_scan")}
        </button>
      </div>

      {inputMode === "scan" && (
        <div className="card">
          <h3>{t("verifier.scan_title")}</h3>
          <p style={{ fontSize: "0.82rem", color: "#64748b", marginBottom: "0.75rem" }}>
            {t("verifier.scan_desc")}
          </p>
          {scanError && <div className="alert alert-error">{scanError}</div>}
          <div style={{ display: "flex", gap: "0.5rem", marginBottom: "1rem" }}>
            <input
              style={{ flex: 1, background: "#0f1117", border: "1px solid #2d3148", borderRadius: "0.5rem", padding: "0.5rem 0.75rem", color: "#e2e8f0" }}
              value={scannedId}
              onChange={(e) => { setScannedId(e.target.value); setScannedAttestation(null); setScanError(null); }}
              placeholder={t("verifier.attestation_id_placeholder")}
            />
            <button className="btn btn-primary" disabled={scanLoading || !scannedId.trim()} onClick={handleScanLookup}>
              {scanLoading ? t("verifier.looking_up") : t("verifier.look_up")}
            </button>
          </div>
          {scannedAttestation && (
            <div className="att-item">
              <div className="row">
                <span className="claim">{scannedAttestation.claim_type}</span>
                <span className={`badge ${scannedAttestation.revoked ? "badge-revoked" : "badge-valid"}`}>
                  {scannedAttestation.revoked ? t("common.revoked") : t("common.valid")}
                </span>
              </div>
              <span className="meta">{t("common.issuer", { value: scannedAttestation.issuer })}</span>
              <span className="meta">{t("common.subject", { value: scannedAttestation.subject })}</span>
              {scannedAttestation.expiration && (
                <span className="meta">
                  {t("common.expires", { date: new Date(Number(scannedAttestation.expiration) * 1000).toLocaleDateString() })}
                </span>
              )}
              <span className="meta">{t("common.id", { id: scannedAttestation.id })}</span>
            </div>
          )}
        </div>
      )}

      {inputMode === "manual" && (
        <>
          {error && <div className="alert alert-error">{error}</div>}
          <div className="card">
            <h3>{t("verifier.check_claim_title")}</h3>
            <div className="field">
              <label>{t("issuer.subject_address")}</label>
              <input value={subject} onChange={(e) => { setSubject(e.target.value); setCheckResult(null); }} placeholder="G..." />
            </div>
            <div className="field">
              <label>{t("issuer.claim_type")}</label>
              <input value={claimType} onChange={(e) => { setClaimType(e.target.value); setCheckResult(null); }} placeholder="KYC, AML…" />
            </div>
            <div style={{ display: "flex", gap: "0.5rem" }}>
              <button className="btn btn-primary" disabled={loading || !subject || !claimType} onClick={handleCheck}>
                {t("verifier.verify_claim")}
              </button>
              <button className="btn btn-outline" disabled={loading || !subject} onClick={handleLoadAll}>
                {t("verifier.load_all")}
              </button>
            </div>

            {checkResult !== null && (
              <div className={`alert ${checkResult ? "alert-success" : "alert-error"}`} style={{ marginTop: "1rem" }}>
                {checkResult
                  ? t("verifier.claim_valid", { address: subject.slice(0, 8) + "…", claimType })
                  : t("verifier.claim_invalid", { claimType })}
              </div>
            )}
          </div>

          {attestations.length > 0 && (
            <div className="card">
              <h3>{t("verifier.all_for", { address: subject.slice(0, 12) + "…" })}</h3>
              <div className="att-list">
                {attestations.map((a) => (
                  <div key={a.id} className="att-item">
                    <div className="row">
                      <span className="claim">{a.claim_type}</span>
                      <span className={`badge ${a.revoked ? "badge-revoked" : "badge-valid"}`}>
                        {a.revoked ? t("common.revoked") : t("common.valid")}
                      </span>
                    </div>
                    <span className="meta">{t("common.issuer", { value: a.issuer })}</span>
                    <span className="meta">{t("common.id", { id: a.id })}</span>
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
