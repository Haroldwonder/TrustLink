import { useState, useCallback, useEffect, useRef } from "react";
import { AttestationEvent } from "./hooks/useAttestationSubscription";

export interface Toast {
  id: number;
  message: string;
  type: "created" | "revoked";
}

let _nextId = 0;

export function useToasts() {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const timers = useRef<Map<number, ReturnType<typeof setTimeout>>>(new Map());

  const dismiss = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
    const timer = timers.current.get(id);
    if (timer) { clearTimeout(timer); timers.current.delete(id); }
  }, []);

  const push = useCallback((event: AttestationEvent) => {
    const id = _nextId++;
    const type = event.type === "attestation_created" ? "created" : "revoked";
    const action = type === "created" ? "created" : "revoked";
    const message = `Attestation ${action}: ${event.claim_type} (${event.id.slice(0, 8)}…)`;
    setToasts((prev) => [...prev.slice(-4), { id, message, type }]);
    const timer = setTimeout(() => dismiss(id), 5000);
    timers.current.set(id, timer);
  }, [dismiss]);

  useEffect(() => {
    const map = timers.current;
    return () => { map.forEach(clearTimeout); };
  }, []);

  return { toasts, push, dismiss };
}

export function ToastContainer({ toasts, onDismiss }: { toasts: Toast[]; onDismiss: (id: number) => void }) {
  return (
    <div style={{
      position: "fixed",
      bottom: "1.5rem",
      right: "1.5rem",
      display: "flex",
      flexDirection: "column",
      gap: "0.5rem",
      zIndex: 1000,
      pointerEvents: "none",
    }}>
      {toasts.map((t) => (
        <div
          key={t.id}
          style={{
            background: t.type === "created" ? "#052e16" : "#450a0a",
            border: `1px solid ${t.type === "created" ? "#16a34a" : "#dc2626"}`,
            borderRadius: "0.5rem",
            padding: "0.75rem 1rem",
            color: "#e2e8f0",
            fontSize: "0.875rem",
            display: "flex",
            alignItems: "center",
            gap: "0.75rem",
            pointerEvents: "all",
            minWidth: "280px",
            maxWidth: "360px",
          }}
        >
          <span style={{ flex: 1 }}>{t.message}</span>
          <button
            onClick={() => onDismiss(t.id)}
            style={{ background: "none", border: "none", color: "#94a3b8", cursor: "pointer", fontSize: "1rem", lineHeight: 1 }}
            aria-label="Dismiss"
          >
            ×
          </button>
        </div>
      ))}
    </div>
  );
}
