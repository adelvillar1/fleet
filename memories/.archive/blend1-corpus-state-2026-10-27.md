---
name: blend1-corpus-state-2026-10-27
description: "blend1's project corpus is 84 items over 10 source files, unchanged by the 2026-10-27T03:47Z catch-up refresh for the missed 2026-10-27T00:35Z window, with the source set passed as explicit absolute globs because the lane-home cwd derives nothing."
type: project
date: 2026-10-27
basis: inferred
tier: durable
shape: [state-sync, resolved]
phrases: [mcp__blend1__refresh, tools/call refresh with project and sources, action "unchanged", services/blend1/service/mcp_server.py, BLEND1_PYTHONPATH plus the four sibling *_PYTHONPATH vars, store manifest absolute keys]
missing: []
recall-count: 0
superseded-by:
valid-until: 2026-10-27
---

Project `blend1`'s fact corpus is 84 items over 10 source files — 7 `docs/**/*.md` (5 plans + 2 recaps) plus `CLAUDE.md`, `TECHNICAL-DOCUMENTATION.md`, `FUNCTIONAL-SPECIFICATIONS.md`, all present in `~/Projects/blend1` — and was unchanged by the 2026-10-27T03:47Z catch-up for the missed 2026-10-27T00:35Z window.

The run: the vendored lane child launched from `services/blend1` on the `src/lanes.rs` spec (BLEND1_PYTHONPATH plus BLEND1_{SYS1,SEM1,SDM1,GEN1}_PYTHONPATH; env limited to PATH/HOME/LANG/TMPDIR), line-delimited JSON over stdio (KI-318), initialize protocolVersion 2024-11-05 answered serverInfo {"name": "blend1", "version": "0.1.0"}, notifications/initialized, then two tools/call frames on the one spawn spec.

Call 1, the task-literal `{"project": "blend1"}` from the lane-home cwd: `{"source_files": 0, "items": 0, "dropped_duplicates": 0, "changed": false, "action": "no-source", "missing_sources": []}` — KI-316, this project's second datum.

Call 2, the store's own recovered sources absolute (`~/Projects/blend1/docs/**/*.md` + the three root contract docs): `{"project": "blend1", "source_files": 10, "items": 84, "dropped_duplicates": 0, "changed": false, "action": "unchanged", "store": "~/.local/share/blend1/vectors/blend1", "missing_sources": []}`.

Why the sources must be explicit: the default set is cwd-relative and the child's cwd is `services/blend1`, which holds no docs; the source set was recovered from the store manifest's absolute keys, which name `~/Projects/blend1` — the corpus's repo — not the vendored services copy.

Corroboration beyond the report: manifest.json and vectors.bin kept mtime 2026-10-19T19:36:28Z and sha256 `0ab7c1d5…` / `8522c594…` (84 entries, 768 dims); every source predates the build (newest `CLAUDE.md` 2026-10-14T22:54:38Z), and `unchanged` is the lane's set-equality over (key, sha256) — a real no-op, not KI-223's cwd artifact and not a silent stale pass. `docs/architecture/` holds only `.svg`/`.drawio`, which the `.md`-only filter rightly ignores.

Driver discipline this run: a project-scoped UTC-stamped unique name written and executed in one shell invocation then removed (KI-317), `/usr/local/bin` front-loaded so python3 is 3.13.6 rather than Xcode's 3.9.6 (KI-324), and a select() deadline per read so a saturated embedder fails the run instead of wedging it (KI-325). Evidence appended to KI-316.
Retired 2026-10-28: Superseded by blend1-corpus-state-2026-10-28 (written this run): the KI-339-reconciled job string widened the declared source set to the root *.md glob, so the 2026-10-28T00:35Z catch-up rebuilt the corpus to 94 items / 11 sources (README.md added) with a new store (sha256 73131f0c… / 4567b48e…, 2026-10-28 build). The 84 items / 10 sources / byte-identical-Oct-19-build state that fact recorded no longer describes the corpus.
