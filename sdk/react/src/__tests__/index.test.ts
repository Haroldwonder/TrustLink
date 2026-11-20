import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import * as sdkReact from "../index";
import { useHasValidClaim } from "../index";

describe("sdk/react/src/index exports", () => {
  it("exports useIssuerStats", () => {
    expect(typeof sdkReact.useIssuerStats).toBe("function");
  });

  it("exports useGlobalStats", () => {
    expect(typeof sdkReact.useGlobalStats).toBe("function");
  });

  it("exports useHasValidClaim", () => {
    expect(typeof sdkReact.useHasValidClaim).toBe("function");
  });
});

describe("useHasValidClaim", () => {
  const subject = "GABC123";
  const claimType = "kyc";

  beforeEach(() => {
    vi.useRealTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("loads data on mount and clears loading", async () => {
    const fetcher = vi.fn().mockResolvedValue(true);

    const { result } = renderHook(() =>
      useHasValidClaim(subject, claimType, { fetcher }),
    );

    expect(result.current.loading).toBe(true);
    expect(result.current.data).toBeUndefined();

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.data).toBe(true);
    expect(result.current.error).toBeUndefined();
    expect(fetcher).toHaveBeenCalledTimes(1);
    expect(fetcher).toHaveBeenCalledWith(subject, claimType);
  });

  it("populates error state on rejection", async () => {
    const failure = new Error("boom");
    const fetcher = vi.fn().mockRejectedValue(failure);

    const { result } = renderHook(() =>
      useHasValidClaim(subject, claimType, { fetcher }),
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error).toBe(failure);
    expect(result.current.data).toBeUndefined();
  });

  it("refetch re-invokes the fetcher", async () => {
    const fetcher = vi
      .fn()
      .mockResolvedValueOnce(true)
      .mockResolvedValueOnce(false);

    const { result } = renderHook(() =>
      useHasValidClaim(subject, claimType, { fetcher }),
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });
    expect(result.current.data).toBe(true);
    expect(fetcher).toHaveBeenCalledTimes(1);

    await act(async () => {
      await result.current.refetch();
    });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.data).toBe(false);
    expect(fetcher).toHaveBeenCalledTimes(2);
  });

  it("polls the fetcher repeatedly when pollIntervalMs is set", async () => {
    vi.useFakeTimers();
    const fetcher = vi.fn().mockResolvedValue(true);

    const { result } = renderHook(() =>
      useHasValidClaim(subject, claimType, { fetcher, pollIntervalMs: 1000 }),
    );

    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(fetcher).toHaveBeenCalledTimes(1);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000);
    });
    expect(fetcher).toHaveBeenCalledTimes(2);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000);
    });
    expect(fetcher).toHaveBeenCalledTimes(3);

    expect(result.current.data).toBe(true);
  });

  it("skips the fetch when enabled is false", async () => {
    const fetcher = vi.fn().mockResolvedValue(true);

    const { result } = renderHook(() =>
      useHasValidClaim(subject, claimType, { fetcher, enabled: false }),
    );

    await act(async () => {
      await Promise.resolve();
    });

    expect(fetcher).not.toHaveBeenCalled();
    expect(result.current.loading).toBe(false);
    expect(result.current.data).toBeUndefined();
  });
});