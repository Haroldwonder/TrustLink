import { useState, useEffect } from "react";
import { getAdmin, getAdminCouncil, addAdmin, removeAdmin } from "../contract";

interface Props { address: string; }

export default function CouncilPanel({ address }: Props) {
  const [members, setMembers] = useState<string[]>([]);
  const [primaryAdmin, setPrimaryAdmin] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState(false);
  const [targetAddr, setTargetAddr] = useState("");
  const [status, setStatus] = useState<{ type: "success" | "error"; msg: string } | null>(null);

  useEffect(() => {
    async function load() {
      setLoading(true);
      try {
        const [admin, council] = await Promise.all([
          getAdmin(),
          getAdminCouncil().catch(() => [] as string[]),
        ]);
        setPrimaryAdmin(admin);
        setMembers(council.length > 0 ? council : [admin]);
      } catch (e: unknown) {
        setStatus({ type: "error", msg: (e as Error).message });
      } finally {
        setLoading(false);
      }
    }
    load();
  }, []);

  async function handleAdd() {
    if (!targetAddr.trim()) return;
    setActionLoading(true);
    setStatus(null);
    try {
      await addAdmin(address, targetAddr.trim());
      setStatus({ type: "success", msg: `${targetAddr} added to council.` });
      setMembers((prev) => [...prev, targetAddr.trim()]);
      setTargetAddr("");
    } catch (e: unknown) {
      setStatus({ type: "error", msg: (e as Error).message });
    } finally {
      setActionLoading(false);
    }
  }

  async function handleRemove() {
    if (!targetAddr.trim()) return;
    setActionLoading(true);
    setStatus(null);
    try {
      await removeAdmin(address, targetAddr.trim());
      setStatus({ type: "success", msg: `${targetAddr} removed from council.` });
      setMembers((prev) => prev.filter((m) => m !== targetAddr.trim()));
      setTargetAddr("");
    } catch (e: unknown) {
      setStatus({ type: "error", msg: (e as Error).message });
    } finally {
      setActionLoading(false);
    }
  }

  return (
    <div className="panel">
      <h2>Council Panel</h2>

      {status && <div className={`alert alert-${status.type}`}>{status.msg}</div>}

      <div className="card">
        <h3>Council Members</h3>
        {loading && <p className="empty">Loading…</p>}
        {!loading && members.length === 0 && <p className="empty">No council members found.</p>}
        {!loading && members.length > 0 && (
          <ul style={{ listStyle: "none", padding: 0, margin: 0 }}>
            {members.map((m) => (
              <li
                key={m}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "0.5rem",
                  padding: "0.4rem 0",
                  borderBottom: "1px solid #2d3148",
                  fontSize: "0.875rem",
                  wordBreak: "break-all",
                }}
              >
                <span style={{ color: "#94a3b8", flexShrink: 0 }}>◆</span>
                <span>{m}</span>
                {m === primaryAdmin && (
                  <span className="badge badge-valid" style={{ marginLeft: "auto", flexShrink: 0 }}>
                    Primary
                  </span>
                )}
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="card">
        <h3>Manage Council Members</h3>
        <p style={{ fontSize: "0.8rem", color: "#94a3b8", marginBottom: "0.75rem" }}>
          Only existing council members can add or remove others.
        </p>
        <div className="field">
          <label>Member Address</label>
          <input
            value={targetAddr}
            onChange={(e) => setTargetAddr(e.target.value)}
            placeholder="G..."
          />
        </div>
        <div style={{ display: "flex", gap: "0.5rem" }}>
          <button
            className="btn btn-primary"
            disabled={actionLoading || !targetAddr.trim()}
            onClick={handleAdd}
          >
            Add Member
          </button>
          <button
            className="btn btn-danger"
            disabled={actionLoading || !targetAddr.trim()}
            onClick={handleRemove}
          >
            Remove Member
          </button>
        </div>
      </div>
    </div>
  );
}
