import { describe, it, expect, vi } from "vitest";
import { CircuitBreaker, withRetry } from "../resilience";

describe("CircuitBreaker", () => {
  it("starts closed", () => {
    const breaker = new CircuitBreaker();
    expect(breaker.getState()).toBe("closed");
    expect(breaker.isOpen()).toBe(false);
  });

  it("opens after the failure threshold is reached", () => {
    const breaker = new CircuitBreaker({ failureThreshold: 2 });
    breaker.recordFailure();
    expect(breaker.isOpen()).toBe(false);
    breaker.recordFailure();
    expect(breaker.isOpen()).toBe(true);
    expect(breaker.getState()).toBe("open");
  });

  it("resets to closed on success", () => {
    const breaker = new CircuitBreaker({ failureThreshold: 1 });
    breaker.recordFailure();
    expect(breaker.isOpen()).toBe(true);
    breaker.recordSuccess();
    expect(breaker.isOpen()).toBe(false);
    expect(breaker.getState()).toBe("closed");
  });

  it("moves to half-open after the reset timeout elapses", () => {
    const breaker = new CircuitBreaker({ failureThreshold: 1, resetTimeoutMs: 10 });
    breaker.recordFailure();
    expect(breaker.isOpen()).toBe(true);
    return new Promise<void>((resolve) => {
      setTimeout(() => {
        expect(breaker.isOpen()).toBe(false);
        expect(breaker.getState()).toBe("half-open");
        resolve();
      }, 20);
    });
  });
});

describe("withRetry", () => {
  it("returns the result on first success", async () => {
    const fn = vi.fn().mockResolvedValue("ok");
    const result = await withRetry(fn, { maxAttempts: 3, initialDelayMs: 1 });
    expect(result).toBe("ok");
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it("retries on failure and eventually succeeds", async () => {
    const fn = vi
      .fn()
      .mockRejectedValueOnce(new Error("fail once"))
      .mockResolvedValue("ok");
    const result = await withRetry(fn, { maxAttempts: 3, initialDelayMs: 1 });
    expect(result).toBe("ok");
    expect(fn).toHaveBeenCalledTimes(2);
  });

  it("throws the last error after exhausting attempts", async () => {
    const fn = vi.fn().mockRejectedValue(new Error("always fails"));
    await expect(
      withRetry(fn, { maxAttempts: 2, initialDelayMs: 1 })
    ).rejects.toThrow("always fails");
    expect(fn).toHaveBeenCalledTimes(2);
  });

  it("rejects immediately when the circuit breaker is open", async () => {
    const breaker = new CircuitBreaker({ failureThreshold: 1 });
    breaker.recordFailure();
    const fn = vi.fn().mockResolvedValue("ok");
    await expect(withRetry(fn, {}, breaker)).rejects.toThrow(
      "Circuit breaker is open"
    );
    expect(fn).not.toHaveBeenCalled();
  });
});
