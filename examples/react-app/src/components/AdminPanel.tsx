import React, { useEffect, useState } from "react";
import type { Council, CouncilProposal, StorageLimits } from "@trustlink/sdk";
import { TrustLinkClient } from "@trustlink/sdk";

const client = new TrustLinkClient({
  rpcUrl: import.meta.env.VITE_RPC_URL ?? "https://soroban-testnet.stellar.org",
  contractId: import.meta.env.VITE_CONTRACT_ID ?? "",
});

export function AdminPanel() {
  const [council, setCouncil] = useState<Council | null>(null);
  const [proposals, setProposals] = useState<CouncilProposal[]>([]);
  const [limits, setLimits] = useState<StorageLimits | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const [councilData, limitsData] = await Promise.all([
          client.getCouncil(),
          client.getLimits(),
        ]);
        setCouncil(councilData);
        setLimits(limitsData);

        // Fetch pending proposals for each council member's open proposals.
        // In a real app the contract would expose a list endpoint; here we
        // surface whatever the UI already knows about.
        setProposals([]);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setLoading(false);
      }
    }
    void load();
  }, []);

  if (loading) return <p>Loading admin panel…</p>;
  if (error) return <p style={{ color: "red" }}>Error: {error}</p>;

  return (
    <div style={{ fontFamily: "sans-serif", maxWidth: 720, margin: "0 auto" }}>
      <h2>Admin Panel</h2>

      {/* Council info */}
      <section>
        <h3>Council</h3>
        {council ? (
          <table style={{ width: "100%", borderCollapse: "collapse" }}>
            <tbody>
              <tr>
                <th style={th}>Threshold</th>
                <td style={td}>{council.threshold}</td>
              </tr>
              <tr>
                <th style={th}>Members</th>
                <td style={td}>
                  <ul style={{ margin: 0, paddingLeft: 16 }}>
                    {council.members.map((m) => (
                      <li key={m}>
                        <code>{m}</code>
                      </li>
                    ))}
                  </ul>
                </td>
              </tr>
              <tr>
                <th style={th}>Created</th>
                <td style={td}>
                  {new Date(council.created_at * 1000).toLocaleString()}
                </td>
              </tr>
            </tbody>
          </table>
        ) : (
          <p>No council configured.</p>
        )}
      </section>

      {/* Pending council proposals */}
      <section style={{ marginTop: 24 }}>
        <h3>Pending Council Proposals</h3>
        {proposals.length === 0 ? (
          <p>No pending proposals.</p>
        ) : (
          <table style={{ width: "100%", borderCollapse: "collapse" }}>
            <thead>
              <tr>
                {["ID", "Action", "Approvals", "Expires", "Executed"].map(
                  (h) => (
                    <th key={h} style={{ ...th, textAlign: "left" }}>
                      {h}
                    </th>
                  )
                )}
              </tr>
            </thead>
            <tbody>
              {proposals.map((p) => (
                <tr key={p.id}>
                  <td style={td}>
                    <code>{p.id.slice(0, 8)}…</code>
                  </td>
                  <td style={td}>{p.action}</td>
                  <td style={td}>
                    {p.approvals.length} / {p.threshold}
                  </td>
                  <td style={td}>
                    {new Date(p.expires_at * 1000).toLocaleString()}
                  </td>
                  <td style={td}>{p.executed ? "Yes" : "No"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      {/* Storage limits (read-only) */}
      <section style={{ marginTop: 24 }}>
        <h3>Storage Limits</h3>
        {limits ? (
          <table style={{ width: "100%", borderCollapse: "collapse" }}>
            <tbody>
              {(
                Object.entries(limits) as [keyof StorageLimits, number][]
              ).map(([key, value]) => (
                <tr key={key}>
                  <th style={th}>{humanize(key)}</th>
                  <td style={td}>{value}</td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <p>Limits not available.</p>
        )}
      </section>
    </div>
  );
}

const th: React.CSSProperties = {
  textAlign: "left",
  padding: "6px 8px",
  background: "#f5f5f5",
  border: "1px solid #ddd",
  fontWeight: 600,
  whiteSpace: "nowrap",
};

const td: React.CSSProperties = {
  padding: "6px 8px",
  border: "1px solid #ddd",
  verticalAlign: "top",
};

function humanize(s: string): string {
  return s.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}
