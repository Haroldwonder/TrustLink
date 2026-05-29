import { useState, useEffect, useCallback, useRef } from "react";
import { TrustLinkClient } from "@trustlink/sdk";
import type { TrustLinkClientOptions, Attestation } from "@trustlink/sdk";

// ── shared hook state shape ───────────────────────────────────────────────────

interface AsyncState<T> {
  data: T | null;
  loading: boolean;
  error: Error | null;
  refetch: () => void;
}

function useAsync<T>(fn: () => Promise<T>, deps: unknown[]): AsyncState<T> {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const counter = useRef(0);

  const run = useCallback(() => {
    const id = ++counter.current;
    setLoading(true);
    setError(null);
    fn()
      .then((result) => { if (id === counter.current) { setData(result); setLoading(false); } })
      .catch((err) => { if (id === counter.current) { setError(err instanceof Error ? err : new Error(String(err))); setLoading(false); } });
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);

  useEffect(() => { run(); }, [run]);

  return { data, loading, error, refetch: run };
}

// ── useTrustLink ──────────────────────────────────────────────────────────────

/**
 * Returns a stable TrustLinkClient instance for the given contract.
 * Re-creates the client only when contractId or network options change.
 */
export function useTrustLink(options: TrustLinkClientOptions): TrustLinkClient {
  const ref = useRef<{ client: TrustLinkClient; key: string } | null>(null);
  const key = `${options.contractId}:${options.network}:${options.rpcUrl ?? ""}`;

  if (!ref.current || ref.current.key !== key) {
    ref.current = { client: new TrustLinkClient(options), key };
  }

  return ref.current.client;
}

// ── useHasValidClaim ──────────────────────────────────────────────────────────

/**
 * Checks whether `subject` holds a valid claim of `claimType`.
 */
export function useHasValidClaim(
  client: TrustLinkClient,
  subject: string,
  claimType: string
): AsyncState<boolean> {
  return useAsync(
    () => client.hasValidClaim(subject, claimType),
    [client, subject, claimType]
  );
}

// ── useSubjectAttestations ────────────────────────────────────────────────────

/**
 * Fetches all attestations for `subject` (up to `limit`, default 50).
 */
export function useSubjectAttestations(
  client: TrustLinkClient,
  subject: string,
  { start = 0, limit = 50 }: { start?: number; limit?: number } = {}
): AsyncState<Attestation[]> {
  return useAsync(
    () => client.getSubjectAttestations(subject, start, limit),
    [client, subject, start, limit]
  );
}
