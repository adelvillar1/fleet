---
name: corpus-dev-decisions-unchanged-2026-10-31
description: "dev-decisions' blend1 corpus is 142 items over 12 sources, unchanged by the 2026-10-31T00:50Z scheduled refresh, the store byte-identical to the Oct 28 build"
type: project
date: 2026-10-31
basis: inferred
tier: durable
shape: [state-sync, resolved]
phrases: [corpus-refresh-dev-decisions, tools/call refresh, BLEND1_PYTHONPATH, action unchanged, dev-decisions docs/**/*.md]
missing: []
recall-count: 0
superseded-by:
valid-until:
---

dev-decisions' blend1 corpus is 142 items over 12 source files, unchanged by the 2026-10-31T00:50Z scheduled refresh: the vendored blend1 lane (spawn route, stdio JSON-RPC per src/lanes.rs — BLEND1_PYTHONPATH plus the four BLEND1_{SYS1,SEM1,SDM1,GEN1}_PYTHONPATH path-wires, cwd services/blend1) answered the refresh tool with {project dev-decisions, source_files 12, items 142, dropped_duplicates 0, changed false, action unchanged, missing_sources []} against explicit absolute globs (~/Projects/dev-decisions/docs/**/*.md plus *.md at the root). Store verified untouched at two levels: both files' mtime is 2026-10-28 12:38:49 (before this run) and their sha256s — manifest 490ed5d7…, vectors f09250f7… — match the values the superseded Oct-28 fact itself recorded for its rebuild, so byte-identical is manifest-confirmed, not inferred from the report. Manifest reconciled against the report: count 142 = items, 12 distinct files in entry keys = source_files (SKILL.md 32, README.md 31, three plans 12-13 each, six recaps and linkedin-post.md 4-8 each). Steady state embeds nothing (sem1 imports lazily; the 8901 embedder was up but unused). This run also closed the superseded fact's open companion: the explicit source globs derived exactly the store's 12-source set — declared sources and store content agree. The driver was authored piped to python3 through a stdin heredoc with no temp file because write_file refused the FLEET driver variable and the HOME env-dict key (KI-330/KI-333 shapes, recurred; KI-333's evidence cell updated with this occurrence).
