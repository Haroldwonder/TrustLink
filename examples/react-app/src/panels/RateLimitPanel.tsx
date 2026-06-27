import { useState, useEffect } from "react";
import { getRateLimit, RateLimit } from "../contract";

interface Props {
  address: string;
}

export const RATE_LIMIT_WARNING_THRESHOLD = 0.8;

export default function RateLimitPanel({ address }: Props) {
  const [rateLimit, setRateLimit] = useState<RateLimit | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    getRateLimit(address)
      .then((rl) => { if (!cancelled) { setRateLimit(rl); setLoading(false); } })
      .catch((e: unknown) => { if (!cancelled) { setError((e as Error).message); setLoading(false); } });
    return () => { cancelled = true; };
  }, [address]);

  if (loading) return <p className="empty" style={{ fontSize: "0.85rem" }}>Loading rate limit…</p>;
  if (error) return null;
  if (!rateLimit) return null;

  const { limit, window_seconds, current_count } = rateLimit;
  const usagePct = limit > 0 ? current_count / limit : 0;
  const nearLimit = usagePct >= RATE_LIMIT_WARNING_THRESHOLD;
  const windowHours = Math.round(window_seconds / 3600);

  const barColor = usagePct >= 0.9 ? "#ef4444" : usagePct >= 0.8 ? "#f59e0b" : "#10b981";

  return (
    <div
      style={{
        background: "#0f1117",
        border: `1px solid ${nearLimit ? "#f59e0b" : "#2d3148"}`,
        borderRadius: "0.5rem",
        padding: "1rem",
        marginBottom: "1.5rem",
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.5rem" }}>
        <span style={{ fontSize: "0.85rem", color: "#94a3b8", fontWeight: 600 }}>Rate Limit</span>
        <span style={{ fontSize: "0.8rem", color: "#64748b" }}>
          per {windowHours}h window
        </span>
      </div>

      <div style={{ display: "flex", alignItems: "baseline", gap: "0.4rem", marginBottom: "0.6rem" }}>
        <span style={{ fontSize: "1.5rem", fontWeight: "bold", color: barColor }}>{current_count}</span>
        <span style={{ fontSize: "0.9rem", color: "#64748b" }}>/ {limit} attestations</span>
      </div>

      <div style={{ background: "#1e2035", borderRadius: "999px", height: "6px", overflow: "hidden" }}>
        <div
          style={{
            width: `${Math.min(usagePct * 100, 100)}%`,
            height: "100%",
            background: barColor,
            borderRadius: "999px",
            transition: "width 0.3s ease",
          }}
        />
      </div>

      {nearLimit && (
        <div className="alert alert-error" style={{ marginTop: "0.75rem", fontSize: "0.82rem", padding: "0.5rem 0.75rem" }}>
          Warning: you have used {Math.round(usagePct * 100)}% of your rate limit for this window.
          Submissions may be rejected.
        </div>
      )}
    </div>
  );
}
