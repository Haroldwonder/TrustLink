"""
Retry helpers for TrustLink Python bindings.

Provides :func:`with_retry` (sync) and :func:`with_retry_async` (async) which
wrap a callable with exponential-backoff retry logic.  Both helpers are opt-out:
callers who prefer to handle retries themselves can pass ``max_attempts=1``.
"""

from __future__ import annotations

import asyncio
import functools
import random
import time
from typing import Any, Callable, Coroutine, Optional, TypeVar

F = TypeVar("F", bound=Callable[..., Any])
AF = TypeVar("AF", bound=Callable[..., Coroutine[Any, Any, Any]])

# Errors that indicate a transient RPC/network problem and are safe to retry.
# Any other exception propagates immediately.
_TRANSIENT_ERRORS = (
    ConnectionError,
    TimeoutError,
    OSError,
)

try:
    # stellar_sdk raises these on HTTP-level failures
    from stellar_sdk.exceptions import BaseRequestError as _StellarRequestError

    _TRANSIENT_ERRORS = _TRANSIENT_ERRORS + (_StellarRequestError,)  # type: ignore[assignment]
except ImportError:
    pass


def _is_transient(exc: BaseException) -> bool:
    """Return True if *exc* is a transient error that is safe to retry."""
    if isinstance(exc, _TRANSIENT_ERRORS):
        return True
    # Catch SDK errors by message substring when we can't import the class
    msg = str(exc).lower()
    return any(
        token in msg
        for token in ("timeout", "connection", "network", "reset by peer", "broken pipe")
    )


def _backoff_delay(attempt: int, base_ms: float, max_ms: float, jitter: float) -> float:
    """Return the sleep duration (seconds) for *attempt* (1-based)."""
    base = base_ms * (2 ** (attempt - 1))
    capped = min(base, max_ms)
    jitter_ms = capped * jitter * random.random()
    return (capped + jitter_ms) / 1000.0


def with_retry(
    fn: Callable[..., Any],
    *args: Any,
    max_attempts: int = 3,
    base_ms: float = 200.0,
    max_ms: float = 10_000.0,
    jitter: float = 0.2,
    **kwargs: Any,
) -> Any:
    """Call ``fn(*args, **kwargs)`` with exponential-backoff retry.

    Only transient errors (network timeouts, connection resets, etc.) trigger a
    retry.  Non-transient errors (contract errors, auth failures, …) propagate
    immediately on the first attempt.

    Args:
        fn:           Callable to invoke.
        *args:        Positional arguments forwarded to *fn*.
        max_attempts: Maximum number of total attempts (default: 3).
        base_ms:      Initial backoff delay in milliseconds (default: 200).
        max_ms:       Maximum backoff cap in milliseconds (default: 10 000).
        jitter:       Random jitter factor 0–1 applied to the capped delay
                      (default: 0.2).
        **kwargs:     Keyword arguments forwarded to *fn*.

    Returns:
        The return value of *fn* on success.

    Raises:
        The last exception raised by *fn* after all attempts are exhausted.
    """
    last_exc: Optional[BaseException] = None
    for attempt in range(1, max_attempts + 1):
        try:
            return fn(*args, **kwargs)
        except Exception as exc:  # noqa: BLE001
            if not _is_transient(exc):
                raise
            last_exc = exc
            if attempt == max_attempts:
                break
            delay = _backoff_delay(attempt, base_ms, max_ms, jitter)
            time.sleep(delay)
    raise last_exc  # type: ignore[misc]


async def with_retry_async(
    fn: Callable[..., Coroutine[Any, Any, Any]],
    *args: Any,
    max_attempts: int = 3,
    base_ms: float = 200.0,
    max_ms: float = 10_000.0,
    jitter: float = 0.2,
    **kwargs: Any,
) -> Any:
    """Async version of :func:`with_retry`.

    Awaits ``fn(*args, **kwargs)`` with exponential-backoff retry.
    Uses :func:`asyncio.sleep` between attempts so the event loop is not
    blocked.

    Args:
        fn:           Async callable to invoke.
        *args:        Positional arguments forwarded to *fn*.
        max_attempts: Maximum number of total attempts (default: 3).
        base_ms:      Initial backoff delay in milliseconds (default: 200).
        max_ms:       Maximum backoff cap in milliseconds (default: 10 000).
        jitter:       Random jitter factor 0–1 applied to the capped delay
                      (default: 0.2).
        **kwargs:     Keyword arguments forwarded to *fn*.

    Returns:
        The return value of *fn* on success.

    Raises:
        The last exception raised by *fn* after all attempts are exhausted.
    """
    last_exc: Optional[BaseException] = None
    for attempt in range(1, max_attempts + 1):
        try:
            return await fn(*args, **kwargs)
        except Exception as exc:  # noqa: BLE001
            if not _is_transient(exc):
                raise
            last_exc = exc
            if attempt == max_attempts:
                break
            delay = _backoff_delay(attempt, base_ms, max_ms, jitter)
            await asyncio.sleep(delay)
    raise last_exc  # type: ignore[misc]


def retryable(
    max_attempts: int = 3,
    base_ms: float = 200.0,
    max_ms: float = 10_000.0,
    jitter: float = 0.2,
) -> Callable[[F], F]:
    """Decorator that wraps a **sync** method with :func:`with_retry`.

    Apply to read-only methods on sync clients::

        @retryable(max_attempts=3)
        def has_valid_claim(self, subject: str, claim_type: str) -> bool:
            ...

    The decorated method accepts an additional ``retry_attempts`` keyword
    argument that callers can pass to override ``max_attempts`` per call.
    Pass ``retry_attempts=1`` to opt out of retries entirely.
    """

    def decorator(fn: F) -> F:
        @functools.wraps(fn)
        def wrapper(*args: Any, **kwargs: Any) -> Any:
            attempts = kwargs.pop("retry_attempts", max_attempts)
            return with_retry(
                fn,
                *args,
                max_attempts=attempts,
                base_ms=base_ms,
                max_ms=max_ms,
                jitter=jitter,
                **kwargs,
            )

        return wrapper  # type: ignore[return-value]

    return decorator


def retryable_async(
    max_attempts: int = 3,
    base_ms: float = 200.0,
    max_ms: float = 10_000.0,
    jitter: float = 0.2,
) -> Callable[[AF], AF]:
    """Decorator that wraps an **async** method with :func:`with_retry_async`.

    Apply to read-only methods on async clients::

        @retryable_async(max_attempts=3)
        async def has_valid_claim(self, subject: str, claim_type: str) -> bool:
            ...

    The decorated method accepts an additional ``retry_attempts`` keyword
    argument that callers can pass to override ``max_attempts`` per call.
    Pass ``retry_attempts=1`` to opt out of retries entirely.
    """

    def decorator(fn: AF) -> AF:
        @functools.wraps(fn)
        async def wrapper(*args: Any, **kwargs: Any) -> Any:
            attempts = kwargs.pop("retry_attempts", max_attempts)
            return await with_retry_async(
                fn,
                *args,
                max_attempts=attempts,
                base_ms=base_ms,
                max_ms=max_ms,
                jitter=jitter,
                **kwargs,
            )

        return wrapper  # type: ignore[return-value]

    return decorator
