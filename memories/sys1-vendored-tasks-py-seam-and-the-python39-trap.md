---
name: sys1-vendored-tasks-py-seam-and-the-python39-trap
description: "Fleet's vendored services/sys1/src/sys1/tasks.py is THE seam for new sys1 head sets (tool_route/curate/desk_coverage precedent) — and module-level make_score crashes the import on the system Python 3.9 the lane's own tests pin"
type: project
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Verified 2026-11-04, building the desk's coverage beat. The vendored
sys1 copy's `tasks.py` is how Fleet gives sys1 a new head set: add a
`Task(...)` and register it in the bottom tuple — the
`tool_route`/`curate` tasks were Fleet's own additions there, and
`desk_coverage` (anchored 1–5 coverage score + `material_gap` +
`off_brief` nouls) joined them the same way. `sys1_classify` is then
called with `{"task": "<id>", "text": <state>}` over MCP.

**The trap: `make_score` is not import-safe on the system Python.**
`types.py:make_score` builds its legend with
`dict(zip(labels, legend, strict=True))` — Python 3.10+ — and tasks.py
executes head builders at MODULE level, so any module-level
`make_score(..., legend=...)` kills the whole `sys1` import under
`/usr/bin/python3` (3.9.6, which the perception tests pin via
`env PATH=/usr/bin:/bin:...`). Symptom: every `perception::tests::*`
red with "EOF while parsing a value" (the child's empty stdout), and
the real error only shows by running the child's exact python command.
`make_choice`/`make_noul` are 3.9-safe; only `make_score`'s strict zip
is not. The cure: construct the score `Head` directly (labels
`"1".."5"`, the descriptions dict spelled out, score_min/score_max
set) with a comment naming why — a legend-length mismatch now raises
at dict-build instead of silently truncating. The blend1 startup guard
(`zip(strict=)` named at startup, KI-387's note) is the same class.

**Why:** the sys1 lane runs on whatever python3 the PATH resolves — the
system one is 3.9 — and a vendored-file edit that imports fine under
the brew Python 3.15 still reds the whole suite.

**How to apply:** for any new vendored-task work, verify the import
under the SYSTEM python (`env -i PATH=/usr/bin:/bin:... python3 -c
"from sys1.tasks import get_task; …"`) before running cargo tests;
build score heads directly, never via make_score at module level. See
also [[lanes-estate-verified-2026-11-04]].
