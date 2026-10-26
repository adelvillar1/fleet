#!/usr/bin/env python3
"""Drive the vendored blend1 lane child over stdio JSON-RPC by hand.

The vendored server speaks newline-delimited JSON. The child runs with the
TARGET PROJECT's repo as cwd so refresh's default source set (docs/**/*.md
plus the root contract docs, cwd-relative) resolves there; corpus keys are
absolute paths (KI-223), so cwd only picks the source set, never identities.
"""
import json
import subprocess
import sys

fleet = "/Users/alejandrodelvillar/Projects/fleet"
blend1 = f"{fleet}/services/blend1"
target = "/Users/alejandrodelvillar/Projects/deepseek-harness"

env = {
    "BLEND1_PYTHONPATH": f"{blend1}/src",
    "BLEND1_SYS1_PYTHONPATH": f"{fleet}/services/sys1/src",
    "BLEND1_SEM1_PYTHONPATH": f"{fleet}/services/sem1/src",
    "BLEND1_SDM1_PYTHONPATH": f"{fleet}/services/sdm1/src",
    "BLEND1_GEN1_PYTHONPATH": f"{fleet}/services/gen1/src",
    "PATH": "/usr/bin:/bin:/usr/local/bin:/opt/homebrew/bin",
}


def send(proc, payload):
    proc.stdin.write((json.dumps(payload) + "\n").encode())
    proc.stdin.flush()


def read_reply(proc, want_id):
    while True:
        line = proc.stdout.readline()
        if not line:
            raise RuntimeError("child closed stdout")
        reply = json.loads(line)
        if reply.get("id") == want_id:
            return reply


proc = subprocess.Popen(
    ["python3", "-u", f"{blend1}/service/mcp_server.py"],
    cwd=target,
    env=env,
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
)

send(proc, {
    "jsonrpc": "2.0", "id": 1, "method": "initialize",
    "params": {"protocolVersion": "2024-11-05", "capabilities": {},
               "clientInfo": {"name": "catchup-driver", "version": "0.0"}},
})
init = read_reply(proc, 1)
print("INIT:", json.dumps(init.get("result", init))[:200], flush=True)

send(proc, {"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}})

send(proc, {
    "jsonrpc": "2.0", "id": 2, "method": "tools/call",
    "params": {"name": "refresh", "arguments": {"project": "deepseek-harness"}},
})
result = read_reply(proc, 2)
print("REFRESH:", json.dumps(result, indent=2), flush=True)

proc.stdin.close()
try:
    proc.wait(timeout=10)
except subprocess.TimeoutExpired:
    proc.kill()
err = proc.stderr.read().decode()
if err.strip():
    print("STDERR:", err[-2000:], file=sys.stderr)
