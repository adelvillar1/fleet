#!/usr/bin/env python3
"""Drive the vendored blend1 MCP server over stdio JSON-RPC, by hand.

Launch spec from fleet src/lanes.rs launch_spec + sibling_paths:
    cwd   = services/blend1
    cmd   = python3 -u services/blend1/service/mcp_server.py
    env   = BASE_ENV (PATH, HOME, LANG, TMPDIR from this process)
            + BLEND1_PYTHONPATH=<home>/src
            + BLEND1_SYS1/SEM1/SDM1/GEN1_PYTHONPATH=<services root>/<svc>/src
            + declared passthrough copied only when present in the caller
"""
import json
import os
import subprocess

fleet_root = os.path.expanduser("~/Projects/fleet")
lane_home = os.path.join(fleet_root, "services", "blend1")
services_root = os.path.join(fleet_root, "services")

BASE_ENV = ["PATH", "HOME", "LANG", "TMPDIR"]
PASSTHROUGH = [
    "LIQUID_API_KEY", "FASTINO_API_KEY", "TINYFISH_API_KEY",
    "DASHSCOPE_API_KEY", "STEPFUN_API_KEY", "TOKEN_PLAN_API_KEY",
    "OLLAMA_API_KEY", "TABPFN_API_KEY",
    "BLEND1_ESCALATION_MODEL", "BLEND1_ESCALATION_BASE",
    "BLEND1_ESCALATION_KEY_ENV", "BLEND1_ESCALATION_API_KEY",
    "BLEND1_LOG_DIR",
    "BLEND1_CRITIC_1_BASE", "BLEND1_CRITIC_1_MODEL", "BLEND1_CRITIC_1_KEY_ENV",
    "BLEND1_CRITIC_2_BASE", "BLEND1_CRITIC_2_MODEL", "BLEND1_CRITIC_2_KEY_ENV",
    "BLEND1_VECTORS_DIR", "BLEND1_PROJECT",
]

child_env = {name: os.environ[name] for name in BASE_ENV if name in os.environ}
for name in PASSTHROUGH:
    if name in os.environ:
        child_env[name] = os.environ[name]
child_env["BLEND1_PYTHONPATH"] = os.path.join(lane_home, "src")
for sibling in ("sys1", "sem1", "sdm1", "gen1"):
    child_env["BLEND1_%s_PYTHONPATH" % sibling.upper()] = os.path.join(
        services_root, sibling, "src")

proc = subprocess.Popen(
    ["python3", "-u", os.path.join(lane_home, "service", "mcp_server.py")],
    cwd=lane_home, env=child_env,
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
    text=True,
)


def send(payload):
    proc.stdin.write(json.dumps(payload) + "\n")
    proc.stdin.flush()


def recv():
    while True:
        line = proc.stdout.readline()
        if not line:
            raise RuntimeError("child closed stdout")
        line = line.strip()
        if line:
            return json.loads(line)


send({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
    "protocolVersion": "2024-11-05",
    "capabilities": {},
    "clientInfo": {"name": "fleet-catchup-driver", "version": "1.0"},
}})
init = recv()
print("initialize ->", init["result"]["serverInfo"], flush=True)

send({"jsonrpc": "2.0", "method": "notifications/initialized"})

pmo = os.path.expanduser("~/Projects/pmocockpit")
send({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
    "name": "refresh",
    "arguments": {
        "project": "pmocockpit",
        "sources": [
            pmo + "/docs/**/*.md",
            pmo + "/CLAUDE.md",
            pmo + "/TECHNICAL-DOCUMENTATION.md",
            pmo + "/FUNCTIONAL-SPECIFICATIONS.md",
        ],
    },
}})
result = recv()
print(result["result"]["content"][0]["text"], flush=True)

proc.stdin.close()
proc.wait(timeout=15)
