---
name: sem1-blend1-corpus-state-2026-10-27
description: "sem1's blend1 corpus is 31 items over 4 source files, unchanged by the 2026-10-27 catch-up refresh for the missed 2026-10-27T00:25Z window, with FUNCTIONAL-SPECIFICATIONS.md reported as the one missing declared source."
type: project
date: 2026-10-27
basis: inferred
tier: durable
shape: [state-sync, resolved]
phrases: [services/blend1/service/mcp_server.py, BLEND1_SEM1_PYTHONPATH, tools/call refresh project sem1 sources, ~/.local/share/blend1/vectors/sem1/embeddinggemma-2/manifest.json, source_files 4 items 31 unchanged, fleet-blend1-sem1-refresh driver]
missing: []
recall-count: 1
superseded-by:
valid-until: 2026-10-28
---

The 2026-10-27T03:49Z catch-up (for the 2026-10-27T00:25Z `corpus-refresh-sem1` window fleet was down for) refreshed sem1 through the vendored blend1 lane child, spawned to src/lanes.rs's spec (cwd `services/blend1`, BLEND1_PYTHONPATH + BLEND1_{SYS1,SEM1,SDM1,GEN1}_PYTHONPATH, child PATH front-loading /usr/local/bin so python3 is 3.13.6 per KI-324) and driven line-delimited by hand.

- The literal task (project only, as the job's task string still seeds): `{"project": "sem1"}` → source_files 0, items 0, action "no-source", missing_sources [] — KI-316's shape, for the second time on this project.
- The pinned call (sem1's four declared sources as absolute globs: `~/Projects/sem1/docs/**/*.md`, CLAUDE.md, TECHNICAL-DOCUMENTATION.md, FUNCTIONAL-SPECIFICATIONS.md) → `{"source_files": 4, "items": 31, "dropped_duplicates": 0, "changed": false, "action": "unchanged", "missing_sources": ["/Users/alejandrodelvillar/Projects/sem1/FUNCTIONAL-SPECIFICATIONS.md"]}`.
- Store untouched: `~/.local/share/blend1/vectors/sem1/embeddinggemma-2/{manifest.json,vectors.bin}` mtime 2026-10-25T19:05:49-0500 (2026-10-26T00:05:49Z), 31 entries / 768 dims / model embeddinggemma-2. Every source is older than that build — newest CLAUDE.md 2026-10-24T15:03:40Z — so `unchanged` is a real set-equality no-op, not a stale pass. sem1's repo carries no FUNCTIONAL-SPECIFICATIONS.md at all.
- The four sources in the store are CLAUDE.md, TECHNICAL-DOCUMENTATION.md, docs/plans/2026-10-07-semantic-embeddings-lane.md, docs/recaps/SESSION-RECAP-2026-10-07-sem1-born.md; the other four files under sem1/docs are the architecture diagram in .json/.drawio/.html/.png/.svg, which the .md-only filter rightly ignores.
- The job's task string in `~/.fleet/cron/jobs.json` still seeds the project alone (last_run 2026-10-26T00:25Z, runs 2) — KI-316's durable fix has not landed.
- No embed contention this window: the embedder at 127.0.0.1:8901 answered /health 200 in 2.6 ms, and the refresh never reached the embed path (hash-compare only).
Retired 2026-10-29: Superseded per-run corpus state: the 31/4 corpus it describes was superseded by the 2026-10-28 rebuild (38 items / 5 sources) and verified unchanged through the 2026-10-29T00:25Z refresh. Its KI-316 claim ("the job's task string still seeds the project alone") is now false: ~/.fleet/cron/jobs.json carries the full spawn-route task string with explicit source globs, verified 2026-10-29 (runs 4, successes 4).
