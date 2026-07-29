import { useState, useEffect } from "react";

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
}

/**
 * Fetches issuer statistics for the given issuer address.
 *
 * @param issuer - The issuer address to query.
 * @param fetchStats - Async function that retrieves IssuerStats for the given address.
 */
export function useIssuerStats(
  issuer: string,
  fetchStats: (issuer: string) => Promise<IssuerStats>
): UseIssuerStatsResult {
  const [data, setData] = useState<IssuerStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    fetchStats(issuer)
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
  }, [issuer, fetchStats]);

  return { data, loading, error };
}
