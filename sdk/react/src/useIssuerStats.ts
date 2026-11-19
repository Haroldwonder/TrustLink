import { useState, useEffect, useRef, useCallback } from "react";

export interface IssuerStats {
  total_issued: number;
  active: number;
  revoked: number;
  expired: number;
}

export interface UseIssuerStatsResult {
  data: IssuerStats | null;
  loading: boolean;
  error: Error | null;
  /** Re-runs the fetch. Stable across renders. */
  refetch: () => void;
}

/**
 * Fetches issuer statistics for the given issuer address.
 *
 * @param issuer - The issuer address to query.
 * @param fetchStats - Async function that retrieves IssuerStats for the given address.
 *
 * `fetchStats` is held in a ref rather than listed as an effect dependency, so
 * passing an inline arrow — the usual call pattern — does not re-trigger the
 * fetch on every render. Depending on its identity caused an unbounded fetch
 * loop (Issue #1321). The fetch re-runs when `issuer` changes, or on `refetch`.
 *
 * @example
 * ```tsx
 * const { data, loading, error } = useIssuerStats(
 *   issuerAddress,
 *   (issuer) => trustlinkClient.getIssuerStats(issuer)
 * );
 * ```
 */
export function useIssuerStats(
  issuer: string,
  fetchStats: (issuer: string) => Promise<IssuerStats>
): UseIssuerStatsResult {
  const [data, setData] = useState<IssuerStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [refetchCount, setRefetchCount] = useState(0);

  // See the note on the JSDoc above: keeping the caller's fetcher in a ref lets
  // the effect always call the latest closure without depending on its
  // identity, which an inline arrow changes on every render (Issue #1321).
  const fetchStatsRef = useRef(fetchStats);
  fetchStatsRef.current = fetchStats;

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    fetchStatsRef
      .current(issuer)
      .then((stats) => {
        if (!cancelled) {
          setData(stats);
          setLoading(false);
        }
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setError(err instanceof Error ? err : new Error(String(err)));
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
    // `issuer` is a genuine input and stays a dependency; `fetchStats` does not.
  }, [issuer, refetchCount]);

  const refetch = useCallback(() => {
    setRefetchCount((count) => count + 1);
  }, []);

  return { data, loading, error, refetch };
}
