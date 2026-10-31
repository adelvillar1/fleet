---
name: corpus-dev-decisions-rebuilt-2026-10-28
description: "dev-decisions' blend1 corpus is 142 items over 12 sources after the 2026-10-28T00:50Z catch-up rebuild — the two new root docs (README.md, SKILL.md) added 63 items to the prior 79/10 state"
type: project
date: 2026-10-28
basis: inferred
tier: durable
shape: [state-sync]
phrases: [corpus-refresh-dev-decisions, tools/call refresh, BLEND1_PYTHONPATH, action unchanged, dev-decisions docs/**/*.md]
missing: [the declared source set in the job's task string (seeded wording drift KI-343/KI-316 tracks), whether the next scheduled window's explicit sources match the store's actual 12-source set]
recall-count: 0
superseded-by: corpus-dev-decisions-unchanged-2026-10-31
valid-until:
---

dev-decisions' blend1 corpus was rebuilt by the 2026-10-28T00:50Z catch-up (executed 2026-10-28 12:35-12:40CDT, the missed window while fleet was down) to 142 items over 12 source files — the declared source set passed as explicit absolute globs (`~/Projects/dev-decisions/docs/**/*.md` plus the two root docs `README.md` and `SKILL.md`; the repo carries no CLAUDE.md, TECHNICAL-DOCUMENTATION.md, or FUNCTIONAL-SPECIFICATIONS.md, so the job's "every project's corpus is docs tree plus root .md files" wording is what ran). The pre-refresh store held 79 entries / 10 sources (Oct 20 19:55 build; the delta is the two root docs' 63 items). Report: {project dev-decisions, source_files 12, items 142, dropped_duplicates 0, changed true, action rebuilt, missing_sources []}; store manifest.json + vectors.bin rewritten 12:38CDT, 142 entries / 768 dims, sha256 490ed5d7… / f09250f7…, by-file counts verified (README.md 31, SKILL.md 32, the ten prior sources unchanged). The launch spec was assembled from src/lanes.rs (BLEND1_PYTHONPATH plus BLEND1_{SYS1,SEM1,SDM1,GEN1}_PYTHONPATH → services/<svc>/src, cwd services/blend1, child python3 3.13.6 via the inherited PATH — KI-324's zip(strict=) requirement), driven line-delimited per KI-318; the ~120 s queue wait before the reply was KI-325/KI-349 embedder contention from the ~12 concurrent catch-up children, not a wedge.
