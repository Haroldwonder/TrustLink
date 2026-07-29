import { useState, useEffect, useRef } from "react";
import { getGlobalStats, GlobalStats } from "../contract";

interface UseGlobalStatsResult {
  data: GlobalStats | null;
  loading: boolean;
  error: string | null;
}

export function useGlobalStats(pollingIntervalMs?: number): UseGlobalStatsResult {
  const [data, setData] = useState<GlobalStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;

    async function fetchStats() {
      try {
        const stats = await getGlobalStats();
        if (mountedRef.current) {
          setData(stats);
          setError(null);
        }
      } catch (e: unknown) {
        if (mountedRef.current) {
          setError((e as Error).message);
        }
      } finally {
        if (mountedRef.current) {
          setLoading(false);
        }
      }
    }

    fetchStats();

    if (pollingIntervalMs && pollingIntervalMs > 0) {
      const id = setInterval(fetchStats, pollingIntervalMs);
      return () => {
        mountedRef.current = false;
        clearInterval(id);
      };
    }

    return () => {
      mountedRef.current = false;
    };
  }, [pollingIntervalMs]);

  return { data, loading, error };
}
