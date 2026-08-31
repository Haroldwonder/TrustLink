"""Tests for retry/backoff logic in trustlink._retry (Issue #1212)."""

import pytest

from trustlink._retry import with_retry, with_retry_async


class TestWithRetry:
    """Tests for the sync with_retry helper."""

    def test_retry_then_succeed(self):
        calls = {"count": 0}

        def flaky():
            calls["count"] += 1
            if calls["count"] < 3:
                raise ConnectionError("transient")
            return "ok"

        result = with_retry(flaky, max_attempts=5, base_ms=1, max_ms=1)
        assert result == "ok"
        assert calls["count"] == 3

    def test_retry_exhausted(self):
        calls = {"count": 0}

        def always_fails():
            calls["count"] += 1
            raise TimeoutError("still failing")

        with pytest.raises(TimeoutError):
            with_retry(always_fails, max_attempts=3, base_ms=1, max_ms=1)
        assert calls["count"] == 3

    def test_non_transient_error_propagates_immediately(self):
        calls = {"count": 0}

        def bad_input():
            calls["count"] += 1
            raise ValueError("not transient")

        with pytest.raises(ValueError):
            with_retry(bad_input, max_attempts=5, base_ms=1, max_ms=1)
        assert calls["count"] == 1


class TestWithRetryAsync:
    """Tests for the async with_retry_async helper."""

    @pytest.mark.asyncio
    async def test_retry_then_succeed(self):
        calls = {"count": 0}

        async def flaky():
            calls["count"] += 1
            if calls["count"] < 3:
                raise ConnectionError("transient")
            return "ok"

        result = await with_retry_async(flaky, max_attempts=5, base_ms=1, max_ms=1)
        assert result == "ok"
        assert calls["count"] == 3

    @pytest.mark.asyncio
    async def test_retry_exhausted(self):
        calls = {"count": 0}

        async def always_fails():
            calls["count"] += 1
            raise TimeoutError("still failing")

        with pytest.raises(TimeoutError):
            await with_retry_async(always_fails, max_attempts=3, base_ms=1, max_ms=1)
        assert calls["count"] == 3

    @pytest.mark.asyncio
    async def test_non_transient_error_propagates_immediately(self):
        calls = {"count": 0}

        async def bad_input():
            calls["count"] += 1
            raise ValueError("not transient")

        with pytest.raises(ValueError):
            await with_retry_async(bad_input, max_attempts=5, base_ms=1, max_ms=1)
        assert calls["count"] == 1
