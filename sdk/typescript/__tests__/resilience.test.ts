import { describe, it, expect, jest } from "@jest/globals";
import { withRetry, CircuitBreaker } from "../src/resilience";

const fastOpts = { initialDelayMs: 1, maxDelayMs: 2, jitter: 0 };

describe("withRetry", () => {
  it("returns the result after a failure then success", async () => {
    let attempts = 0;
    const fn = jest.fn(async () => {
      attempts++;
      if (attempts < 2) throw new Error("transient");
      return "ok";
    });

    await expect(withRetry(fn, { ...fastOpts, maxAttempts: 3 })).resolves.toBe("ok");
    expect(fn).toHaveBeenCalledTimes(2);
  });

  it("throws the last error once retries are exhausted", async () => {
    const fn = jest.fn(async () => {
      throw new Error("always fails");
    });

    await expect(
      withRetry(fn, { ...fastOpts, maxAttempts: 3 }),
    ).rejects.toThrow("always fails");
    expect(fn).toHaveBeenCalledTimes(3);
  });

  it("rejects immediately when the circuit breaker is open", async () => {
    const breaker = new CircuitBreaker({ failureThreshold: 1 });
    breaker.recordFailure();
    expect(breaker.getState()).toBe("open");

    const fn = jest.fn(async () => "unused");
    await expect(
      withRetry(fn, { ...fastOpts, maxAttempts: 2 }, breaker),
    ).rejects.toThrow("Circuit breaker is open");
    expect(fn).not.toHaveBeenCalled();
  });

  it("records success on the breaker after recovering", async () => {
    const breaker = new CircuitBreaker({ failureThreshold: 5 });
    let attempts = 0;
    const fn = async () => {
      attempts++;
      if (attempts < 2) throw new Error("transient");
      return 42;
    };

    await expect(
      withRetry(fn, { ...fastOpts, maxAttempts: 3 }, breaker),
    ).resolves.toBe(42);
    expect(breaker.getState()).toBe("closed");
  });
});

describe("CircuitBreaker", () => {
  it("opens after reaching the failure threshold and half-opens after the reset timeout", async () => {
    const breaker = new CircuitBreaker({ failureThreshold: 2, resetTimeoutMs: 5 });
    breaker.recordFailure();
    expect(breaker.isOpen()).toBe(false);
    breaker.recordFailure();
    expect(breaker.getState()).toBe("open");
    expect(breaker.isOpen()).toBe(true);

    await new Promise((r) => setTimeout(r, 10));
    expect(breaker.isOpen()).toBe(false);
    expect(breaker.getState()).toBe("half-open");

    breaker.recordSuccess();
    expect(breaker.getState()).toBe("closed");
  });
});
