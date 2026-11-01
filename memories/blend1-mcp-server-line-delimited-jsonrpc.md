---
name: blend1-mcp-server-line-delimited-jsonrpc
description: "The vendored blend1 MCP server speaks newline-delimited JSON-RPC (not LSP framing), answers initialize only after a minutes-long cold import pass, and its refresh tool needs the four BLEND1_<SVC>_PYTHONPATH sibling vars."
type: project
date: 2026-11-01
basis: inferred
tier: durable
shape: [state-sync]
phrases: [newline-delimited JSON-RPC, Content-Length, _import_blend1, tools/call refresh, BLEND1_SEM1_PYTHONPATH, refresh_deepseek_harness.py]
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The vendored blend1 lane server (services/blend1/service/mcp_server.py, the fork's bundle) speaks newline-delimited JSON-RPC on stdio — one JSON object per stdin line, replies one JSON object + "\n" on stdout (its respond()/error() are sys.stdout.write) — NOT LSP Content-Length framing, despite the "Transport: stdio JSON-RPC (MCP 2024-11-05)" docstring. Its notifications (id=None) are silently skipped, `ping` must be answered with an empty result, and `initialize` is emitted by the main loop only AFTER _import_blend1() completes, which on a first-of-the-evening child measures minutes (00:56 launch had not answered by +2m15s), while a warm child answers in 0.1s. A Content-Length-framing driver therefore dead-reads until timeout while holding requests in a partially-consumed stream, then breaks the child with BrokenPipeError on exit. Hand-driving refresh calls must: write bytes (stdin PIPE is binary), read line-delimited, send initialize before the reply is possible and allow a 300s window, and expect the refresh report inside result.content[].text (JSON) with keys project/source_files/items/dropped_duplicates/changed/action/store/missing_sources. Worked driver kept at .corpus-refresh/2026-11-01-deepseek-harness/refresh_deepseek_harness.py in the fleet repo; its launch env is BASE_ENV (PATH/HOME/LANG/TMPDIR) + BLEND1_PYTHONPATH + BLEND1_{SYS1,SEM1,SDM1,GEN1}_PYTHONPATH, cwd services/blend1, exactly as src/lanes.rs::launch_spec + child_env build it (KI-310: the refresh leg needs the sem1 sibling var named).
