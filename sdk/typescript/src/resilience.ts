/**
 * Resilience utilities: exponential backoff retry and circuit breaker.
 */

export interface RetryOptions {
  /** Maximum number of attempts (default: 3) */
  maxAttempts?: number;
  /** Initial delay in ms (default: 200) */
  initialDelayMs?: number;
  /** Maximum delay cap in ms (default: 10_000) */
  maxDelayMs?: number;
  /** Jitter factor 0–1 (default: 0.2) */
  jitter?: number;
  /** Connection timeout in ms applied per attempt (default: 30_000) */
  timeoutMs?: number;
}

export interface CircuitBreakerOptions {
  /** Failures before opening the circuit (default: 5) */
  failureThreshold?: number;
  /** Ms to wait before trying half-open (default: 30_000) */
  resetTimeoutMs?: number;
}

type CircuitState = "closed" | "open" | "half-open";

export class CircuitBreaker {
  private state: CircuitState = "closed";
  private failures = 0;
  private lastFailureTime = 0;

  private readonly failureThreshold: number;
  private readonly resetTimeoutMs: number;

  constructor(opts: CircuitBreakerOptions = {}) {
    this.failureThreshold = opts.failureThreshold ?? 5;
    this.resetTimeoutMs = opts.resetTimeoutMs ?? 30_000;
  }

  isOpen(): boolean {
    if (this.state === "open") {
      if (Date.now() - this.lastFailureTime >= this.resetTimeoutMs) {
        this.state = "half-open";
        return false;
      }
      return true;
    }
    return false;
  }

  recordSuccess(): void {
    this.failures = 0;
    this.state = "closed";
  }

  recordFailure(): void {
    this.failures++;
    this.lastFailureTime = Date.now();
    if (this.failures >= this.failureThreshold) {
      this.state = "open";
    }
  }

  getState(): CircuitState {
    return this.state;
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}

function withTimeout<T>(promise: Promise<T>, ms: number): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const timer = setTimeout(
      () => reject(new Error(`Operation timed out after ${ms}ms`)),
      ms
    );
    promise.then(
      (v) => { clearTimeout(timer); resolve(v); },
      (e) => { clearTimeout(timer); reject(e); }
    );
  });
}

/**
 * Execute `fn` with exponential backoff retry and optional circuit breaker.
 */
export async function withRetry<T>(
  fn: () => Promise<T>,
  retryOpts: RetryOptions = {},
  breaker?: CircuitBreaker
): Promise<T> {
  const maxAttempts = retryOpts.maxAttempts ?? 3;
  const initialDelayMs = retryOpts.initialDelayMs ?? 200;
  const maxDelayMs = retryOpts.maxDelayMs ?? 10_000;
  const jitter = retryOpts.jitter ?? 0.2;
  const timeoutMs = retryOpts.timeoutMs ?? 30_000;

  if (breaker?.isOpen()) {
    throw new Error("Circuit breaker is open — request rejected");
  }

  let lastError: unknown;

  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      const result = await withTimeout(fn(), timeoutMs);
      breaker?.recordSuccess();
      return result;
    } catch (err) {
      lastError = err;
      breaker?.recordFailure();

      if (attempt === maxAttempts) break;

      const base = initialDelayMs * Math.pow(2, attempt - 1);
      const capped = Math.min(base, maxDelayMs);
      const jitterMs = capped * jitter * Math.random();
      await sleep(capped + jitterMs);
    }
  }

  throw lastError;
}
