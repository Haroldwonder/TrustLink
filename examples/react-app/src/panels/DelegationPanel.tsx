import { useState } from "react";
import {
  delegateClaimType,
  revokeDelegation,
  createAttestationAsDelegate,
  Delegation,
} from "../contract";

interface Props {
  address: string;
}

export default function DelegationPanel({ address }: Props) {
  const [tab, setTab] = useState<"grant" | "revoke" | "attest" | "active">("grant");
  const [status, setStatus] = useState<{ type: "success" | "error"; msg: string } | null>(null);
  const [loading, setLoading] = useState(false);

  // Grant form
  const [grantDelegate, setGrantDelegate] = useState("");
  const [grantClaimType, setGrantClaimType] = useState("");
  const [grantExpiration, setGrantExpiration] = useState("");

  // Revoke form
  const [revokeDelegate, setRevokeDelegate] = useState("");
  const [revokeClaimType, setRevokeClaimType] = useState("");

  // Attest-as-delegate form
  const [delegator, setDelegator] = useState("");
  const [attestSubject, setAttestSubject] = useState("");
  const [attestClaimType, setAttestClaimType] = useState("");
  const [attestMetadata, setAttestMetadata] = useState("");
  const [attestExpiration, setAttestExpiration] = useState("");

  // Local tracking of granted delegations (since contract has no list endpoint)
  const [activeDelegations, setActiveDelegations] = useState<Delegation[]>([]);

  function setOk(msg: string) { setStatus({ type: "success", msg }); }
  function setErr(e: unknown) { setStatus({ type: "error", msg: (e as Error).message }); }

  async function handleGrant() {
    if (!grantDelegate || !grantClaimType) return;
    setLoading(true); setStatus(null);
    try {
      const exp = grantExpiration ? BigInt(grantExpiration) : null;
      await delegateClaimType(address, grantDelegate.trim(), grantClaimType.trim(), exp);
      const entry: Delegation = {
        delegator: address,
        delegate: grantDelegate.trim(),
        claim_type: grantClaimType.trim(),
        expiration: exp,
      };
      setActiveDelegations((prev) => {
        const deduped = prev.filter(
          (d) => !(d.delegate === entry.delegate && d.claim_type === entry.claim_type)
        );
        return [...deduped, entry];
      });
      setOk(`Delegation granted to ${grantDelegate.slice(0, 8)}…`);
      setGrantDelegate(""); setGrantClaimType(""); setGrantExpiration("");
    } catch (e) { setErr(e); }
    finally { setLoading(false); }
  }

  async function handleRevoke() {
    if (!revokeDelegate || !revokeClaimType) return;
    setLoading(true); setStatus(null);
    try {
      await revokeDelegation(address, revokeDelegate.trim(), revokeClaimType.trim());
      setActiveDelegations((prev) =>
        prev.filter((d) => !(d.delegate === revokeDelegate.trim() && d.claim_type === revokeClaimType.trim()))
      );
      setOk("Delegation revoked.");
      setRevokeDelegate(""); setRevokeClaimType("");
    } catch (e) { setErr(e); }
    finally { setLoading(false); }
  }

  async function handleAttest() {
    if (!delegator || !attestSubject || !attestClaimType) return;
    setLoading(true); setStatus(null);
    try {
      const exp = attestExpiration ? BigInt(attestExpiration) : null;
      await createAttestationAsDelegate(
        address,
        delegator.trim(),
        attestSubject.trim(),
        attestClaimType.trim(),
        exp,
        attestMetadata || null
      );
      setOk("Attestation created on behalf of delegator.");
      setDelegator(""); setAttestSubject(""); setAttestClaimType(""); setAttestMetadata(""); setAttestExpiration("");
    } catch (e) { setErr(e); }
    finally { setLoading(false); }
  }

  const TABS = [
    { id: "grant" as const, label: "Grant" },
    { id: "revoke" as const, label: "Revoke" },
    { id: "attest" as const, label: "Attest as Delegate" },
    { id: "active" as const, label: "Active" },
  ];

  return (
    <div className="panel">
      <h2>Delegation</h2>
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

      {tab === "grant" && (
        <div className="card">
          <h3>Grant Delegation</h3>
          <p style={{ fontSize: "0.85rem", color: "#94a3b8", marginBottom: "0.75rem" }}>
            Authorise a sub-issuer (delegate) to create attestations of a specific claim type on your behalf.
          </p>
          <div className="field">
            <label>Delegate Address</label>
            <input value={grantDelegate} onChange={(e) => setGrantDelegate(e.target.value)} placeholder="G..." />
          </div>
          <div className="field">
            <label>Claim Type</label>
            <input value={grantClaimType} onChange={(e) => setGrantClaimType(e.target.value)} placeholder="KYC_PASSED, AML_CLEARED…" />
          </div>
          <div className="field">
            <label>Expiration (unix seconds, optional)</label>
            <input
              value={grantExpiration}
              onChange={(e) => setGrantExpiration(e.target.value)}
              placeholder="leave blank for no expiration"
              type="number"
            />
          </div>
          <button
            className="btn btn-primary"
            disabled={loading || !grantDelegate || !grantClaimType}
            onClick={handleGrant}
          >
            {loading ? "Granting…" : "Grant"}
          </button>
        </div>
      )}

      {tab === "revoke" && (
        <div className="card">
          <h3>Revoke Delegation</h3>
          <p style={{ fontSize: "0.85rem", color: "#94a3b8", marginBottom: "0.75rem" }}>
            Remove a delegate's authority to issue a specific claim type on your behalf.
          </p>
          <div className="field">
            <label>Delegate Address</label>
            <input value={revokeDelegate} onChange={(e) => setRevokeDelegate(e.target.value)} placeholder="G..." />
          </div>
          <div className="field">
            <label>Claim Type</label>
            <input value={revokeClaimType} onChange={(e) => setRevokeClaimType(e.target.value)} placeholder="KYC_PASSED, AML_CLEARED…" />
          </div>
          <button
            className="btn btn-danger"
            disabled={loading || !revokeDelegate || !revokeClaimType}
            onClick={handleRevoke}
          >
            {loading ? "Revoking…" : "Revoke"}
          </button>
        </div>
      )}

      {tab === "attest" && (
        <div className="card">
          <h3>Create Attestation as Delegate</h3>
          <p style={{ fontSize: "0.85rem", color: "#94a3b8", marginBottom: "0.75rem" }}>
            Issue an attestation on behalf of an issuer who has delegated this claim type to you.
          </p>
          <div className="field">
            <label>Delegator (issuer who delegated to you)</label>
            <input value={delegator} onChange={(e) => setDelegator(e.target.value)} placeholder="G..." />
          </div>
          <div className="field">
            <label>Subject Address</label>
            <input value={attestSubject} onChange={(e) => setAttestSubject(e.target.value)} placeholder="G..." />
          </div>
          <div className="field">
            <label>Claim Type</label>
            <input value={attestClaimType} onChange={(e) => setAttestClaimType(e.target.value)} placeholder="KYC_PASSED…" />
          </div>
          <div className="field">
            <label>Metadata (optional)</label>
            <input value={attestMetadata} onChange={(e) => setAttestMetadata(e.target.value)} placeholder="optional note" />
          </div>
          <div className="field">
            <label>Expiration (unix seconds, optional)</label>
            <input
              value={attestExpiration}
              onChange={(e) => setAttestExpiration(e.target.value)}
              placeholder="leave blank for no expiration"
              type="number"
            />
          </div>
          <button
            className="btn btn-primary"
            disabled={loading || !delegator || !attestSubject || !attestClaimType}
            onClick={handleAttest}
          >
            {loading ? "Attesting…" : "Create Attestation"}
          </button>
        </div>
      )}

      {tab === "active" && (
        <div className="card">
          <h3>Active Delegations</h3>
          <p style={{ fontSize: "0.85rem", color: "#94a3b8", marginBottom: "0.75rem" }}>
            Delegations granted during this session. Use Grant / Revoke tabs to manage them.
          </p>
          {activeDelegations.length === 0 ? (
            <p className="empty">No delegations granted in this session.</p>
          ) : (
            <div className="att-list">
              {activeDelegations.map((d) => (
                <div key={`${d.delegate}-${d.claim_type}`} className="att-item">
                  <div className="row">
                    <span className="claim">{d.claim_type}</span>
                    <span className="badge badge-valid">Active</span>
                  </div>
                  <span className="meta">Delegate: {d.delegate}</span>
                  {d.expiration != null && (
                    <span className="meta">
                      Expires: {new Date(Number(d.expiration) * 1000).toLocaleDateString()}
                    </span>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
