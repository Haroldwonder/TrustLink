import { useState, useEffect } from "react";
import {
  submitAttestationRequest,
  getSubjectRequests,
  getIssuerRequests,
  fulfillRequest,
  rejectRequest,
  AttestationRequest,
  isIssuer,
} from "../contract";

interface Props { address: string; }

export default function AttestationRequestPanel({ address }: Props) {
  const [tab, setTab] = useState<"submit" | "pending" | "manage">("submit");
  const [issuerAddr, setIssuerAddr] = useState("");
  const [claimType, setClaimType] = useState("");
  const [status, setStatus] = useState<{ type: "success" | "error"; msg: string } | null>(null);
  const [loading, setLoading] = useState(false);

  const [subjectRequests, setSubjectRequests] = useState<AttestationRequest[]>([]);
  const [issuerRequests, setIssuerRequests] = useState<AttestationRequest[]>([]);
  const [isUserIssuer, setIsUserIssuer] = useState(false);

  const [fulfillId, setFulfillId] = useState("");
  const [fulfillExpiration, setFulfillExpiration] = useState("");
  const [rejectId, setRejectId] = useState("");
  const [rejectReason, setRejectReason] = useState("");

  useEffect(() => {
    isIssuer(address).then(setIsUserIssuer);
  }, [address]);

  useEffect(() => {
    if (tab === "pending") loadSubjectRequests();
    if (tab === "manage") loadIssuerRequests();
  }, [tab]);

  async function loadSubjectRequests() {
    setLoading(true);
    try {
      const reqs = await getSubjectRequests(address);
      setSubjectRequests(reqs);
    } catch (e: unknown) {
      setStatus({ type: "error", msg: (e as Error).message });
    } finally {
      setLoading(false);
    }
  }

  async function loadIssuerRequests() {
    setLoading(true);
    try {
      const reqs = await getIssuerRequests(address);
      setIssuerRequests(reqs);
    } catch (e: unknown) {
      setStatus({ type: "error", msg: (e as Error).message });
    } finally {
      setLoading(false);
    }
  }

  async function handleSubmit() {
    if (!issuerAddr || !claimType) return;
    setLoading(true);
    setStatus(null);
    try {
      await submitAttestationRequest(address, issuerAddr.trim(), claimType.trim());
      setStatus({ type: "success", msg: "Request submitted." });
      setIssuerAddr("");
      setClaimType("");
    } catch (e: unknown) {
      setStatus({ type: "error", msg: (e as Error).message });
    } finally {
      setLoading(false);
    }
  }

  async function handleFulfill() {
    if (!fulfillId) return;
    setLoading(true);
    setStatus(null);
    try {
      const exp = fulfillExpiration ? BigInt(Math.floor(new Date(fulfillExpiration).getTime() / 1000)) : null;
      await fulfillRequest(address, fulfillId.trim(), exp);
      setStatus({ type: "success", msg: "Request fulfilled." });
      setFulfillId("");
      setFulfillExpiration("");
      await loadIssuerRequests();
    } catch (e: unknown) {
      setStatus({ type: "error", msg: (e as Error).message });
    } finally {
      setLoading(false);
    }
  }

  async function handleReject() {
    if (!rejectId) return;
    setLoading(true);
    setStatus(null);
    try {
      await rejectRequest(address, rejectId.trim(), rejectReason || null);
      setStatus({ type: "success", msg: "Request rejected." });
      setRejectId("");
      setRejectReason("");
      await loadIssuerRequests();
    } catch (e: unknown) {
      setStatus({ type: "error", msg: (e as Error).message });
    } finally {
      setLoading(false);
    }
  }

  function statusBadge(s: string) {
    if (s === "fulfilled") return <span className="badge badge-valid">Fulfilled</span>;
    if (s === "rejected") return <span className="badge badge-revoked">Rejected</span>;
    return <span className="badge">Pending</span>;
  }

  return (
    <div className="panel">
      <h2>Attestation Requests</h2>
      {status && <div className={`alert alert-${status.type}`}>{status.msg}</div>}

      <div style={{ display: "flex", gap: "0.5rem", marginBottom: "1rem", borderBottom: "1px solid #2d3148", paddingBottom: "0.5rem" }}>
        <button
          className={`tab ${tab === "submit" ? "active" : ""}`}
          onClick={() => setTab("submit")}
          style={{ flex: 1, textAlign: "center", padding: "0.5rem" }}
        >
          Submit Request
        </button>
        <button
          className={`tab ${tab === "pending" ? "active" : ""}`}
          onClick={() => setTab("pending")}
          style={{ flex: 1, textAlign: "center", padding: "0.5rem" }}
        >
          My Requests
        </button>
        {isUserIssuer && (
          <button
            className={`tab ${tab === "manage" ? "active" : ""}`}
            onClick={() => setTab("manage")}
            style={{ flex: 1, textAlign: "center", padding: "0.5rem" }}
          >
            Manage Requests
          </button>
        )}
      </div>

      {tab === "submit" && (
        <div className="card">
          <h3>Submit Attestation Request</h3>
          <p style={{ fontSize: "0.8rem", color: "#94a3b8", marginBottom: "1rem" }}>
            Request an attestation from an issuer. They will review and fulfill or reject your request.
          </p>
          <div className="field">
            <label>Issuer Address</label>
            <input
              value={issuerAddr}
              onChange={(e) => setIssuerAddr(e.target.value)}
              placeholder="G..."
            />
          </div>
          <div className="field">
            <label>Claim Type</label>
            <input
              value={claimType}
              onChange={(e) => setClaimType(e.target.value)}
              placeholder="KYC_PASSED, ACCREDITED_INVESTOR, etc."
            />
          </div>
          <button
            className="btn btn-primary"
            disabled={loading || !issuerAddr || !claimType}
            onClick={handleSubmit}
          >
            Submit Request
          </button>
        </div>
      )}

      {tab === "pending" && (
        <div className="card">
          <h3>My Requests</h3>
          {loading && <p className="empty">Loading…</p>}
          {!loading && subjectRequests.length === 0 && (
            <p className="empty">No requests found.</p>
          )}
          <div className="att-list">
            {subjectRequests.map((r) => (
              <div key={r.id} className="att-item">
                <div className="row">
                  <span className="claim">{r.claim_type}</span>
                  {statusBadge(r.status)}
                </div>
                <span className="meta">Issuer: {r.issuer}</span>
                <span className="meta">
                  Submitted: {new Date(Number(r.created_at) * 1000).toLocaleDateString()}
                </span>
                {r.fulfilled_at && (
                  <span className="meta">
                    Fulfilled: {new Date(Number(r.fulfilled_at) * 1000).toLocaleDateString()}
                  </span>
                )}
                <span className="meta">ID: {r.id}</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {tab === "manage" && (
        <div>
          <div className="card">
            <h3>Pending Requests</h3>
            {loading && <p className="empty">Loading…</p>}
            {!loading && issuerRequests.filter((r) => r.status === "pending").length === 0 && (
              <p className="empty">No pending requests.</p>
            )}
            <div className="att-list">
              {issuerRequests
                .filter((r) => r.status === "pending")
                .map((r) => (
                  <div key={r.id} className="att-item">
                    <div className="row">
                      <span className="claim">{r.claim_type}</span>
                      <span className="badge">Pending</span>
                    </div>
                    <span className="meta">Subject: {r.subject}</span>
                    <span className="meta">
                      Submitted: {new Date(Number(r.created_at) * 1000).toLocaleDateString()}
                    </span>
                    <span className="meta">ID: {r.id}</span>
                  </div>
                ))}
            </div>
          </div>

          <div className="card">
            <h3>Fulfill Request</h3>
            <div className="field">
              <label>Request ID</label>
              <input
                value={fulfillId}
                onChange={(e) => setFulfillId(e.target.value)}
                placeholder="request ID"
              />
            </div>
            <div className="field">
              <label>Expiration (optional)</label>
              <input
                type="date"
                value={fulfillExpiration}
                onChange={(e) => setFulfillExpiration(e.target.value)}
              />
            </div>
            <button
              className="btn btn-primary"
              disabled={loading || !fulfillId}
              onClick={handleFulfill}
            >
              Fulfill
            </button>
          </div>

          <div className="card">
            <h3>Reject Request</h3>
            <div className="field">
              <label>Request ID</label>
              <input
                value={rejectId}
                onChange={(e) => setRejectId(e.target.value)}
                placeholder="request ID"
              />
            </div>
            <div className="field">
              <label>Reason (optional)</label>
              <input
                value={rejectReason}
                onChange={(e) => setRejectReason(e.target.value)}
                placeholder="reason for rejection"
              />
            </div>
            <button
              className="btn btn-danger"
              disabled={loading || !rejectId}
              onClick={handleReject}
            >
              Reject
            </button>
          </div>

          <div className="card">
            <h3>All Requests</h3>
            <div className="att-list">
              {issuerRequests.map((r) => (
                <div key={r.id} className="att-item">
                  <div className="row">
                    <span className="claim">{r.claim_type}</span>
                    {statusBadge(r.status)}
                  </div>
                  <span className="meta">Subject: {r.subject}</span>
                  <span className="meta">
                    Submitted: {new Date(Number(r.created_at) * 1000).toLocaleDateString()}
                  </span>
                  <span className="meta">ID: {r.id}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
