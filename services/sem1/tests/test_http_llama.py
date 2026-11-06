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
        try:
            self.wfile.write(raw)
        except BrokenPipeError:
            # The client's own budget retired first — the answer this
            # stub was late with has nobody to reach. That is the case
            # some of these pins exist to prove, so it is not an error
            # here; it is the measurement landing.
            pass

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


class _BlackHole:
    """Accepts the connection and never answers — the endpoint that is
    up but whose answer never comes. This is the shape KI-452 was
    measured against: no timeout short of the client's own retires it."""

    def __init__(self):
        self.sock = socket.socket()
        self.sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self.sock.bind(("127.0.0.1", 0))
        self.sock.listen(8)
        self.port = self.sock.getsockname()[1]
        self.conns = []
        threading.Thread(target=self._accept, daemon=True).start()

    def _accept(self):
        while True:
            try:
                conn, _ = self.sock.accept()
            except OSError:
                return
            self.conns.append(conn)

    def close(self):
        self.sock.close()
        for conn in self.conns:
            conn.close()


def test_a_deadline_bounds_the_chain_the_caller_is_paying_for():
    """KI-452: a per-attempt timeout composes into a chain far longer
    than a caller's budget for the CALL. `deadline` is that budget in
    the unit the caller metered — and the chain must stop when it is
    spent, not when its own worst case is.

    Measured, not asserted from the formula: a 3.0 s per-attempt
    timeout with a 1.5 s deadline answers in ~1.5 s, and the error
    names both numbers so an operator can tell "the caller's budget
    ran out" apart from "the endpoint is busy"."""
    hole = _BlackHole()
    try:
        started = time.monotonic()
        try:
            http_llama.embed(
                ["x"],
                endpoint=f"http://127.0.0.1:{hole.port}",
                timeout=3.0,
                deadline=1.5,
            )
        except ProviderUnavailableError as error:
            message = str(error)
            elapsed = time.monotonic() - started
            assert elapsed < 3.0, (
                f"the deadline did not bound the chain: {elapsed:.1f}s spent "
                f"on a 1.5s deadline (the chain's own worst case is "
                f"{http_llama.worst_case_seconds(3.0):.0f}s)"
            )
            assert "caller's 1.5s deadline" in message, message
            # The reason names the chain's own cost beside the deadline:
            # the two numbers an operator needs to size a budget rather
            # than guess at it. And it must NOT read "up but busy" —
            # that invites a retry towards a caller that has already
            # given up.
            assert "retry chain alone can spend 13s" in message, message
            assert "up but busy" not in message, message
        else:
            raise AssertionError("a black-holed endpoint must not answer")
    finally:
        hole.close()


def test_the_named_worst_case_is_the_chain_it_describes():
    """`worst_case_seconds` exists so a caller can size a budget to the
    mechanism. A number that is not derived from the constants would
    be a comment dressed as arithmetic, so this drives the REAL chain
    with no deadline and checks the named figure is what it spent."""
    server, port = _serve(delay=5.0, slow=99)
    per_attempt = 0.2
    try:
        started = time.monotonic()
        try:
            http_llama.embed(
                ["x"], endpoint=f"http://127.0.0.1:{port}", timeout=per_attempt
            )
        except ProviderUnavailableError:
            pass
        else:
            raise AssertionError("a saturated endpoint must eventually raise")
        spent = time.monotonic() - started
        named = http_llama.worst_case_seconds(per_attempt)
        # The chain spends its attempts plus its backoff and nothing
        # more; a tenth of headroom absorbs the stub's own scheduling.
        assert spent <= named + 0.5, (
            f"the chain spent {spent:.1f}s against a named worst case of "
            f"{named:.1f}s — the figure is not the mechanism"
        )
        # And the figure is the composition, not a literal that happens
        # to fit: 3 attempts + 1 s + 3 s of backoff.
        assert named == per_attempt * (http_llama.RETRIES + 1) + sum(
            http_llama.BACKOFF_SECONDS
        ), named
    finally:
        server.shutdown()


if __name__ == "__main__":
    test_a_refused_endpoint_is_absent_not_busy()
    print("ok: a refused endpoint is absent, not busy")
    test_a_timeout_is_busy_retried_and_never_told_to_start()
    print("ok: a timeout is busy, retried, and never told to start")
    test_busy_then_answer_lands_on_the_retry()
    print("ok: busy then answer lands on the retry")
    test_a_deadline_bounds_the_chain_the_caller_is_paying_for()
    print("ok: a deadline bounds the chain the caller is paying for")
    test_the_named_worst_case_is_the_chain_it_describes()
    print("ok: the named worst case is the chain it describes")
    print("all passed")
