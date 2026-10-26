---
name: blend1-refresh-spawn-driver-recipe
description: "Spawning the vendored blend1 MCP server to refresh a corpus: cwd must be the TARGET repo (KI-316), siblings wire as BLEND1_<SVC>_PYTHONPATH, and fleet's generated/blend1-refresh.py is the working driver."
type: project
date: 2026-10-26
basis: inferred
tier: durable
shape: [change, deps]
phrases: [mcp__blend1__refresh, python3 -u service/mcp_server.py, BLEND1_PYTHONPATH, tools/call refresh, services/blend1, launch_spec]
missing: []
recall-count: 2
superseded-by:
valid-until:
---

The lane tool `mcp__blend1__refresh` is not natively available in this harness (no invocation bridge at the agent seat; tool_search surfaces the name but only mcp_resource/mcp_prompt exist), so corpus-refresh jobs spawn the vendored blend1 MCP server directly: `python3 -u <services root>/blend1/service/mcp_server.py`, newline-delimited JSON-RPC (KI-318 — Content-Length framing is silently discarded and the driver hangs). The launch spec that works (KI-316, correcting this fact's earlier registry-shaped wording "cwd = services/blend1"): (1) the child's cwd is the TARGET PROJECT's repo — refresh's default source set (docs/**/*.md + the root contract docs) resolves cwd-relative, while corpus keys are absolute paths since KI-223, so cwd picks the source set and never identities; a child launched from the lane's home resolves another project's docs to nothing and reports action "no-source" with 0 files; (2) env is BLEND1_PYTHONPATH=<services root>/blend1/src plus the four sibling vars under the adapter's own names BLEND1_SYS1_PYTHONPATH / BLEND1_SEM1_PYTHONPATH / BLEND1_SDM1_PYTHONPATH / BLEND1_GEN1_PYTHONPATH = <services root>/<svc>/src (refresh refuses without BLEND1_SEM1_PYTHONPATH; bare SYS1_PYTHONPATH-style names are not read). Handshake: initialize (protocolVersion 2024-11-05), notifications/initialized, tools/call name "refresh" arguments {project} (sources optional when cwd is the target repo; explicit absolute globs also work — see blend1-refresh-needs-explicit-absolute-sources). Report: {project, source_files, items, dropped_duplicates, changed, action ∈ rebuilt|unchanged|no-source, store, missing_sources}. A checked-in working driver lives at fleet generated/blend1-refresh.py (parameterize target + project; sed-retarget in one pass as prior runs did with /tmp/blend1_refresh.py). Re-verified 2026-10-26T00:55Z on deepseek-harness: 456 files / 3279 items / changed false / action "unchanged" / missing_sources [], store mtimes 2026-10-20T20:03:32Z. Prior re-verifications of this route: sys1 2026-10-24 (38/257 unchanged), pmocockpit 2026-10-24 (5/72 unchanged), dev-decisions 2026-10-22 and 2026-10-24 (10/79 unchanged). This fact supersedes blend1-refresh-lane-invocation-route (retired 2026-10-22); its own earlier body carried the stale cwd=lane-home spec that produced the 2026-10-26 no-source first attempt — the KI-316 correction lived only in the per-project corpus-state facts until now.
