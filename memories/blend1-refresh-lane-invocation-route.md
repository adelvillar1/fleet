---
name: blend1-refresh-lane-invocation-route
description: "The lane tool mcp__blend1__refresh is not natively available in this harness; the working route is spawning the vendored blend1 MCP server directly with fleet's own launch spec."
type: project
date: 2026-10-17
basis: inferred
tier: durable
shape: [change, deps]
phrases: [mcp__blend1__refresh, python3 -u service/mcp_server.py, BLEND1_PYTHONPATH, tools/call refresh, services/blend1, launch_spec]
missing: []
recall-count: 50
superseded-by:
valid-until:
---

The lane tool `mcp__blend1__refresh` is not one of Fleet's native tools in this harness — there is no MCP lane bridge exposed to the agent here, so a scheduled `corpus-refresh-*` job cannot call it by name. The working route is to spawn the vendored blend1 lane directly with fleet's own launch spec (src/lanes.rs `launch_spec`): `python3 -u <home>/service/mcp_server.py` with cwd = services/blend1, env = mcp::BASE_ENV (PATH, HOME, LANG, TMPDIR) plus `BLEND1_PYTHONPATH=<home>/src` and the four sibling path vars `BLEND1_SYS1_PYTHONPATH` / `BLEND1_SEM1_PYTHONPATH` / `BLEND1_SDM1_PYTHONPATH` / `BLEND1_GEN1_PYTHONPATH` pointing at `<services root>/<svc>/src`. Then the stdio JSON-RPC handshake: initialize (protocolVersion 2024-11-05), notifications/initialized, tools/list (returns ask, web, doctor, search, refresh, seed), tools/call with name "refresh" and arguments {project, sources}. The lane's own `refresh` resolves project names to `<root>/<project>` under ~/.local/share/blend1/vectors and returns {project, source_files, items, changed, action, store} with action ∈ rebuilt | unchanged | no-source. Note the relpath quirk: `_chunks` builds relpath relative to the child's cwd, so the lane's home cwd yields keys like `../../../sdm1/CLAUDE.md#(preamble)` — the corpus's keys are cwd-relative, which is why the staleness hash comparison only holds for the same cwd. The prior corpus-refresh-sys1 run in session 20261017T001501Z-bce8a3a6 used this same route, so it is the established fallback rather than an improvisation.
