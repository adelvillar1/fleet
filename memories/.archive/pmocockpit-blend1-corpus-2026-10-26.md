---
name: pmocockpit-blend1-corpus-2026-10-26
description: "pmocockpit's blend1 corpus is 72 items over 5 source files, unchanged by the 2026-10-23T01:10Z scheduled refresh and the 2026-10-25T01:10Z catch-up (run 2026-10-26), with the three root contract docs absent from the repo and named in missing_sources."
type: project
date: 2026-10-26
basis: inferred
tier: durable
shape: [state-sync]
phrases: [corpus-refresh-pmocockpit, mcp__blend1__refresh, tools/call refresh, action: unchanged, missing_sources, source_files: 5]
missing: []
recall-count: 0
superseded-by:
valid-until: 2026-10-28
---

The pmocockpit fact corpus remains 72 items over 5 source files, unchanged by the 2026-10-23T01:10Z scheduled refresh and by the 2026-10-25T01:10Z catch-up for the window fleet was down (run 2026-10-26T00:1xZ): the blend1 lane's refresh reported {project: pmocockpit, source_files: 5, items: 72, dropped_duplicates: 0, changed: false, action: unchanged}, the store manifest at ~/.local/share/blend1/vectors/pmocockpit/embeddinggemma-2/manifest.json kept its Oct 19 09:33:23 mtime with count 72, and missing_sources still names the three root docs absent from the repo: CLAUDE.md, TECHNICAL-DOCUMENTATION.md, FUNCTIONAL-SPECIFICATIONS.md (KI-225's named-not-skipped fix reporting as designed). The catch-up ran through the spawn route (KI-221/224's fix): the vendored blend1 MCP server launched with fleet's launch spec (cwd services/blend1, BLEND1_PYTHONPATH + the four BLEND1_<SVC>_PYTHONPATH siblings) and tools/call refresh driven over stdio JSON-RPC by hand. It reproduced KI-316 on this window: the literal project-only call derives nothing (source_files: 0, action: no-source) because the default source set is cwd-relative and the lane child's cwd is services/blend1 — the report above came from passing the sources explicitly (pmocockpit/docs/**/*.md plus the three root contract docs, absolute), the store's own recovered key set.
Retired 2026-10-29: Two durable facts described one pmocockpit store state: this 2026-10-26 fact and the newer 2026-10-28 one were both live (the 10-28 fact was written as a plain write, so this ancestor was never marked superseded). The 2026-10-29 catch-up verified the store again at 72 items / 5 sources and folded the state into pmocockpit-corpus-2026-10-29, so this description is now two generations stale; KI-355's documented workaround is to finish the chain by hand.
