---
name: sdm1-blend1-corpus-state-2026-10-24
description: "sdm1's blend1 corpus is 80 items over 12 source files and was unchanged by the 2026-10-24T00:20Z scheduled refresh, with all four declared sources present."
type: project
date: 2026-10-24
basis: inferred
tier: durable
shape: [state-sync]
phrases: [BLEND1_SDM1_PYTHONPATH, services/blend1/service/mcp_server.py, tools/call refresh, action: unchanged, missing_sources: [], ~/.local/share/blend1/vectors/sdm1]
missing: []
recall-count: 1
superseded-by:
valid-until: 2026-10-25
---

sdm1's blend1 corpus is 80 items over 12 source files and was unchanged by the 2026-10-24T00:20Z scheduled refresh, with all four declared sources present. Verified via the spawn route (memories/blend1-lane-spawn-route-refresh): the vendored blend1 MCP server returned {project: sdm1, source_files: 12, items: 80, changed: false, action: "unchanged", missing_sources: []} for the job's exact source list (docs/**/*.md + CLAUDE.md + TECHNICAL-DOCUMENTATION.md + FUNCTIONAL-SPECIFICATIONS.md), and the store at ~/.local/share/blend1/vectors/sdm1 hashed identically before and after the call (aggregate shasum 018b12b046a51d7043b15ed999848c0be099187f both sides), so the unchanged verdict is byte-verified rather than taken from the report alone. Same numbers as the 2026-10-23T00:20Z run — the corpus has now been stable across two consecutive scheduled refreshes.
Retired 2026-10-26: Retired without a replacement body: the newer state fact `sdm1-blend1-corpus-state-2026-10-26` (written this session) already carries the same corpus state against the 2026-10-25T00:20Z window, plus the no-source trap the literal task string walks into — keeping this one's 2026-10-24 window would only be a stale surface.
