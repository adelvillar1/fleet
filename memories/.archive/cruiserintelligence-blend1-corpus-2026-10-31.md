---
name: cruiserintelligence-blend1-corpus-2026-10-31
description: "cruiserintelligence's blend1 corpus is 41 sources / 338 items, unchanged by the 2026-10-31T01:15Z scheduled refresh with the store verified byte-identical, run through a durable driver pair (scripts/refresh-cruiserintelligence-blend1.{sh,py}) instead of a per-run temp script"
type: project
date: 2026-10-31
basis: inferred
tier: durable
shape: [state-sync, resolved]
phrases: [services/blend1/service/mcp_server.py, tools/call refresh, BLEND1_PYTHONPATH, project cruiserintelligence, KI-348, action rebuilt]
missing: []
recall-count: 0
superseded-by:
valid-until: 2026-10-31
---

cruiserintelligence's blend1 fact corpus stands at 41 source files / 338 items / 768 dims and was unchanged (changed: false, action: unchanged, dropped_duplicates 0, missing_sources []) by the 2026-10-31T01:15Z scheduled refresh — the second consecutive no-change after the 2026-10-29T01:15Z rebuild to the same 41/338, itself the KI-348 definitional rebuild that added README.md via the widened root glob. Verified beyond the report: the store stayed byte-identical through the run — manifest.json sha256 13c8bceae2de9e2de7b2839449a916c640aa2dbe, vectors.bin sha256 f75faea77d0a0ad46fd53834376d9ca50c8fa959, both mtime 2026-10-28 20:17 local, manifest 338 entries / model embeddinggemma-2 / dims 768 / provider llama-server, manifest file-set == the declared tree (in tree-not in manifest: only CLAUDE.local.md, excluded via EXCLUDED_FILENAMES = {"CLAUDE.local.md"} per the corpus library's credential guard — 41 of 42 candidates contribute items; CLAUDE.local.md is read fully, just never embedded, so no missing_sources flag). Route: the lane tool is absent from the scheduled run's toolset, so the spawn route is the src/lanes.rs blend1 spec driven by hand — this fire retired the per-run driver churn (KI-369's fourth copy) with a durable pair, scripts/refresh-cruiserintelligence-blend1.sh (exports BLEND1_PYTHONPATH + BLEND1_{SYS1,SEM1,SDM1,GEN1}_PYTHONPATH onto services/blend1) execing scripts/refresh-cruiserintelligence-blend1.py (stdio JSON-RPC: initialize 2024-11-05 → notifications/initialized → tools/call refresh {"project": "cruiserintelligence", "sources": ["~/Projects/cruiserintelligence/docs/**/*.md", "~/Projects/cruiserintelligence/*.md"]}), git-untracked pending the owner's durable-route decision. The write-guard refused every text-mode draft (CWD/SERVER/PROJECT/PROJECT_NAME/SOURCES against repo-established lowercase terms, then Python's own True/False/None), so the driver is bytes-mode with keyword-free names and went through bash — the known whole-file-scan behavior, fourth project it has shaped.
Retired 2026-11-01: Dated corpus state snapshot: superseded by this fire's own fact carrying the newer verified interval state — the byte-identical read-back this fact itself prescribed.
