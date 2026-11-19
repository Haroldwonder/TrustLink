import { useState, useEffect, useRef, useCallback } from "react";

export interface GlobalStats {
  total_attestations: number;
  total_revocations: number;
  total_issuers: number;
}

export interface UseGlobalStatsResult {
  data: GlobalStats | null;
  loading: boolean;
  error: Error | null;
  /** Re-runs the fetch. Stable across renders. */
  refetch: () => void;
}

/**
 * Fetches contract-wide global statistics.
 *
 * Mirrors the `get_global_stats` contract function which returns:
 *   - total_attestations: cumulative count of all attestations ever created
 *   - total_revocations:  cumulative count of all revocations
 *   - total_issuers:      current number of registered issuers
 *
 * @param fetchStats - Async function that retrieves GlobalStats (no arguments;
 *   supply a bound or arrow function that calls your RPC client).
 *
 * The inline arrow below is deliberate and safe: `fetchStats` is held in a ref
 * rather than listed as an effect dependency, so a new function reference on
 * every render does not re-trigger the fetch. Listing it in the deps caused an
 * unbounded fetch loop with exactly this documented usage (Issue #1321).
 *
 * @example
 * ```tsx
 * const { data, loading, error, refetch } = useGlobalStats(
 *   () => trustlinkClient.getGlobalStats()
 * );
 * ```
 */
export function useGlobalStats(
  fetchStats: () => Promise<GlobalStats>
): UseGlobalStatsResult {
  const [data, setData] = useState<GlobalStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  // Bumped by `refetch` to re-run the effect on demand, since `fetchStats` is
  // no longer a dependency that could do it.
  const [refetchCount, setRefetchCount] = useState(0);

  // The caller's fetcher is kept in a ref and updated on every render, so the
  // effect always calls the latest closure without depending on its identity.
  // A `useCallback`-less inline arrow — which is what the documented usage and
  // most real call sites pass — is a new reference each render, and depending on
  // it re-ran the fetch on every render including the ones its own setState
  // caused (Issue #1321).
  const fetchStatsRef = useRef(fetchStats);
  fetchStatsRef.current = fetchStats;

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    fetchStatsRef
      .current()
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
    // Intentionally empty apart from `refetchCount`: fetch once on mount, then
    // only when `refetch` is called. See the ref comment above.
  }, [refetchCount]);

  const refetch = useCallback(() => {
    setRefetchCount((count) => count + 1);
  }, []);

  return { data, loading, error, refetch };
}
