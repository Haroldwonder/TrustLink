import { useState, useEffect } from "react";
import { createAttestation, revokeAttestation, getSubjectAttestations, listTemplates, createAttestationFromTemplate, getRateLimit, getRequireRegisteredClaimType, RateLimit, Attestation, AttestationTemplate } from "../contract";
import { SkeletonAttestationList } from "../SkeletonList";
import IssuerDashboard from "./IssuerDashboard";
import TemplatePanel from "./TemplatePanel";
import RateLimitPanel, { RATE_LIMIT_WARNING_THRESHOLD } from "./RateLimitPanel";

interface Props { address: string; }

export default function IssuerPanel({ address }: Props) {
  const [tab, setTab] = useState<"dashboard" | "create" | "revoke" | "lookup" | "templates">("dashboard");
  const [subject, setSubject] = useState("");
  const [claimType, setClaimType] = useState("");
  const [metadata, setMetadata] = useState("");
  const [status, setStatus] = useState<{ type: "success" | "error"; msg: string } | null>(null);
  const [loading, setLoading] = useState(false);

  const [rateLimit, setRateLimit] = useState<RateLimit | null>(null);

  useEffect(() => {
    getRateLimit(address)
      .then(setRateLimit)
      .catch(() => { /* rate limit info is advisory; silently ignore fetch errors */ });
  }, [address]);

  const nearLimit = rateLimit != null && rateLimit.limit > 0
    && (rateLimit.current_count / rateLimit.limit) >= RATE_LIMIT_WARNING_THRESHOLD;

  const [requireRegisteredClaimType, setRequireRegisteredClaimType] = useState(false);

  useEffect(() => {
    getRequireRegisteredClaimType().then(setRequireRegisteredClaimType).catch(() => null);
  }, []);

  const [revokeId, setRevokeId] = useState("");
  const [revokeReason, setRevokeReason] = useState("");

  const [lookupAddr, setLookupAddr] = useState("");
  const [attestations, setAttestations] = useState<Attestation[]>([]);
  const [lookupLoading, setLookupLoading] = useState(false);

  const [templates, setTemplates] = useState<AttestationTemplate[]>([]);
  const [selectedTemplate, setSelectedTemplate] = useState("");
  const [templateSubject, setTemplateSubject] = useState("");
  const [templateLoading, setTemplateLoading] = useState(false);

  useEffect(() => {
    listTemplates(address).then(setTemplates).catch(() => {});
  }, [address]);

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

  async function handleFromTemplate() {
    if (!selectedTemplate || !templateSubject) return;
    setTemplateLoading(true);
    setStatus(null);
    try {
      await createAttestationFromTemplate(address, selectedTemplate, templateSubject.trim(), null);
      setStatus({ type: "success", msg: "Attestation created from template." });
      setTemplateSubject("");
    } catch (e: unknown) {
      setStatus({ type: "error", msg: (e as Error).message });
    } finally {
      setTemplateLoading(false);
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
        <RateLimitPanel address={address} />
        <IssuerDashboard address={address} />
      </div>
    );
  }

  if (tab === "templates") {
    return (
      <div>
        <div style={{ display: "flex", gap: "0.5rem", marginBottom: "1rem", borderBottom: "1px solid #2d3148", paddingBottom: "0.5rem" }}>
          {(["dashboard", "create", "revoke", "lookup", "templates"] as const).map((t) => (
            <button key={t} className={`tab ${tab === t ? "active" : ""}`} onClick={() => setTab(t)} style={{ flex: 1, textAlign: "center", padding: "0.5rem", textTransform: "capitalize" }}>
              {t}
            </button>
          ))}
        </div>
        <TemplatePanel address={address} />
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
          {nearLimit && (
            <div className="alert alert-error" style={{ fontSize: "0.85rem" }}>
              Warning: you are near your rate-limit ({rateLimit!.current_count}/{rateLimit!.limit} used).
              This submission may be rejected with RateLimitExceeded.
            </div>
          )}
          {requireRegisteredClaimType && (
            <div className="alert alert-error" style={{ marginBottom: "1rem", fontSize: "0.8rem" }}>
              This contract requires claim types to be pre-registered. Free-text claim types will be rejected.
            </div>
          )}
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
          {templates.length > 0 && (
            <div style={{ marginTop: "1.5rem", borderTop: "1px solid #2d3148", paddingTop: "1rem" }}>
              <h4 style={{ marginBottom: "0.75rem", color: "#94a3b8", fontSize: "0.85rem" }}>Or create from template</h4>
              <div className="field">
                <label>Template</label>
                <select
                  value={selectedTemplate}
                  onChange={(e) => setSelectedTemplate(e.target.value)}
                  style={{ background: "#0f1117", border: "1px solid #2d3148", borderRadius: "0.5rem", padding: "0.5rem 0.75rem", color: "#e2e8f0", width: "100%" }}
                >
                  <option value="">Select a template…</option>
                  {templates.map((t) => (
                    <option key={t.template_id} value={t.template_id}>
                      {t.template_id} — {t.claim_type}
                    </option>
                  ))}
                </select>
              </div>
              <div className="field">
                <label>Subject Address</label>
                <input
                  value={templateSubject}
                  onChange={(e) => setTemplateSubject(e.target.value)}
                  placeholder="G..."
                />
              </div>
              <button
                className="btn btn-outline"
                disabled={templateLoading || !selectedTemplate || !templateSubject}
                onClick={handleFromTemplate}
              >
                {templateLoading ? "Creating…" : "Create from Template"}
              </button>
            </div>
          )}
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
