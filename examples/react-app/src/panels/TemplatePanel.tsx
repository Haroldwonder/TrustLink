import { useState, useEffect, useCallback } from "react";
import {
  listTemplates,
  createTemplate,
  deleteTemplate,
  AttestationTemplate,
} from "../contract";

interface Props { address: string; }

export default function TemplatePanel({ address }: Props) {
  const [templates, setTemplates] = useState<AttestationTemplate[]>([]);
  const [loading, setLoading] = useState(true);
  const [status, setStatus] = useState<{ type: "success" | "error"; msg: string } | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [deleting, setDeleting] = useState<Set<string>>(new Set());

  const [templateId, setTemplateId] = useState("");
  const [claimType, setClaimType] = useState("");
  const [metadata, setMetadata] = useState("");

  const load = useCallback(() => {
    setLoading(true);
    listTemplates(address)
      .then(setTemplates)
      .catch((e: unknown) => setStatus({ type: "error", msg: (e as Error).message }))
      .finally(() => setLoading(false));
  }, [address]);

  useEffect(() => { load(); }, [load]);

  async function handleCreate() {
    if (!templateId || !claimType) return;
    setSubmitting(true);
    setStatus(null);
    try {
      await createTemplate(address, templateId.trim(), claimType.trim(), metadata || null);
      setStatus({ type: "success", msg: "Template created." });
      setTemplateId(""); setClaimType(""); setMetadata("");
      load();
    } catch (e: unknown) {
      setStatus({ type: "error", msg: (e as Error).message });
    } finally {
      setSubmitting(false);
    }
  }

  async function handleDelete(id: string) {
    setDeleting((prev) => new Set(prev).add(id));
    setStatus(null);
    try {
      await deleteTemplate(address, id);
      setStatus({ type: "success", msg: "Template deleted." });
      load();
    } catch (e: unknown) {
      setStatus({ type: "error", msg: (e as Error).message });
    } finally {
      setDeleting((prev) => { const s = new Set(prev); s.delete(id); return s; });
    }
  }

  return (
    <div className="panel">
      <h2>Attestation Templates</h2>

      {status && <div className={`alert alert-${status.type}`}>{status.msg}</div>}

      <div className="card" style={{ marginBottom: "1.5rem" }}>
        <h3>Create Template</h3>
        <div className="field">
          <label>Template ID</label>
          <input
            value={templateId}
            onChange={(e) => setTemplateId(e.target.value)}
            placeholder="e.g. kyc-standard"
          />
        </div>
        <div className="field">
          <label>Claim Type</label>
          <input
            value={claimType}
            onChange={(e) => setClaimType(e.target.value)}
            placeholder="KYC, AML, accredited-investor…"
          />
        </div>
        <div className="field">
          <label>Metadata (optional)</label>
          <input
            value={metadata}
            onChange={(e) => setMetadata(e.target.value)}
            placeholder="optional note"
          />
        </div>
        <button
          className="btn btn-primary"
          disabled={submitting || !templateId || !claimType}
          onClick={handleCreate}
        >
          {submitting ? "Creating…" : "Create Template"}
        </button>
      </div>

      <div className="card">
        <h3>My Templates</h3>
        {loading ? (
          <p className="empty">Loading…</p>
        ) : templates.length === 0 ? (
          <p className="empty">No templates yet.</p>
        ) : (
          <div className="att-list">
            {templates.map((t) => (
              <div key={t.template_id} className="att-item">
                <div className="row">
                  <span className="claim">{t.claim_type}</span>
                  <span className="meta" style={{ fontFamily: "monospace", fontSize: "0.75rem" }}>
                    {t.template_id}
                  </span>
                </div>
                {t.metadata && <span className="meta">Note: {t.metadata}</span>}
                <button
                  className="btn btn-danger"
                  style={{ marginTop: "0.4rem", fontSize: "0.75rem", padding: "0.2rem 0.6rem" }}
                  disabled={deleting.has(t.template_id)}
                  onClick={() => handleDelete(t.template_id)}
                >
                  {deleting.has(t.template_id) ? "Deleting…" : "Delete"}
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
