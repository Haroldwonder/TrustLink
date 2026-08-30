import { describe, it, expect, vi } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { useGlobalStats, GlobalStats } from "../useGlobalStats";

const mockStats: GlobalStats = {
  total_attestations: 1500,
  total_revocations: 42,
  total_issuers: 7,
};

describe("useGlobalStats", () => {
  it("returns loading=true initially", () => {
    const fetchStats = vi.fn(() => new Promise<GlobalStats>(() => {}));
    const { result } = renderHook(() => useGlobalStats(fetchStats));
    expect(result.current.loading).toBe(true);
    expect(result.current.data).toBeNull();
    expect(result.current.error).toBeNull();
  });

  it("returns data on successful fetch", async () => {
    const fetchStats = vi.fn().mockResolvedValue(mockStats);
    const { result } = renderHook(() => useGlobalStats(fetchStats));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.data).toEqual(mockStats);
    expect(result.current.error).toBeNull();
  });

  it("populates all three stat fields correctly", async () => {
    const fetchStats = vi.fn().mockResolvedValue(mockStats);
    const { result } = renderHook(() => useGlobalStats(fetchStats));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.data?.total_attestations).toBe(1500);
    expect(result.current.data?.total_revocations).toBe(42);
    expect(result.current.data?.total_issuers).toBe(7);
  });

  it("returns error on failed fetch", async () => {
    const fetchStats = vi.fn().mockRejectedValue(new Error("network error"));
    const { result } = renderHook(() => useGlobalStats(fetchStats));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.data).toBeNull();
    expect(result.current.error).toBeInstanceOf(Error);
    expect(result.current.error?.message).toBe("network error");
  });

  it("wraps non-Error rejections in an Error", async () => {
    const fetchStats = vi.fn().mockRejectedValue("string error");
    const { result } = renderHook(() => useGlobalStats(fetchStats));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.error).toBeInstanceOf(Error);
    expect(result.current.error?.message).toBe("string error");
  });

  it("re-fetches when fetchStats reference changes", async () => {
    const fetchStats1 = vi.fn().mockResolvedValue(mockStats);
    const fetchStats2 = vi.fn().mockResolvedValue({
      ...mockStats,
      total_attestations: 2000,
    });

    const { result, rerender } = renderHook(
      ({ fn }: { fn: () => Promise<GlobalStats> }) => useGlobalStats(fn),
      { initialProps: { fn: fetchStats1 } }
    );
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.data?.total_attestations).toBe(1500);

    rerender({ fn: fetchStats2 });
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.data?.total_attestations).toBe(2000);
    expect(fetchStats2).toHaveBeenCalledTimes(1);
  });

  it("ignores stale response if hook unmounts before fetch resolves", async () => {
    let resolve!: (v: GlobalStats) => void;
    const fetchStats = vi.fn(
      () => new Promise<GlobalStats>((res) => { resolve = res; })
    );
    const { result, unmount } = renderHook(() => useGlobalStats(fetchStats));
    expect(result.current.loading).toBe(true);
    unmount();
    // Resolve after unmount — should not cause state update or error
    resolve(mockStats);
    // No assertion to make (just must not throw)
  });

  it("sets loading back to false after error", async () => {
    const fetchStats = vi.fn().mockRejectedValue(new Error("fail"));
    const { result } = renderHook(() => useGlobalStats(fetchStats));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.error).not.toBeNull();
  });
});
