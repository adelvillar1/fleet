---
name: fleet-second-brain-is-seven-fields-in-the-gui
description: "2026-10-20 — the parse-gap findings behind the owner's second-brain request (boards.rs keeps 7 of ~25 fields of tower_data.json; attention queue is a reduction; every derived file 11+ days stale while events.jsonl is live); superseded as a design by fleet-second-brain-is-a-tool-plugin"
type: project
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

**SUPERSEDED AS A DESIGN, 2026-10-20 (same session):** the owner then ruled
second brain is "a separate tool that needs to be treated as a plugin in fleet,
but the plugin should offer actions within it that allow the user to refresh
the data" — the built seam is [[fleet-second-brain-is-a-tool-plugin]]. Keep
this file for the measured parse gaps and the attention reduction, which remain
true and still unshipped; do not read the five-section proposal below as the
plan.

The owner's 2026-10-20 request: "regarding second brain, as implemented in the
GUI is missing a lot of information. I essentially want what I have here but
better organized, http://127.0.0.1:4175/ — how can we make this work on fleet."

**What 4175 is.** `~/.second-brain/control_tower.py serve 4175` (cwd
`Projects/second-brain`) serving a STATIC `site/` directory (Python
SimpleHTTP, no templating): index (attention queue, pulse, timeline, parked &
shelved, reference repos), plus `knowledge.html`, `maturity.html`, `loops.html`,
`gates.html`, `reports.html`, `conflicts.html`, and `project/<name>.html` per
project. Routes `/src/projects|skills|codex|second-brain/` serve raw source;
`POST /api/run`, `/api/conflict`, `/api/lifecycle` are the write-backs (X-Tower
header CSRF guard, dispositions need Postgres via `db.py`).

**The finding that shapes the work: fleet already reads the whole picture and
throws most of it away.** `~/.second-brain/tower_data.json` (621 KB) carries
28 own projects with role, origin, `lifecycle_note`, `drift_days`,
`live_handoff` (next score + `next_lines`), `in_progress`/`planned` rows with
title, path, box counts and `stale_days`, `feat_timeline`, tech/spec
summaries, 400 timeline rows and 28 conflict catalog rows. `src/boards.rs`
(`parse_tower_project`, `parse_tower_data`) keeps only name, lifecycle,
branch, last_commit, commits_30d, dirty, plan status counts — and hard-caps
the timeline with `.take(8)` (=43) and conflicts with `.take(6)` (=43) where
400 and 28 exist. `paintBrain` (`src/gui/assets/app.js`, ~5862) renders only
the four method sections (gates, loops, harness, maturity) plus four count
tiles; `paintTower` (~5436) is the console. Nothing in the browser shows the
attention queue, plans, handoffs, project identity or the conflict board.

**The attention queue is a reduction, not a port.** Its rule is in
`control_tower.py:1944`/2133–2136: pool = role ≠ reference and lifecycle in
(active, dormant); rows = projects with `drift_days` + in-progress plans with
`stale_days >= 21` + handoffs with `next >= 0.6` + the open conflict count —
33 items reproduced exactly from tower_data.json. A pure
`attention(&TowerDataRead, &ReportsRead)` in boards.rs serves GUI and TUI from
one source.

**Freshness splits from completeness, and the owner's reference page is the
stale one.** Built 2026-10-09 01:52 from inputs dated 10-04/10-07/10-08;
`events.jsonl` holds 818 entries filed on or after 2026-10-04 with the newest
at 20:24 that day. So a faithful port inherits an 11-day-old picture. Options
given: read-only + per-section `generated_at` and a banner when the ledger is
newer; a rebuild action shelling `control_tower.py build` (adds that repo CLI
and Postgres for dispositions); or derive plans/recaps in Rust (drops the
classifications). Recommended the first now, the second later.

**The proposed surface: one knowledge home, five sections** — Attention
(default, the 33 grouped by class, severity then age), Projects (with chips for
needs-attention/active/parked/reference and the existing drawer deepened to
identity, open plans, shipped timeline, handoff next actions), Evidence (the
existing tower console plus the real conflicts card), Method (gates, loops,
harness, maturity), Library later (plans, recaps, lessons, needs-review — lives
in `knowledge_docs.json`, 5 MB, so it needs a bounded read or a precomputed
slice and cannot ride the 60s cadence). One-level rail, sticky section nav:
the 4175 page's own weak point is 33 identical cards in one long column.

**Build order:** P1 attention + projects (the data is already on disk), P2
timeline/conflict truncation + tower consolidation, P3 library, P4 freshness.
P1 is a single increment: widened parse, one tested reduction, one payload,
one panel rewrite. I stopped at the shape because the section layout is a
design call — but note this session's pattern: the owner later scolds for not
shipping.

**2026-10-20 update — superseded twice over, but the parse gaps are now the
redesign's material.** The plugin seam exists
([[fleet-second-brain-is-a-tool-plugin]]) and the owner then called the shipped
panel "rather underwhelming" visually; two mockups built from this file's
measured fields are awaiting a pick
([[fleet-second-brain-visual-redesign-awaiting-pick]]). The widened parse + the
pure `attention()` reduction described here are still the backend half of that
work.

**How to apply:** don't port the Python page; widen the read. Every displayed
section must name its build time and the staleness check between
`events.jsonl` and `generated_at`. This work is blocked behind the owner's
open plan-first directive ([[plan-brainstorming-before-development]]) and the
plan must carry ASCII mockups ([[fleet-gui-editorial-desk-redesign]]). Related:
[[gui-revamp-browser-face-decision]], [[fleet-tui-revamp]],
[[verify-the-binary-is-newer-than-the-source]] (any attempt to show it needs a
rebuild + process restart), [[fleet-gui-editorial-desk-redesign]].
