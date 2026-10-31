---
name: second-brain-corpus-179-unchanged-20261031
description: "second-brain's blend1 corpus (179 items / 24 sources) was unchanged by the 2026-10-31T00:45Z scheduled refresh, store byte-identical"
type: project
date: 2026-10-31
basis: inferred
tier: durable
shape: [state-sync]
phrases: [blend1 refresh, second-brain corpus, tools/call refresh, stdio spawn route, BLEND1_SEM1_PYTHONPATH]
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The second-brain project's blend1 corpus is 179 items over 24 source files (18 under ~/Projects/second-brain/docs/**/*.md plus 6 root .md: AGENTS.md, FUNCTIONAL-SPECIFICATIONS.md, README.md, TECHNICAL-DOCUMENTATION.md, knowledge.md, second-brain.md). The 2026-10-31T00:45Z scheduled refresh (job corpus-refresh-second-brain) ran the lane child via the stdio spawn route — python3 -u services/blend1/service/mcp_server.py with cwd services/blend1 and the five *_PYTHONPATH vars from src/lanes.rs's launch spec (BLEND1 plus BLEND1_SYS1/SEM1/SDM1/GEN1) — and reported: changed=false, action=unchanged, items=179, dropped_duplicates=0, missing_sources=[]. Verified byte-level: manifest.json sha256 4ea98c04727f40a9 and vectors.bin sha256 a4255ff85ba26437 identical before and after, mtime still 2026-10-28 12:38 — the third consecutive no-change refresh since the Oct 28 build. Embedder was live at 127.0.0.1:8901 but unused (hashes matched, no embedding pass).
