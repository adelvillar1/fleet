---
name: blend1-refresh-needs-explicit-sources-from-the-store
description: "A corpus-refresh task followed to the letter returns no-source for every project: the lane's default source set is cwd-relative under services/blend1/, so the refresh call must carry the sources derived from the store's own manifest keys."
type: project
date: 2026-10-26
basis: inferred
tier: durable
shape: [state-sync, fail-open, resolved]
phrases: [action: no-source, missing_sources, sources derived from the store manifest keys, docs/**/*.md plus the three root contract docs, BLEND1_SEM1_PYTHONPATH, seed_task]
missing: []
recall-count: 0
superseded-by: corpus-refresh-tasks-declare-explicit-sources
valid-until:
---

The seeded `corpus-refresh-*` task string says to drive `tools/call` with name `refresh` "with the project <project>" and names no `sources`. Executed literally that derives nothing and returns:

`{"source_files": 0, "items": 0, "changed": false, "action": "no-source", "missing_sources": []}`

Why: over MCP the child's cwd is the lane's home (`services/blend1/`), and `_source_files` with `sources=None` does `matched = list(Path("docs").rglob("*.md"))` plus `DEFAULT_ROOT_DOCS` checked as `Path(name).is_file()` — all relative to that cwd, which holds no docs. `missing_sources` is `[]` by construction on the default path (it is only populated when `sources` is given), so an empty derivation reads as a clean report. The failure is a fail-open: a run that follows its task exactly records success over an empty corpus.

The working route (what every successful catch-up does): pass `sources` explicitly as absolute paths/globs, recovered from the store's own manifest, whose keys are absolute `<file>#<heading>` after the KI-223 fix:

1. read `~/.local/share/blend1/vectors/<project>/embeddinggemma-2/manifest.json`, split each `entries[].key` on `#`, and take the distinct file paths;
2. those resolve to the project root's `docs/**/*.md` plus the three root contract docs `CLAUDE.md`, `TECHNICAL-DOCUMENTATION.md`, `FUNCTIONAL-SPECIFICATIONS.md` — send them as a `sources` array with the `project`;
3. the same spawn spec still applies: `python3 -u services/blend1/service/mcp_server.py`, cwd `services/blend1`, env `PATH`/`HOME`/`LANG`/`TMPDIR` plus `BLEND1_PYTHONPATH` and the four `BLEND1_<SVC>_PYTHONPATH` siblings (`sys1`, `sem1`, `sdm1`, `gen1`).

Verified 2026-10-26 on `cruiserintelligence`: project-only → `no-source` in 0.10 s; the same child with the explicit four-entry source set → `source_files: 40, items: 334, action: unchanged`. Logged as KI-316 (`docs/known-issues.md`), open, with the task string or the no-source report as the two candidate fixes.
