import { useState, useEffect } from "react";
import { registerIssuer, removeIssuer, isIssuer, getConfig, type ContractConfig } from "../contract";
import { useGlobalStats } from "../hooks/useGlobalStats";

interface Props { address: string; }

export default function AdminPanel({ address }: Props) {
  const [issuerAddr, setIssuerAddr] = useState("");
  const [status, setStatus] = useState<{ type: "success" | "error" | "info"; msg: string } | null>(null);
  const [loading, setLoading] = useState(false);
  const [checkAddr, setCheckAddr] = useState("");
  const [checkResult, setCheckResult] = useState<boolean | null>(null);
  const [config, setConfig] = useState<ContractConfig | null>(null);
  const [configError, setConfigError] = useState<string | null>(null);

  const { data: stats, loading: statsLoading, error: statsError } = useGlobalStats(30_000);

  useEffect(() => {
    getConfig()
      .then(setConfig)
      .catch((e: unknown) => setConfigError((e as Error).message));
  }, []);

  async function handle(action: "register" | "remove") {
    if (!issuerAddr.trim()) return;
    setLoading(true);
    setStatus(null);
    try {
      if (action === "register") {
        await registerIssuer(address, issuerAddr.trim());
        setStatus({ type: "success", msg: `Issuer ${issuerAddr} registered.` });
      } else {
        await removeIssuer(address, issuerAddr.trim());
        setStatus({ type: "success", msg: `Issuer ${issuerAddr} removed.` });
      }
      setIssuerAddr("");
    } catch (e: unknown) {
      setStatus({ type: "error", msg: (e as Error).message });
    } finally {
      setLoading(false);
    }
  }

  async function handleCheck() {
    if (!checkAddr.trim()) return;
    setLoading(true);
    try {
      const result = await isIssuer(checkAddr.trim());
      setCheckResult(result);
    } catch (e: unknown) {
      setStatus({ type: "error", msg: (e as Error).message });
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="panel">
      <h2>Admin Panel</h2>

      {status && (
        <div role="alert" className={`alert alert-${status.type}`}>
          {status.msg}
        </div>
      )}

      <div className="card">
        <h3>Global Stats</h3>
        {statsLoading && <p className="empty">Loading…</p>}
        {statsError && <div role="alert" className="alert alert-error">{statsError}</div>}
        {stats && (
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(140px, 1fr))", gap: "1rem" }}>
            <div style={{ background: "#0f1117", border: "1px solid #2d3148", borderRadius: "0.5rem", padding: "1rem" }}>
              <div style={{ fontSize: "0.8rem", color: "#94a3b8", marginBottom: "0.5rem" }}>Total Attestations</div>
              <div style={{ fontSize: "1.75rem", fontWeight: "bold", color: "#7c6af7" }} aria-label={`Total attestations: ${stats.total_attestations.toString()}`}>{stats.total_attestations.toString()}</div>
            </div>
            <div style={{ background: "#0f1117", border: "1px solid #2d3148", borderRadius: "0.5rem", padding: "1rem" }}>
              <div style={{ fontSize: "0.8rem", color: "#94a3b8", marginBottom: "0.5rem" }}>Total Revocations</div>
              <div style={{ fontSize: "1.75rem", fontWeight: "bold", color: "#ef4444" }} aria-label={`Total revocations: ${stats.total_revocations.toString()}`}>{stats.total_revocations.toString()}</div>
            </div>
            <div style={{ background: "#0f1117", border: "1px solid #2d3148", borderRadius: "0.5rem", padding: "1rem" }}>
              <div style={{ fontSize: "0.8rem", color: "#94a3b8", marginBottom: "0.5rem" }}>Total Issuers</div>
              <div style={{ fontSize: "1.75rem", fontWeight: "bold", color: "#10b981" }} aria-label={`Total issuers: ${stats.total_issuers.toString()}`}>{stats.total_issuers.toString()}</div>
            </div>
          </div>
        )}
      </div>

      <div className="card">
        <h3>Contract Config</h3>
        {configError && <div role="alert" className="alert alert-error">{configError}</div>}
        {config && (
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "0.875rem" }}>
            <tbody>
              <tr><td style={{ color: "#94a3b8", padding: "0.25rem 0" }}>TTL (days)</td><td>{config.ttl_config.ttl_days}</td></tr>
              <tr><td style={{ color: "#94a3b8", padding: "0.25rem 0" }}>Max per issuer</td><td>{config.limits.max_attestations_per_issuer}</td></tr>
              <tr><td style={{ color: "#94a3b8", padding: "0.25rem 0" }}>Max per subject</td><td>{config.limits.max_attestations_per_subject}</td></tr>
              <tr><td style={{ color: "#94a3b8", padding: "0.25rem 0" }}>Attestation fee</td><td>{config.fee_config.attestation_fee.toString()}</td></tr>
              <tr><td style={{ color: "#94a3b8", padding: "0.25rem 0" }}>Fee collector</td><td style={{ wordBreak: "break-all" }}>{config.fee_config.fee_collector}</td></tr>
            </tbody>
          </table>
        )}
      </div>

      <div className="card">
        <h3>Register / Remove Issuer</h3>
        <div className="field">
          <label htmlFor="admin-issuer-addr">Issuer Address</label>
          <input
            id="admin-issuer-addr"
            value={issuerAddr}
            onChange={(e) => setIssuerAddr(e.target.value)}
            placeholder="G..."
            aria-describedby="admin-issuer-hint"
          />
          <span id="admin-issuer-hint" style={{ fontSize: "0.75rem", color: "var(--text-muted)" }}>
            Enter a Stellar address starting with G
          </span>
        </div>
        <div style={{ display: "flex", gap: "0.5rem" }}>
          <button
            className="btn btn-primary"
            disabled={loading || !issuerAddr}
            onClick={() => handle("register")}
            aria-disabled={loading || !issuerAddr}
          >
            Register
          </button>
          <button
            className="btn btn-danger"
            disabled={loading || !issuerAddr}
            onClick={() => handle("remove")}
            aria-disabled={loading || !issuerAddr}
          >
            Remove
          </button>
        </div>
      </div>

      <div className="card">
        <h3>Check Issuer Status</h3>
        <div className="field">
          <label htmlFor="admin-check-addr">Address to check</label>
          <input
            id="admin-check-addr"
            value={checkAddr}
            onChange={(e) => { setCheckAddr(e.target.value); setCheckResult(null); }}
            placeholder="G..."
          />
        </div>
        <button
          className="btn btn-outline"
          disabled={loading || !checkAddr}
          onClick={handleCheck}
          aria-disabled={loading || !checkAddr}
        >
          Check
        </button>
        {checkResult !== null && (
          <p style={{ marginTop: "0.75rem", fontSize: "0.875rem" }}>
            {checkAddr} is{" "}
            <span className={`badge ${checkResult ? "badge-valid" : "badge-revoked"}`}>
              {checkResult ? "a registered issuer" : "not an issuer"}
            </span>
          </p>
        )}
      </div>
    </div>
  );
}
