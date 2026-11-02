---
name: blend1-stdio-driver-not-stdin
description: "A driver script piped AS the blend1 stdio server's stdin looks like a silent success (exit 0, no output): the server skips unparseable input lines and exits when the file ends — the driver must SUBPROCESS the server with pipes instead."
type: project
date: 2026-11-01
basis: inferred
tier: durable
shape: [fail-open]
phrases: [mcp_server.py < driver.py, JSONDecodeError continue, silent exit 0, Popen spawn pipes, result.content[0].text]
missing: []
recall-count: 1
superseded-by:
valid-until:
---

When driving the vendored blend1 lane (python3 -u services/blend1/service/mcp_server.py) by hand for a scheduled corpus refresh: the server's main loop does json.loads per line and `continue`s on JSONDecodeError, then falls off EOF — so feeding the driver script itself as the server's stdin (`mcp_server.py < driver.py`) consumes the Python source as garbage JSON-RPC, prints nothing, and exits 0. That silent success nearly passed as a completed refresh on 2026-11-01T00:40Z until the missing output was questioned. The correct route: a driver process that Popen-spawns the server with pipes, sends initialize (protocolVersion 2024-11-05) → notifications/initialized → tools/call refresh on the child's stdin, and reads replies from the child's stdout; the refresh result rides as JSON inside result.content[0].text. Env: BLEND1_PYTHONPATH plus BLEND1_SYS1/SEM1/SDM1/GEN1_PYTHONPATH pointing at the sibling services' src, cwd services/blend1.
