---
name: blend1-refresh-over-mcp-must-name-sources
description: "A blend1 `refresh` over MCP must carry `sources` as absolute globs of the project root — the project name alone derives nothing and returns a clean-looking `action: "no-source"` (KI-316, unfixed)."
type: project
date: 2026-10-26
basis: inferred
tier: durable
shape: [change, fail-open, resolved]
phrases: [action: no-source, source_files: 0, tools/call refresh, docs/**/*.md, missing_sources, BLEND1_SEM1_PYTHONPATH]
missing: []
recall-count: 0
superseded-by:
valid-until: 2026-10-25
---

A `refresh` call over MCP that carries the project name alone derives NOTHING and reports a clean-looking success: the lane child's cwd is the lane's home (`services/blend1/`), so the default source set (`docs/**/*.md` plus the three root contract docs) is cwd-relative and matches zero files, and the lane answers `{source_files: 0, items: 0, changed: false, action: "no-source", missing_sources: []}` — `missing_sources` is empty by construction when `sources` is `None`, so no surface complains. Confirmed twice on the real child: the 2026-10-25 cruiserintelligence catch-up and the 2026-10-26 sdm1 catch-up, both 0-file on `{"project": "<p>"}` and full-count on explicit sources. So every corpus-refresh driver passes `sources` explicitly as ABSOLUTE globs of the project root: `<repo>/docs/**/*.md`, `<repo>/CLAUDE.md`, `<repo>/TECHNICAL-DOCUMENTATION.md`, `<repo>/FUNCTIONAL-SPECIFICATIONS.md`. The seeded task string in `src/cron.rs` `seed_task` still names only the project, so a run that follows it literally does the no-op; the defect is logged as KI-316 and is not fixed yet. A missing entry in the passed set (a doc that does not exist) comes back named in `missing_sources`, which is the signal to trust — an empty `missing_sources` beside `source_files: 0` is the trap.
Retired 2026-10-26: Duplicate: the parallel 2026-10-26 catch-up session wrote and committed the same insight as `blend1-refresh-needs-explicit-sources-from-the-store` (commit c8ea8c2) while this session was writing this one, and theirs carries the extra resolution — derive the sources from the store's own manifest keys. Two facts for one mechanism would split the recall.
