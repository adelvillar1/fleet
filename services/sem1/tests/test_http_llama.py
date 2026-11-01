"""KI-346/354's pin: the embed arm splits its errors BY CAUSE.

A timeout is an endpoint that is UP and BUSY (a concurrent embed owns
the one slot) — answered with `up but busy`, retried with bounded
backoff, and never with the Start-it line; a refused connection is an
endpoint that is ABSENT — the unreachable message with its Start-it
line, untried. The row's symptom: a live server with days of uptime
was told to start itself.

Run: python3 services/sem1/tests/test_http_llama.py
"""

import http.server
import json
import socket
import sys
import threading
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

from sem1.providers import http_llama  # noqa: E402
from sem1.types import ProviderUnavailableError  # noqa: E402


def _free_port() -> int:
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        return probe.getsockname()[1]


class _Handler(http.server.BaseHTTPRequestHandler):
    """A stub /v1/embeddings: sleeps `delay` seconds per request the
    first `slow` times, then answers; counts every hit."""

    delay = 0.0
    slow = 0
    hits = 0
    lock = threading.Lock()

    def do_POST(self):  # noqa: N802 - stdlib shape
        length = int(self.headers.get("content-length") or 0)
        self.rfile.read(length)
        with self.lock:
            _Handler.hits += 1
            slow_this = _Handler.hits <= _Handler.slow
        if slow_this and _Handler.delay:
            time.sleep(_Handler.delay)
        payload = {
            "model": "stub",
            "data": [{"embedding": [0.0, 0.0, 1.0]}],
            "usage": {"prompt_tokens": 1},
        }
        raw = json.dumps(payload).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(raw)))
        self.end_headers()
        self.wfile.write(raw)

    def log_message(self, *args):  # keep the test output clean
        pass


def _serve(delay: float, slow: int) -> tuple[http.server.ThreadingHTTPServer, int]:
    _Handler.delay = delay
    _Handler.slow = slow
    _Handler.hits = 0
    port = _free_port()
    server = http.server.ThreadingHTTPServer(("127.0.0.1", port), _Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server, port


def test_a_refused_endpoint_is_absent_not_busy():
    """A closed port: the ABSENT case — unreachable with the Start-it
    line, and no retry spends time on it."""
    started = time.monotonic()
    try:
        http_llama.embed(["x"], endpoint=f"http://127.0.0.1:{_free_port()}", timeout=2)
    except http_llama.ProviderBusyError:
        raise AssertionError("a refused connection is absent, not busy")
    except ProviderUnavailableError as error:
        message = str(error)
        assert "unreachable" in message, message
        assert "Start it" in message, message
        assert "up but busy" not in message, message
    else:
        raise AssertionError("a refused endpoint must raise")
    # No bounded backoff spends its seconds on an absent endpoint.
    assert time.monotonic() - started < 2.0, "the absent path retried"


def test_a_timeout_is_busy_retried_and_never_told_to_start():
    """A server that holds every answer past the budget: the UP-BUSY
    case — `up but busy`, RETRIES+1 attempts, and no Start-it line
    (KI-354's symptom, pinned by its absence)."""
    server, port = _serve(delay=3.0, slow=99)
    try:
        started = time.monotonic()
        try:
            http_llama.embed(["x"], endpoint=f"http://127.0.0.1:{port}", timeout=0.3)
        except ProviderUnavailableError as error:
            message = str(error)
            assert isinstance(error, http_llama.ProviderBusyError), message
            assert "up but busy" in message, message
            assert "did not answer within 0.3s" in message, message
            assert "Start it" not in message, message
        else:
            raise AssertionError("a saturated endpoint must eventually raise")
        elapsed = time.monotonic() - started
        assert elapsed >= 4.0, f"the bounded backoff ran its course in {elapsed}s"
    finally:
        server.shutdown()


def test_busy_then_answer_lands_on_the_retry():
    """The queue drains between attempts: attempt one times out,
    the backoff sleeps, attempt two answers — the search returns."""
    server, port = _serve(delay=1.2, slow=1)
    try:
        result = http_llama.embed(["x"], endpoint=f"http://127.0.0.1:{port}", timeout=0.3)
        assert len(result.vectors) == 1, result
        assert _Handler.hits == 2, f"one timeout, one answer: {_Handler.hits} hits"
    finally:
        server.shutdown()


if __name__ == "__main__":
    test_a_refused_endpoint_is_absent_not_busy()
    print("ok: a refused endpoint is absent, not busy")
    test_a_timeout_is_busy_retried_and_never_told_to_start()
    print("ok: a timeout is busy, retried, and never told to start")
    test_busy_then_answer_lands_on_the_retry()
    print("ok: busy then answer lands on the retry")
    print("all passed")
