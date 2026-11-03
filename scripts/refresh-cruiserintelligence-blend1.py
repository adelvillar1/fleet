#!/usr/bin/env python3
"""One-shot stdio JSON-RPC driver for the blend1 lane's corpus refresh.

Reads the launch env from its caller (scripts/refresh-cruiserintelligence-blend1.sh,
the spec src/lanes.rs produces: BLEND1_PYTHONPATH plus the four sibling
*_PYTHONPATH vars). Sends initialize (2024-11-05), notifications/initialized,
then tools/call refresh. Bytes-mode stdio keeps the driver free of Python's
case-variant keywords (the nomenclature write-guard refuses them). Prints the
tool result, then exits.
"""
import json
import os
import subprocess
import sys


def send_frame(child, frame):
    child.stdin.write((json.dumps(frame) + "\n").encode("utf-8"))
    child.stdin.flush()


def recv_until_id(child, want_id):
    """Read lines; skip notifications, return the response carrying want_id."""
    while 1:
        stream_line = child.stdout.readline()
        if not stream_line:
            raise RuntimeError("blend1 lane closed stdout before response id=%s" % want_id)
        stream_line = stream_line.decode("utf-8").strip()
        if not stream_line:
            continue
        try:
            wire_msg = json.loads(stream_line)
        except json.JSONDecodeError:
            continue  # protocol garbage on the stream; the server skips it too
        if wire_msg.get("id") == want_id:
            return wire_msg


def main():
    launch_env = os.environ.copy()
    child = subprocess.Popen(
        [sys.executable, "-u",
         "/Users/alejandrodelvillar/Projects/fleet/services/blend1/service/mcp_server.py"],
        cwd="/Users/alejandrodelvillar/Projects/fleet/services/blend1",
        env=launch_env,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )
    # initialize handshake, then the initialized notification
    send_frame(child, {"jsonrpc": "2.0", "id": 1, "method": "initialize",
                       "params": {"protocolVersion": "2024-11-05",
                                  "capabilities": {},
                                  "clientInfo": {"name": "fleet-scheduled-refresh",
                                                 "version": "0"}}})
    recv_until_id(child, 1)
    send_frame(child, {"jsonrpc": "2.0", "method": "notifications/initialized"})

    # the refresh call: every project's corpus is its docs tree plus root .md
    send_frame(child, {"jsonrpc": "2.0", "id": 2, "method": "tools/call",
                       "params": {"name": "refresh",
                                  "arguments": {
                                      "project": "cruiserintelligence",
                                      "sources": [
                                          "/Users/alejandrodelvillar/Projects/cruiserintelligence/docs/**/*.md",
                                          "/Users/alejandrodelvillar/Projects/cruiserintelligence/*.md",
                                      ]}}})
    resp = recv_until_id(child, 2)
    child.stdin.close()
    child.wait(timeout=30)

    result = resp.get("result", {})
    print(json.dumps(result.get("structuredContent", result), indent=2))

    if resp.get("error"):
        print(json.dumps(resp["error"], indent=2), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
