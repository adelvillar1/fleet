---
name: sem1-corpus-31-items-rebuilt-2026-10-26
description: "sem1's blend1 corpus is 31 items over 4 source files after the 2026-10-25T00:25Z catch-up rebuilt it (sem1's new root CLAUDE.md landed 7 items on top of the 24 already held), the immediate re-call confirmed unchanged, and FUNCTIONAL-SPECIFICATIONS.md remains the one declared source absent from the repo."
type: project
date: 2026-10-26
basis: inferred
tier: durable
shape: [state-sync, docs]
phrases: [corpus-refresh-sem1, tools/call refresh, missing_sources, vectors/sem1, action: unchanged, blend1-lane-spawn-route-refresh]
missing: []
recall-count: 1
superseded-by:
valid-until: 2026-10-28
---

The 2026-10-25T00:25Z corpus-refresh-sem1 window was missed while fleet was down and was run as a catch-up on 2026-10-26T00:05Z through the spawn route. The refresh REBUILT the corpus: report {"project": "sem1", "source_files": 4, "items": 31, "dropped_duplicates": 0, "changed": true, "action": "rebuilt", "store": "~/.local/share/blend1/vectors/sem1", "missing_sources": ["/Users/alejandrodelvillar/Projects/sem1/FUNCTIONAL-SPECIFICATIONS.md"]}. The delta is sem1's own root CLAUDE.md — it appeared at Oct 24 10:03 in commit 8bdc26f ("docs: the slim methodology layer"), i.e. after the 2026-10-24T00:25Z run, and contributes 7 items (preamble + 6 ## sections) on top of the 24 held since Oct 19.

Per-file item counts on disk: TECHNICAL-DOCUMENTATION.md 8, docs/plans/2026-10-07-semantic-embeddings-lane.md 12, docs/recaps/SESSION-RECAP-2026-10-07-sem1-born.md 4, CLAUDE.md 7. Verified: the store is embedder-keyed at vectors/sem1/embeddinggemma-2/ with manifest.json carrying 31 entries, vectors.bin 95,232 bytes = 768 dims × 31 × 4 (the arithmetic matches), both files mtime Oct 25 19:05 local (Oct 26 00:05Z), shas vectors.bin eb0219c6… / manifest.json 38eca3f7…. An immediate identical refresh call reported changed=false, action=unchanged — the rebuild is idempotent at the absolute-path key (KI-223's fix). A live blend1 search against the new store returned real cosines and surfaced the new CLAUDE.md entries at 0.6881 and 0.6867, so the vectors are usable, not just written.

FUNCTIONAL-SPECIFICATIONS.md does not exist in ~/Projects/sem1 (only CLAUDE.md, README.md, TECHNICAL-DOCUMENTATION.md at the root) and is named by missing_sources (KI-225's named-not-skipped report). The earlier "sem1's CLAUDE.md is missing (KI-005)" half of this record is now stale: the root CLAUDE.md exists and is in the corpus.

Route notes: python3 -u services/blend1/service/mcp_server.py with cwd = services/blend1, env = mcp::BASE_ENV (PATH, HOME, LANG, TMPDIR) + BLEND1_PYTHONPATH=<home>/src + BLEND1_{SYS1,SEM1,SDM1,GEN1}_PYTHONPATH=<services>/<svc>/src, then initialize (2024-11-05) → notifications/initialized → tools/call refresh with {project, sources}. Over MCP the child's cwd is the lane home, so sources MUST be passed explicitly or the report is no-source; the driver at tmp/blend1_refresh_sem1.py carries the exact four source entries and survives in the repo (run it with python3 -u tmp/blend1_refresh_sem1.py from the fleet root). A plain `python3 -m blend1.cli` search needs PYTHONPATH, not BLEND1_PYTHONPATH — the latter is read by the MCP server, not the interpreter.
Retired 2026-10-29: Superseded per-run corpus state: the corpus moved to 38 items / 5 sources at the 2026-10-28 refresh (README.md joined via the job's root .md glob) and stayed there through the 2026-10-29T00:25Z refresh; this 31/4 state is three refreshes stale, and its missing-source half is obsolete — the job's declared sources no longer name FUNCTIONAL-SPECIFICATIONS.md.
