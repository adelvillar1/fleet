---
name: blend1-lane-spawn-route-refresh
description: "The lane tool mcp__blend1__refresh is not natively available in this harness; the working route is spawning the vendored blend1 MCP server directly with fleet's own launch spec, verified again 2026-10-22."
type: project
date: 2026-10-22
basis: inferred
tier: durable
shape: [change, deps]
phrases: [mcp__blend1__refresh, python3 -u service/mcp_server.py, BLEND1_PYTHONPATH, tools/call refresh, services/blend1, launch_spec]
missing: []
recall-count: 6
superseded-by:
valid-until:
---

The lane tool `mcp__blend1__refresh` is not one of Fleet's native tools in this harness — there is no MCP lane bridge exposed to the agent here, so a scheduled `corpus-refresh-*` job cannot call it by name. The working route is to spawn the vendored blend1 lane directly with fleet's own launch spec (src/lanes.rs `launch_spec`): `python3 -u <home>/service/mcp_server.py` with cwd = services/blend1, env = mcp::BASE_ENV (PATH, HOME, LANG, TMPDIR) plus `BLEND1_PYTHONPATH=<home>/src` and the four sibling path vars `BLEND1_SYS1_PYTHONPATH` / `BLEND1_SEM1_PYTHONPATH` / `BLEND1_SDM1_PYTHONPATH` / `BLEND1_GEN1_PYTHONPATH` pointing at `<services root>/<svc>/src`. Then the stdio JSON-RPC handshake: initialize (protocolVersion 2024-11-05), notifications/initialized, tools/list (returns ask, web, doctor, search, refresh, seed), tools/call with name "refresh" and arguments {project, sources}. The lane's own `refresh` resolves project names to `<root>/<project>` under ~/.local/share/blend1/vectors and returns {project, source_files, items, changed, action, store, missing_sources} with action ∈ rebuilt | unchanged | no-source.

2026-10-22 re-verification (the `corpus-refresh-dev-decisions` scheduled run): the spawn route still works unchanged — initialize returned `{"name": "blend1", "version": "0.1.0"}`, tools/list returned the same six tools, and `refresh` for project dev-decisions with sources `docs/**/*.md` + the three root contract docs returned `{source_files: 10, items: 79, changed: false, action: "unchanged"}` with `missing_sources` naming the three root docs that do not exist in that repo. The chunk keys are now ABSOLUTE paths (`/Users/alejandrodelvillar/Projects/dev-decisions/docs/...`), so KI-223's cwd-poisoning fix is live and an `unchanged` verdict is trustworthy rather than a cwd artifact. KI-277 remains open: the live `corpus-refresh-*` job task strings in `~/.fleet/cron/jobs.json` still name `mcp__blend1__refresh` (the manifest's spawn-route wording never reached the existing store because `seed_missing` only writes absent names), so every scheduled run re-derives this route by hand — which this session did.

This fact supersedes `blend1-refresh-lane-invocation-route`, which was retired 2026-10-22 when its relpath/cwd caveat and its `missing_sources`-less report shape both went stale. A working driver implementing the route verbatim lives at the session scratchpad path `blend1_refresh.py`.
