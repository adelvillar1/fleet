---
name: dev-decisions-corpus-state-2026-10-23
description: "dev-decisions' blend1 corpus is 79 items over 10 source files and was unchanged by the 2026-10-23T00:50Z catch-up refresh; its three declared root docs do not exist and ride missing_sources on every run."
type: project
date: 2026-10-23
basis: inferred
tier: durable
shape: [state-sync, resolved]
phrases: [corpus-refresh-dev-decisions, mcp__blend1__refresh, tmp/blend1_refresh.py dev-decisions, action unchanged, missing_sources, ~/.local/share/blend1/vectors/dev-decisions]
missing: []
recall-count: 22
superseded-by:
valid-until: 2026-10-31
---

The 2026-10-23T00:50Z catch-up run of job corpus-refresh-dev-decisions (the window was missed while fleet was down) drove the established spawn route — python3 tmp/blend1_refresh.py dev-decisions '/Users/alejandrodelvillar/Projects/dev-decisions/docs/**/*.md' plus the three root docs — through fleet's own launch spec (BLEND1_PYTHONPATH + the four sibling path vars, cwd services/blend1, stdio JSON-RPC initialize → tools/call refresh), because mcp__blend1__refresh is not natively callable in this harness (KI-221/KI-277).

Report: {project: dev-decisions, source_files: 10, items: 79, changed: false, action: "unchanged", store: ~/.local/share/blend1/vectors/dev-decisions, missing_sources: [CLAUDE.md, TECHNICAL-DOCUMENTATION.md, FUNCTIONAL-SPECIFICATIONS.md]}. The corpus did NOT rebuild: no file under docs/ has an mtime later than the store's own Oct 20 19:55 write, and the store is byte-identical before and after — manifest.json sha256 b6c8240a3b09dc90, vectors.bin fb7b2ff4fe0081d8, both mtime 1792544117, manifest count 79 over 79 entries, dims 768, provider llama-server, chunk keys absolute paths under the dev-decisions repo.

The three root contract docs are genuinely absent from /Users/alejandrodelvillar/Projects/dev-decisions (ls shows LICENSE, README.md, SKILL.md, config.example.toml, a stray file named dev-decisions, docs, hooks, infographic, scripts), so source_files is 10 of the 13 declared and missing_sources names them on every run — the same benign pattern as gen1 and sem1, and the behavior KI-225's closure introduced (named rather than silently skipped).

Job record: ~/.fleet/cron/jobs.json's corpus-refresh-dev-decisions entry was rewritten from last_run 2026-10-22T00:50Z to 2026-10-23T00:50Z with last_outcome "completed", done under ~/.fleet/cron/jobs.lock with a temp-file rename, matching cron.rs record_run(minute, outcome) which stamps the minute the run was FOR rather than the wall clock — the convention the eight sibling jobs' 2026-10-23 stamps already follow. Backup of the pre-edit store was kept at /tmp/jobs.json.bak-*.
Retired 2026-11-01: Its statement ("is 79 items over 10 sources") is stale: the Oct-28 catch-up rebuilt the corpus to 142 items / 12 sources and the 2026-11-01T00:50Z scheduled refresh re-verified that state (store byte-identical to the Oct-28 build). Superseded-by corpus-dev-decisions-2026-11-01 via the corpus-dev-decisions-rebuilt-2026-10-28 link's own record.
