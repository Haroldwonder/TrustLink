"""Tests for the retry/backoff helpers in trustlink._retry."""

import pytest

from trustlink._retry import with_retry, with_retry_async


class TestWithRetry:
    """Tests for the sync with_retry helper."""

    def test_retry_then_succeed(self, monkeypatch):
        """A transient error followed by success should retry and return the result."""
        monkeypatch.setattr("trustlink._retry.time.sleep", lambda _: None)
        calls = {"n": 0}

        def flaky():
            calls["n"] += 1
            if calls["n"] < 3:
                raise ConnectionError("boom")
            return "ok"

        result = with_retry(flaky, max_attempts=5)
        assert result == "ok"
        assert calls["n"] == 3

    def test_retry_exhausted(self, monkeypatch):
        """Exhausting all attempts should raise the last transient error."""
        monkeypatch.setattr("trustlink._retry.time.sleep", lambda _: None)
        calls = {"n": 0}

        def always_fails():
            calls["n"] += 1
            raise TimeoutError("still failing")

        with pytest.raises(TimeoutError):
            with_retry(always_fails, max_attempts=3)
        assert calls["n"] == 3

    def test_non_transient_error_propagates_immediately(self, monkeypatch):
        """Non-transient errors should not be retried."""
        monkeypatch.setattr("trustlink._retry.time.sleep", lambda _: None)
        calls = {"n": 0}

        def bad_value():
            calls["n"] += 1
            raise ValueError("not transient")

        with pytest.raises(ValueError):
            with_retry(bad_value, max_attempts=5)
        assert calls["n"] == 1


class TestWithRetryAsync:
    """Tests for the async with_retry_async helper."""

    @pytest.mark.asyncio
    async def test_retry_then_succeed(self, monkeypatch):
        monkeypatch.setattr("trustlink._retry.asyncio.sleep", _async_noop)
        calls = {"n": 0}

        async def flaky():
            calls["n"] += 1
            if calls["n"] < 3:
                raise ConnectionError("boom")
            return "ok"

        result = await with_retry_async(flaky, max_attempts=5)
        assert result == "ok"
        assert calls["n"] == 3

    @pytest.mark.asyncio
    async def test_retry_exhausted(self, monkeypatch):
        monkeypatch.setattr("trustlink._retry.asyncio.sleep", _async_noop)
        calls = {"n": 0}

        async def always_fails():
            calls["n"] += 1
            raise TimeoutError("still failing")

        with pytest.raises(TimeoutError):
            await with_retry_async(always_fails, max_attempts=3)
        assert calls["n"] == 3

    @pytest.mark.asyncio
    async def test_non_transient_error_propagates_immediately(self, monkeypatch):
        monkeypatch.setattr("trustlink._retry.asyncio.sleep", _async_noop)
        calls = {"n": 0}

        async def bad_value():
            calls["n"] += 1
            raise ValueError("not transient")

        with pytest.raises(ValueError):
            await with_retry_async(bad_value, max_attempts=5)
        assert calls["n"] == 1


async def _async_noop(_delay):
    return None
