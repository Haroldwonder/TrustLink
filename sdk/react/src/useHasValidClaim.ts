import { useCallback, useEffect, useRef, useState } from "react";

/**
 * Options for {@link useHasValidClaim}.
 */
export interface UseHasValidClaimOptions {
  /**
   * When `false`, the hook will not fetch and `data` stays `null`.
   * Defaults to `true`.
   */
  enabled?: boolean;
  /**
   * When set to a positive number, the hook will re-fetch the claim status
   * on that interval (in milliseconds). The interval is cleared on unmount
   * and whenever the inputs change.
   */
  pollIntervalMs?: number;
}

/**
 * Result returned by {@link useHasValidClaim}.
 */
export interface UseHasValidClaimResult {
  /**
   * Whether the subject currently holds a valid claim of the requested type.
   * `null` until the first successful fetch (or while disabled).
   */
  data: boolean | null;
  /** `true` while a fetch is in flight. */
  loading: boolean;
  /** The last error encountered, if any. */
  error: Error | null;
  /** Manually trigger a re-fetch. */
  refetch: () => void;
}

/**
 * Fetcher signature used by {@link useHasValidClaim}. It should resolve to
 * `true` when `subject` holds a valid claim of `claimType`, `false` otherwise.
 */
export type HasValidClaimFetcher = (
  subject: string,
  claimType: string,
) => Promise<boolean>;

/**
 * React hook that reports whether `subject` holds a valid claim of
 * `claimType`.
 *
 * The fetcher is kept in a ref so that passing an inline arrow function does
 * not cause a refetch loop; the effect only depends on the primitive inputs
 * (`subject`, `claimType`, `options.enabled`, `options.pollIntervalMs`) and an
 * internal `refetchCount`.
 *
 * @param subject - The address (or identifier) whose claims are checked.
 * @param claimType - The claim type to check for.
 * @param fetcher - Function that performs the lookup against the client.
 * @param options - Optional configuration (see {@link UseHasValidClaimOptions}).
 * @returns The current claim status, loading/error state, and a `refetch`.
 */
export function useHasValidClaim(
  subject: string,
  claimType: string,
  fetcher: HasValidClaimFetcher,
  options: UseHasValidClaimOptions = {},
): UseHasValidClaimResult {
  const { enabled = true, pollIntervalMs } = options;

  const [data, setData] = useState<boolean | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<Error | null>(null);
  const [refetchCount, setRefetchCount] = useState<number>(0);

  // Keep the latest fetcher in a ref so inline arrows don't retrigger effects.
  const fetcherRef = useRef<HasValidClaimFetcher>(fetcher);
  useEffect(() => {
    fetcherRef.current = fetcher;
  }, [fetcher]);

  const refetch = useCallback(() => {
    setRefetchCount((count) => count + 1);
  }, []);

  useEffect(() => {
    if (!enabled) {
      return;
    }

    let cancelled = false;

    const run = async () => {
      setLoading(true);
      setError(null);
      try {
        const result = await fetcherRef.current(subject, claimType);
        if (!cancelled) {
          setData(result);
        }
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err : new Error(String(err)));
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    };

    void run();

    let intervalId: ReturnType<typeof setInterval> | undefined;
    if (typeof pollIntervalMs === "number" && pollIntervalMs > 0) {
      intervalId = setInterval(() => {
        void run();
      }, pollIntervalMs);
    }

    return () => {
      cancelled = true;
      if (intervalId !== undefined) {
        clearInterval(intervalId);
      }
    };
  }, [subject, claimType, enabled, pollIntervalMs, refetchCount]);

  return { data, loading, error, refetch };
}