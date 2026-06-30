import { useState, useEffect, useCallback } from "react";
import {
  setWhitelistEnabled,
  isWhitelistEnabled,
  addToWhitelist,
  removeFromWhitelist,
  isWhitelisted,
} from "../contract";

interface Props {
  address: string;
}

const PAGE_SIZE = 10;

export default function WhitelistPanel({ address }: Props) {
  const [tab, setTab] = useState<"manage" | "check" | "list">("manage");
  const [status, setStatus] = useState<{ type: "success" | "error"; msg: string } | null>(null);
  const [loading, setLoading] = useState(false);

  // Whitelist enabled state
  const [enabled, setEnabled] = useState<boolean | null>(null);

  // Add / remove single
  const [subject, setSubject] = useState("");

  // Bulk add (comma-separated or CSV paste)
  const [bulkInput, setBulkInput] = useState("");

  // Check
  const [checkSubject, setCheckSubject] = useState("");
  const [checkResult, setCheckResult] = useState<boolean | null>(null);

  // Local list tracking
  const [whitelist, setWhitelist] = useState<string[]>([]);
  const [page, setPage] = useState(0);

  const pageItems = whitelist.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE);
  const totalPages = Math.max(1, Math.ceil(whitelist.length / PAGE_SIZE));

  const loadStatus = useCallback(async () => {
    try {
      const e = await isWhitelistEnabled(address);
      setEnabled(e);
    } catch { /* ignore */ }
  }, [address]);

  useEffect(() => { loadStatus(); }, [loadStatus]);

  function setOk(msg: string) { setStatus({ type: "success", msg }); }
  function setErr(e: unknown) { setStatus({ type: "error", msg: (e as Error).message }); }

  async function handleToggle() {
    if (enabled === null) return;
    setLoading(true); setStatus(null);
    try {
      await setWhitelistEnabled(address, !enabled);
      setEnabled(!enabled);
      setOk(`Whitelist mode ${!enabled ? "enabled" : "disabled"}.`);
    } catch (e) { setErr(e); }
    finally { setLoading(false); }
  }

  async function handleAdd() {
    const addr = subject.trim();
    if (!addr) return;
    setLoading(true); setStatus(null);
    try {
      await addToWhitelist(address, addr);
      setWhitelist((prev) => prev.includes(addr) ? prev : [...prev, addr]);
      setOk(`Added ${addr.slice(0, 8)}… to whitelist.`);
      setSubject("");
    } catch (e) { setErr(e); }
    finally { setLoading(false); }
  }

  async function handleRemove(target: string) {
    setLoading(true); setStatus(null);
    try {
      await removeFromWhitelist(address, target);
      setWhitelist((prev) => prev.filter((s) => s !== target));
      setOk(`Removed ${target.slice(0, 8)}… from whitelist.`);
    } catch (e) { setErr(e); }
    finally { setLoading(false); }
  }

  async function handleBulkAdd() {
    const entries = bulkInput
      .split(/[\n,]+/)
      .map((s) => s.trim())
      .filter((s) => s.length > 0);
    if (entries.length === 0) return;
    setLoading(true); setStatus(null);
    let added = 0;
    const errors: string[] = [];
    for (const entry of entries) {
      try {
        await addToWhitelist(address, entry);
        setWhitelist((prev) => prev.includes(entry) ? prev : [...prev, entry]);
        added++;
      } catch {
        errors.push(entry.slice(0, 8) + "…");
      }
    }
    if (errors.length === 0) {
      setOk(`Added ${added} address${added !== 1 ? "es" : ""}.`);
      setBulkInput("");
    } else {
      setStatus({ type: "error", msg: `Added ${added}, failed: ${errors.join(", ")}` });
    }
    setLoading(false);
  }

  async function handleCheck() {
    if (!checkSubject) return;
    setLoading(true); setStatus(null); setCheckResult(null);
    try {
      const result = await isWhitelisted(address, checkSubject.trim());
      setCheckResult(result);
    } catch (e) { setErr(e); }
    finally { setLoading(false); }
  }

  const TABS = [
    { id: "manage" as const, label: "Manage" },
    { id: "check" as const, label: "Check" },
    { id: "list" as const, label: `List (${whitelist.length})` },
  ];

  return (
    <div className="panel">
      <h2>Whitelist</h2>
      <div
        style={{
          display: "flex",
          gap: "0.5rem",
          marginBottom: "1rem",
          borderBottom: "1px solid #2d3148",
          paddingBottom: "0.5rem",
        }}
      >
        {TABS.map((t) => (
          <button
            key={t.id}
            className={`tab ${tab === t.id ? "active" : ""}`}
            onClick={() => { setTab(t.id); setStatus(null); }}
            style={{ flex: 1, textAlign: "center", padding: "0.5rem" }}
          >
            {t.label}
          </button>
        ))}
      </div>

      {status && <div className={`alert alert-${status.type}`}>{status.msg}</div>}

      {tab === "manage" && (
        <>
          {/* Toggle */}
          <div className="card" style={{ marginBottom: "1rem" }}>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
              <div>
                <h3 style={{ margin: 0 }}>Whitelist Mode</h3>
                <p style={{ fontSize: "0.85rem", color: "#94a3b8", margin: "0.25rem 0 0" }}>
                  {enabled === null ? "Loading…" : enabled ? "Enabled — only whitelisted subjects may receive attestations." : "Disabled — all subjects are accepted."}
                </p>
              </div>
              <button
                className={`btn ${enabled ? "btn-danger" : "btn-primary"}`}
                disabled={loading || enabled === null}
                onClick={handleToggle}
                style={{ minWidth: "7rem" }}
              >
                {enabled ? "Disable" : "Enable"}
              </button>
            </div>
          </div>

          {/* Add single */}
          <div className="card" style={{ marginBottom: "1rem" }}>
            <h3>Add Subject</h3>
            <div style={{ display: "flex", gap: "0.5rem" }}>
              <input
                style={{ flex: 1, background: "#0f1117", border: "1px solid #2d3148", borderRadius: "0.5rem", padding: "0.5rem 0.75rem", color: "#e2e8f0" }}
                value={subject}
                onChange={(e) => setSubject(e.target.value)}
                placeholder="G..."
              />
              <button className="btn btn-primary" disabled={loading || !subject} onClick={handleAdd}>
                Add
              </button>
            </div>
          </div>

          {/* Bulk add */}
          <div className="card">
            <h3>Bulk Add</h3>
            <p style={{ fontSize: "0.85rem", color: "#94a3b8", marginBottom: "0.5rem" }}>
              Paste comma-separated or newline-separated addresses.
            </p>
            <textarea
              rows={4}
              style={{ width: "100%", background: "#0f1117", border: "1px solid #2d3148", borderRadius: "0.5rem", padding: "0.5rem 0.75rem", color: "#e2e8f0", resize: "vertical", boxSizing: "border-box" }}
              value={bulkInput}
              onChange={(e) => setBulkInput(e.target.value)}
              placeholder={"GABC…,GXYZ…\nor one per line"}
            />
            <button
              className="btn btn-primary"
              disabled={loading || !bulkInput.trim()}
              onClick={handleBulkAdd}
              style={{ marginTop: "0.5rem" }}
            >
              {loading ? "Adding…" : "Bulk Add"}
            </button>
          </div>
        </>
      )}

      {tab === "check" && (
        <div className="card">
          <h3>Check Address</h3>
          <div style={{ display: "flex", gap: "0.5rem", marginBottom: "0.75rem" }}>
            <input
              style={{ flex: 1, background: "#0f1117", border: "1px solid #2d3148", borderRadius: "0.5rem", padding: "0.5rem 0.75rem", color: "#e2e8f0" }}
              value={checkSubject}
              onChange={(e) => setCheckSubject(e.target.value)}
              placeholder="G..."
            />
            <button className="btn btn-outline" disabled={loading || !checkSubject} onClick={handleCheck}>
              Check
            </button>
          </div>
          {checkResult !== null && (
            <div className={`alert alert-${checkResult ? "success" : "error"}`}>
              {checkResult ? "✅ Whitelisted" : "❌ Not whitelisted"}
            </div>
          )}
        </div>
      )}

      {tab === "list" && (
        <div className="card">
          <h3>Whitelist ({whitelist.length} addresses this session)</h3>
          <p style={{ fontSize: "0.85rem", color: "#94a3b8", marginBottom: "0.75rem" }}>
            Showing addresses added during this session. The contract has no enumeration endpoint.
          </p>
          {whitelist.length === 0 ? (
            <p className="empty">No addresses added yet.</p>
          ) : (
            <>
              <div className="att-list">
                {pageItems.map((s) => (
                  <div key={s} className="att-item">
                    <div className="row">
                      <span className="meta" style={{ fontFamily: "monospace", fontSize: "0.8rem" }}>{s}</span>
                      <button
                        className="btn btn-danger"
                        style={{ fontSize: "0.75rem", padding: "0.2rem 0.6rem" }}
                        disabled={loading}
                        onClick={() => handleRemove(s)}
                      >
                        Remove
                      </button>
                    </div>
                  </div>
                ))}
              </div>
              {totalPages > 1 && (
                <div style={{ display: "flex", gap: "0.5rem", marginTop: "0.75rem", alignItems: "center" }}>
                  <button className="btn btn-outline" disabled={page === 0} onClick={() => setPage((p) => p - 1)}>← Prev</button>
                  <span style={{ fontSize: "0.85rem", color: "#94a3b8" }}>Page {page + 1} / {totalPages}</span>
                  <button className="btn btn-outline" disabled={page >= totalPages - 1} onClick={() => setPage((p) => p + 1)}>Next →</button>
                </div>
              )}
            </>
          )}
        </div>
      )}
    </div>
  );
}
