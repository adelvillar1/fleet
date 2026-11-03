---
name: dnd-vtt-corpus-2026-11-01-catchup
description: "dnd-vtt's blend1 corpus is 2219 items over 244 source files, confirmed unchanged by the 2026-11-01 catch-up for the missed 2026-11-01T01:00Z window, executed via a stdio JSON-RPC driver of the vendored lane child."
type: project
date: 2026-11-01
basis: inferred
tier: working
shape: []
phrases: [corpus-refresh-dnd-vtt, dnd-vtt blend1 refresh, 2026-11-01T01:00Z catch-up, stdio JSON-RPC driver services/blend1/service/mcp_server.py, ~/.local/share/blend1/vectors/dnd-vtt]
missing: []
recall-count: 0
superseded-by: consolidation-2026-11-03
valid-until: 2026-11-02
---

Job corpus-refresh-dnd-vtt, 2026-11-01T01:00Z window missed while fleet was down, caught up by driving the vendored lane child directly: python3 -u services/blend1/service/mcp_server.py with cwd services/blend1/ and BLEND1_PYTHONPATH plus BLEND1_SYS1/SEM1/SDM1/GEN1_PYTHONPATH -> <fleet>/services/<svc>/src (the src/lanes.rs launch spec), stdio JSON-RPC initialize (protocolVersion 2024-11-05) -> notifications/initialized -> tools/call refresh {"project":"dnd-vtt","sources":["~/Projects/dnd-vtt/docs/**/*.md","~/Projects/dnd-vtt/*.md"]}. Report: action unchanged, source_files 244 (234 docs-tree .md + 11 root .md; CLAUDE.local.md excluded by the corpus guard), items 2219, dropped_duplicates 32, missing_sources []. Store ~/.local/share/blend1/vectors/dnd-vtt/embeddinggemma-2 untouched (manifest.json and vectors.bin mtime Oct 28 15:16 = the prior build), manifest count == entries == 2219 corroborating the report. One-off driver at /tmp/fleet-corpus-refresh-dndvtt-catchup-20261101-driver.py (unique filename; shared-name /tmp collisions have bitten before); the "wait" tool path needs a timeout below the 600s bash ceiling, which the 580s wait honored.
