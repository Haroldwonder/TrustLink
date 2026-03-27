import { useState } from "react";
import { registerIssuer, removeIssuer, isIssuer } from "../contract";

interface Props { address: string; }

export default function AdminPanel({ address }: Props) {
  const [issuerAddr, setIssuerAddr] = useState("");
  const [status, setStatus] = useState<{ type: "success" | "error" | "info"; msg: string } | null>(null);
  const [loading, setLoading] = useState(false);
  const [checkAddr, setCheckAddr] = useState("");
  const [checkResult, setCheckResult] = useState<boolean | null>(null);

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

      {status && <div className={`alert alert-${status.type}`}>{status.msg}</div>}

      <div className="card">
        <h3>Register / Remove Issuer</h3>
        <div className="field">
          <label>Issuer Address</label>
          <input
            value={issuerAddr}
            onChange={(e) => setIssuerAddr(e.target.value)}
            placeholder="G..."
          />
        </div>
        <div style={{ display: "flex", gap: "0.5rem" }}>
          <button className="btn btn-primary" disabled={loading || !issuerAddr} onClick={() => handle("register")}>
            Register
          </button>
          <button className="btn btn-danger" disabled={loading || !issuerAddr} onClick={() => handle("remove")}>
            Remove
          </button>
        </div>
      </div>

      <div className="card">
        <h3>Check Issuer Status</h3>
        <div className="field">
          <label>Address to check</label>
          <input
            value={checkAddr}
            onChange={(e) => { setCheckAddr(e.target.value); setCheckResult(null); }}
            placeholder="G..."
          />
        </div>
        <button className="btn btn-outline" disabled={loading || !checkAddr} onClick={handleCheck}>
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
