---
name: sdm1-blend1-corpus-2026-10-27-catchup
description: "sdm1's blend1 corpus is 80 items over 12 source files, unchanged by the 2026-10-27 catch-up refresh for the missed 2026-10-27T00:20Z window, with all declared sources present and the store untouched since Oct 20."
type: project
date: 2026-10-27
basis: inferred
tier: durable
shape: [state-sync]
phrases: [corpus-refresh-sdm1, sdm1-refresh-2026-10-27.py, action unchanged, 80 items 12 sources, BLEND1_SDM1_PYTHONPATH, KI-333 heredoc]
missing: []
recall-count: 0
superseded-by:
valid-until: 2026-10-31
---

The 2026-10-27 catch-up for job corpus-refresh-sdm1 (window 2026-10-27T00:20Z missed while fleet was down) ran the vendored blend1 lane child on fleet's launch spec and reported: project sdm1, source_files 12, items 80, dropped_duplicates 0, changed false, action "unchanged", missing_sources [] — the derived (key, sha) set equals the store manifest exactly, so the store (manifest.json + vectors.bin, 80 entries / 12 files, embeddinggemma-2, mtime Oct 20 19:22) was not rewritten: sdm1's docs have not changed since the last rebuild. The 12 sources are the 9 docs/**/*.md plus CLAUDE.md, FUNCTIONAL-SPECIFICATIONS.md, TECHNICAL-DOCUMENTATION.md. The literal task wording ({"project": "sdm1"} over MCP from the lane-home cwd) reported source_files 0 / action "no-source" — the KI-316 shape, recorded not trusted; the real call passed the store's own absolute source set derived from the manifest keys plus the docs glob. Driver: generated/sdm1-refresh-2026-10-27.py (KI-333 heredoc write, ast.parse-checked; env carries only the five BLEND1_*_PYTHONPATH locators and a PATH front-loading /usr/local/bin for KI-324's 3.13.6 python3).
Retired 2026-11-01: Retired without replacement: its 80-item/12-file read is stale against the byte-verified 2026-11-01 state (82 items / 13 files since Oct 28); the replacement fact's name already exists and the supersession graph (this fact → 10-26 → new fact) resolves the chain.
