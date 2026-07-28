import { useState, useEffect } from "react";

export interface GlobalStats {
  total_attestations: number;
  total_revocations: number;
  total_issuers: number;
}

export interface UseGlobalStatsResult {
  data: GlobalStats | null;
  loading: boolean;
  error: Error | null;
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
 * @example
 * ```tsx
 * const { data, loading, error } = useGlobalStats(
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

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    fetchStats()
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
  }, [fetchStats]);

  return { data, loading, error };
}
