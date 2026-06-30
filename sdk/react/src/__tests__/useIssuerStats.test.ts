import { describe, it, expect, vi } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { useIssuerStats, IssuerStats } from "../src/useIssuerStats";

const mockStats: IssuerStats = {
  total_issued: 10,
  active: 7,
  revoked: 2,
  expired: 1,
};

describe("useIssuerStats", () => {
  it("returns loading=true initially", () => {
    const fetchStats = vi.fn(() => new Promise<IssuerStats>(() => {}));
    const { result } = renderHook(() => useIssuerStats("GISSUER", fetchStats));
    expect(result.current.loading).toBe(true);
    expect(result.current.data).toBeNull();
    expect(result.current.error).toBeNull();
  });

  it("returns data on successful fetch", async () => {
    const fetchStats = vi.fn().mockResolvedValue(mockStats);
    const { result } = renderHook(() => useIssuerStats("GISSUER", fetchStats));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.data).toEqual(mockStats);
    expect(result.current.error).toBeNull();
  });

  it("returns error on failed fetch", async () => {
    const fetchStats = vi.fn().mockRejectedValue(new Error("network error"));
    const { result } = renderHook(() => useIssuerStats("GISSUER", fetchStats));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.data).toBeNull();
    expect(result.current.error).toBeInstanceOf(Error);
    expect(result.current.error?.message).toBe("network error");
  });

  it("wraps non-Error rejections in an Error", async () => {
    const fetchStats = vi.fn().mockRejectedValue("string error");
    const { result } = renderHook(() => useIssuerStats("GISSUER", fetchStats));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.error).toBeInstanceOf(Error);
  });

  it("calls fetchStats with the issuer address", async () => {
    const fetchStats = vi.fn().mockResolvedValue(mockStats);
    renderHook(() => useIssuerStats("GTEST123", fetchStats));
    await waitFor(() => expect(fetchStats).toHaveBeenCalledWith("GTEST123"));
  });

  it("re-fetches when issuer changes", async () => {
    const fetchStats = vi.fn().mockResolvedValue(mockStats);
    const { result, rerender } = renderHook(
      ({ issuer }: { issuer: string }) => useIssuerStats(issuer, fetchStats),
      { initialProps: { issuer: "GISSUER1" } }
    );
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(fetchStats).toHaveBeenCalledTimes(1);

    rerender({ issuer: "GISSUER2" });
    await waitFor(() => expect(fetchStats).toHaveBeenCalledTimes(2));
    expect(fetchStats).toHaveBeenLastCalledWith("GISSUER2");
  });
});
