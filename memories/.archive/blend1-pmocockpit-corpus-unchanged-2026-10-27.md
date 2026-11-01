---
name: blend1-pmocockpit-corpus-unchanged-2026-10-27
description: "pmocockpit's blend1 corpus is 72 items over 5 docs sources, unchanged by the 2026-10-27 catch-up refresh for the missed 2026-10-27T01:10Z window, store untouched (mtime Oct 19 09:33, 72 entries / 768 dims)."
type: state-sync
date: 2026-10-27
basis: inferred
tier: working
shape: [state-sync]
phrases: [corpus-refresh-pmocockpit, no-source KI-316, pmocockpit/docs/**/*.md, action: unchanged, missing_sources, line-delimited JSON-RPC]
missing: []
recall-count: 0
superseded-by: consolidation-2026-10-31
valid-until: 2026-10-30
---

The 2026-10-27 catch-up for cron job corpus-refresh-pmocockpit (window 2026-10-27T01:10Z, missed while fleet was down) drove the vendored blend1 lane child by hand and reported {"project": "pmocockpit", "source_files": 5, "items": 72, "dropped_duplicates": 0, "changed": false, "action": "unchanged", "missing_sources": [pmocockpit's absent CLAUDE.md, TECHNICAL-DOCUMENTATION.md, FUNCTIONAL-SPECIFICATIONS.md]} in 0.14s, child exit 0. The store at ~/.local/share/blend1/vectors/pmocockpit/embeddinggemma-2/ was left untouched: manifest.json (72890 B) and vectors.bin (221184 B) both still mtime 2026-10-19 09:33, 72 entries / 768 dims. The literal {"project": "pmocockpit"} call was NOT sent — KI-316 already proved it answers no-source over an empty corpus — so the run went straight to the store-recovered explicit absolute source set (pmocockpit/docs/**/*.md + the three root contract docs). All five docs sources (Aug 26–30 mtimes) predate the Oct 19 store, so unchanged is the honest result, not a silent skip. Run hygiene: KI-317 driver /tmp/fleet-blend1-pmocockpit-refresh-<stamp>-<pid>.py written+executed+removed in one shell invocation; KI-318 line-delimited JSON-RPC; KI-324 child PATH front-loaded /usr/local/bin (child python3 = 3.13.6, though the unchanged path never reaches the zip(strict=) embed leg); KI-325 embedder pre-probe HTTP 200 in 0.16s. Nothing new for docs/known-issues.md — the run followed the documented workaround; KI-316's class stays open (twelve jobs still seed project-only and seed_missing reconciles task strings back to project-only on fleet install).
Retired 2026-10-31: Stale chain tail (KI-355's shape): a working-tier 2026-10-27 state fact left with an empty `superseded-by` while later pmocockpit corpus facts were written (10-28 → 10-29 → this run's 10-31). Its content is also out of date — it records the missing_sources as the three absent root contract docs, whereas the current seeded source shape names only the root `*.md` glob, whose absence is KI-343's standing entry. The durable successor pmocockpit-corpus-2026-10-31 carries this run's verified state, so this fact r…
