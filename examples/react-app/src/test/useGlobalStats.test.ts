import { renderHook, act, waitFor } from "@testing-library/react";
import { vi, describe, it, expect, beforeEach, afterEach } from "vitest";
import { useGlobalStats } from "../hooks/useGlobalStats";
import * as contract from "../contract";

vi.mock("../contract", () => ({
  getGlobalStats: vi.fn(),
  registerIssuer: vi.fn(),
  removeIssuer: vi.fn(),
  isIssuer: vi.fn(),
}));

const mockStats = {
  total_attestations: 42n,
  total_revocations: 5n,
  total_issuers: 3n,
};

describe("useGlobalStats", () => {
  afterEach(() => {
    vi.clearAllMocks();
    vi.useRealTimers();
  });

  it("starts with loading=true and no data", () => {
    vi.mocked(contract.getGlobalStats).mockResolvedValue(mockStats);
    const { result } = renderHook(() => useGlobalStats());
    expect(result.current.loading).toBe(true);
    expect(result.current.data).toBeNull();
    expect(result.current.error).toBeNull();
  });

  it("fetches and returns data on mount", async () => {
    vi.mocked(contract.getGlobalStats).mockResolvedValue(mockStats);
    const { result } = renderHook(() => useGlobalStats());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.data).toEqual(mockStats);
    expect(result.current.error).toBeNull();
  });

  it("sets error state when fetch fails", async () => {
    vi.mocked(contract.getGlobalStats).mockRejectedValue(new Error("network error"));
    const { result } = renderHook(() => useGlobalStats());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.error).toBe("network error");
    expect(result.current.data).toBeNull();
  });

  it("polls at the specified interval", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const updatedStats = { total_attestations: 50n, total_revocations: 6n, total_issuers: 4n };
    vi.mocked(contract.getGlobalStats)
      .mockResolvedValueOnce(mockStats)
      .mockResolvedValueOnce(updatedStats);

    const { result } = renderHook(() => useGlobalStats(5000));

    // Wait for initial fetch using real-time resolution
    await act(async () => {
      await Promise.resolve();
    });
    await waitFor(() => expect(result.current.data).toEqual(mockStats));

    await act(async () => {
      vi.advanceTimersByTime(5000);
      await Promise.resolve();
    });
    await waitFor(() => expect(result.current.data).toEqual(updatedStats));
    expect(contract.getGlobalStats).toHaveBeenCalledTimes(2);
  });

  it("clears interval on unmount", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    vi.mocked(contract.getGlobalStats).mockResolvedValue(mockStats);

    const { unmount } = renderHook(() => useGlobalStats(5000));

    await act(async () => {
      await Promise.resolve();
    });

    unmount();
    vi.clearAllMocks();

    await act(async () => {
      vi.advanceTimersByTime(10000);
      await Promise.resolve();
    });
    expect(contract.getGlobalStats).not.toHaveBeenCalled();
  });

  it("does not poll when no interval is provided", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    vi.mocked(contract.getGlobalStats).mockResolvedValue(mockStats);

    const { result } = renderHook(() => useGlobalStats());

    await act(async () => {
      await Promise.resolve();
    });
    await waitFor(() => expect(result.current.loading).toBe(false));

    vi.clearAllMocks();
    await act(async () => {
      vi.advanceTimersByTime(60000);
      await Promise.resolve();
    });
    expect(contract.getGlobalStats).not.toHaveBeenCalled();
  });

  it("returns updated data across polling cycles", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const stats2 = { total_attestations: 100n, total_revocations: 10n, total_issuers: 5n };
    const stats3 = { total_attestations: 150n, total_revocations: 15n, total_issuers: 6n };
    vi.mocked(contract.getGlobalStats)
      .mockResolvedValueOnce(mockStats)
      .mockResolvedValueOnce(stats2)
      .mockResolvedValueOnce(stats3);

    const { result } = renderHook(() => useGlobalStats(3000));

    await act(async () => { await Promise.resolve(); });
    await waitFor(() => expect(result.current.data).toEqual(mockStats));

    await act(async () => {
      vi.advanceTimersByTime(3000);
      await Promise.resolve();
    });
    await waitFor(() => expect(result.current.data).toEqual(stats2));

    await act(async () => {
      vi.advanceTimersByTime(3000);
      await Promise.resolve();
    });
    await waitFor(() => expect(result.current.data).toEqual(stats3));
  });
});
