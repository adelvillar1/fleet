"""KI-452's pin: the doctor's semantic probe answers inside the budget
of the call that asked it.

The row was filed as a load-sensitive flake — "the doctor call raced a
loaded machine". Measurement says otherwise. The probe underneath the
doctor is `sem1.probe("llama-server")`, and the retry chain below that
is `RETRIES + 1` attempts of a 60 s per-attempt timeout plus backoff:
184 s of wall time against an endpoint that accepts the connection and
never answers. Fleet's own MCP client bounds one `tools/call` at 60 s.
So a black-holed embedder made the doctor answer nothing, the call
under test was abandoned, and the assertion that reads the report
failed — not a flake, and not load-sensitive.

`lane_report()` now passes `deadline=DOCTOR_PROBE_DEADLINE`. These pins
hold that in place three ways: the value reaches the seam, the value is
below the client's own call budget, and the real doctor call against a
black hole actually comes back.

Run: python3 services/blend1/tests/test_doctor_budget.py
"""

import re
import socket
import sys
import threading
import time
from pathlib import Path

SERVICE_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(SERVICE_ROOT / "src"))
sys.path.insert(0, str(SERVICE_ROOT.parent / "sem1" / "src"))

import sem1  # noqa: E402
from blend1 import lanes  # noqa: E402
from sem1.providers import http_llama  # noqa: E402

REPO_ROOT = SERVICE_ROOT.parent.parent


def _the_clients_call_budget() -> int:
    """Fleet's own `CALL_TIMEOUT`, read out of the Rust that spends it.

    The doctor runs under this budget, so a probe deadline that ignored
    it would be a number chosen by feel. Reading the constant from its
    source is what makes the comparison a measurement instead of two
    fixtures agreeing with themselves."""
    source = (REPO_ROOT / "src" / "mcp.rs").read_text()
    match = re.search(
        r"pub const CALL_TIMEOUT: Duration = Duration::from_secs\((\d+)\)", source
    )
    assert match, "CALL_TIMEOUT is still declared in src/mcp.rs"
    return int(match.group(1))


def test_the_doctors_probe_deadline_is_under_the_call_budget():
    """The deadline exists to keep the doctor inside the `tools/call`
    that asked it. A deadline at or above that budget would bound
    nothing: the call is abandoned first and the report is still
    empty."""
    budget = _the_clients_call_budget()
    assert lanes.DOCTOR_PROBE_DEADLINE < budget, (
        f"the doctor's probe deadline ({lanes.DOCTOR_PROBE_DEADLINE}s) must "
        f"sit under the client's call budget ({budget}s) — otherwise the "
        f"probe outlives the call that asked and the report is empty anyway"
    )
    # And it is still a real budget: a probe that cannot answer in a
    # handful of seconds is not answering.
    assert lanes.DOCTOR_PROBE_DEADLINE >= 5, lanes.DOCTOR_PROBE_DEADLINE


def test_the_doctor_passes_its_deadline_to_the_probe():
    """The mechanism, captured at the seam. A constant nothing reads is
    a comment; this names the kwarg the doctor actually sends."""
    seen = {}
    real_probe = sem1.probe

    def spy(provider, endpoint=None, battery=False, **kw):
        seen["provider"] = provider
        seen.update(kw)
        return {"provider": provider, "reachable": False, "error": "spy"}

    sem1.probe = spy
    try:
        lanes.lane_report()
    finally:
        sem1.probe = real_probe

    assert seen.get("provider") == "llama-server", seen
    assert seen.get("deadline") == lanes.DOCTOR_PROBE_DEADLINE, seen


def test_the_doctor_answers_instead_of_hanging_on_a_black_holed_embedder():
    """The measurement the row needed: an endpoint that accepts the
    connection and never answers.

    Pinned on the real `lane_report()`, with only the deadline shortened
    to keep the test quick and the default endpoint pointed at the hole.
    Before the fix this call spent the chain's whole 184 s (at the 60 s
    default) and never answered inside a 60 s call budget; here it must
    come back inside its own deadline, with the semantic lane reported
    unavailable-and-named rather than missing."""
    hole = socket.socket()
    hole.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    hole.bind(("127.0.0.1", 0))
    hole.listen(8)
    port = hole.getsockname()[1]
    accepted = []
    stop = threading.Event()

    def accept_forever():
        hole.settimeout(0.1)
        while not stop.is_set():
            try:
                conn, _ = hole.accept()
            except (socket.timeout, OSError):
                continue
            accepted.append(conn)

    threading.Thread(target=accept_forever, daemon=True).start()

    deadline = 2.0
    previous_deadline = lanes.DOCTOR_PROBE_DEADLINE
    previous_endpoint = http_llama.DEFAULT_ENDPOINT
    lanes.DOCTOR_PROBE_DEADLINE = deadline
    http_llama.DEFAULT_ENDPOINT = f"http://127.0.0.1:{port}"
    try:
        started = time.monotonic()
        report = lanes.lane_report()
        spent = time.monotonic() - started
    finally:
        lanes.DOCTOR_PROBE_DEADLINE = previous_deadline
        http_llama.DEFAULT_ENDPOINT = previous_endpoint
        stop.set()
        hole.close()
        for conn in accepted:
            conn.close()

    # The call that asked finishes: bounded by the deadline it passed,
    # with the rest of the report's own time on top — never the chain's
    # 184 s.
    assert spent < deadline + 15, (
        f"the doctor spent {spent:.1f}s against a {deadline}s probe "
        f"deadline (the chain's own worst case is "
        f"{http_llama.worst_case_seconds(60.0):.0f}s)"
    )
    # The lane is REPORTED, not missing: an unavailable lane names why.
    semantic = report["semantic"]
    assert semantic["available"] is False, semantic
    reason = semantic["reason"]
    assert reason.strip(), semantic
    assert "unreachable" in reason or "deadline" in reason or "failed" in reason, reason


if __name__ == "__main__":
    test_the_doctors_probe_deadline_is_under_the_call_budget()
    print("ok: the doctor's probe deadline sits under the client's call budget")
    test_the_doctor_passes_its_deadline_to_the_probe()
    print("ok: the doctor passes its deadline to the probe")
    test_the_doctor_answers_instead_of_hanging_on_a_black_holed_embedder()
    print("ok: the doctor answers instead of hanging on a black-holed embedder")
    print("all passed")
