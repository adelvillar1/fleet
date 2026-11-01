"""W3's pins: the spawn contract the driver used to re-derive (KI-387)
and the structural interpreter pin (KI-324).

- A client that hangs up mid-handshake gets a named stderr line and a
  clean exit — never a traceback written into the closed pipe.
- An interpreter under 3.10 refuses at startup, naming the version and
  the zip(strict=) sites, so a house PATH that resolves an old
  python3 says WHY the lane will not serve.

Run: python3 services/blend1/tests/test_mcp_server_contract.py
"""

import json
import subprocess
import sys
from pathlib import Path

SERVER = Path(__file__).resolve().parent.parent / "service" / "mcp_server.py"


def test_a_hung_up_client_gets_a_named_exit_not_a_traceback():
    child = subprocess.Popen(
        [sys.executable, "-u", str(SERVER)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    # The initialize line, then the read end closes: the child's
    # answer lands in a closed pipe — KI-387's BrokenPipe class.
    child.stdin.write(
        json.dumps(
            {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {"protocolVersion": "2024-11-05"},
            }
        ).encode()
        + b"\n"
    )
    child.stdin.flush()
    child.stdin.close()
    child.stdout.close()
    code = child.wait(timeout=30)
    stderr = child.stderr.read().decode()
    assert code == 0, f"exit {code}: {stderr}"
    assert "client closed the pipe" in stderr, stderr
    assert "Traceback" not in stderr, stderr


def test_an_old_interpreter_is_refused_by_name():
    # sys.version_info is a plain attribute at runtime: faking the old
    # tuple drives the real guard in the real file.
    probe = (
        "import sys; sys.version_info = (3, 9, 18); "
        f"exec(compile(open({str(SERVER)!r}).read(), 'mcp_server.py', 'exec'))"
    )
    run = subprocess.run(
        [sys.executable, "-c", probe], capture_output=True, text=True, timeout=30
    )
    assert run.returncode == 1, run.stderr
    assert "python 3.10+ is required" in run.stderr, run.stderr
    assert "corpus.py:133" in run.stderr, run.stderr
    assert "3.9.18" in run.stderr, run.stderr


if __name__ == "__main__":
    test_a_hung_up_client_gets_a_named_exit_not_a_traceback()
    print("ok: a hung-up client gets a named exit, not a traceback")
    test_an_old_interpreter_is_refused_by_name()
    print("ok: an old interpreter is refused by name")
    print("all passed")
