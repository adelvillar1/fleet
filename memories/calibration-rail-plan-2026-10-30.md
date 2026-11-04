---
name: calibration-rail-plan-2026-10-30
description: "project 2026-10-30 — the calibration revamp LANDED (8e6eea5): the gate section first, OPS widened 10→13 (the parse itself had dropped verify/embed_timing/lifecycle, not just the face), the eval corpus's 20 scenarios, the day stepper + ?day=; 2,860px/59ctl → gate 350px, busiest section 902px/21; harness 468/49, suite 1,821"
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

Owner direction 2026-10-30: *"one more surface to go using the same
pattern, let's do a deep dive of calibration and then use the rail to
improve organization and usability. plan first."* Plan drafted, gates run
(plan-gate 12/12, arch-gate PASS), the owner said "implement plan as
designed", and the whole plan **LANDED as `8e6eea5`** — all ten criteria
T1–T10 ticked with evidence in the plan file. The knowledge group's five
long pages (settings, bots, skills, memory, workspace history,
calibration) all live on the rail idiom now.

**What landed:**

- **The gate leads** — the failover state, the fit rule in its own
  numbers, and one-line fit-cell rows whose press opens the pane card
  (counts, weighted rates, spend, ask-sys1, tabs stood down); the
  three-chip filter retired.
- **The plan's one wrong assumption, corrected in the build:** the deep
  dive said the FACE dropped four ops, but the PARSE itself dropped
  three — `calib::OPS` carried only ten, so `verify`, `embed_timing`,
  `lifecycle` never reached any reader. `OPS` widened to the full
  thirteen, `ReadRow` gained their fields (node/gates/verified, ms/facts
  — all serde-defaulted, every other row parses unchanged), `row_json`
  ships the four arms, and `fleet calib` gains the groups beside.
- **The eval corpus readable** — the eval section renders the last
  run's aggregate plus the twenty scenarios as rows, pass badges,
  failures named.
- **The day at hand** — `calibration_read_at(home, day)`, `?day=` on the
  route, a stepper in the instrument (the day before / after), an empty
  day honest; `#/calibration/<op>` addresses a section (the OP WORD is
  the id, the display name is not).
- **Measurement:** before 2,860 px / 59 controls / 8 cards / 55 rows on
  one scroll → the gate 350 px / 3 controls, the busiest section (embed
  timings, 18 rows) 902 px / 21, one section body at every address. The
  live rail carried nine sections on the first serve. Harness 468
  checks / 49 cases; suite 1,821 green.

**Traps this build named:** the server-test `get()` helper hard-appends
`?token=` to the path, so a second query param must ride `ask()` with an
explicit `&token=` join (the day value swallowed `?token=` whole); and a
probe navigating by a section's DISPLAY name finds nothing — sections
are addressed by their id.

**How to apply:** when a deep dive says "the surface drops X", check
whether the read drops it first — the plan's file-level fix assumed the
substrate carried what only the face did; widening a shared op list is
safe when the row struct's new fields all default; and the rail idiom's
fifth landing confirms the pattern generalizes without re-litigation.

Related: [[workspace-history-rail-plan-2026-10-30]],
[[memory-rail-plan-and-the-substrate]],
[[skills-rail-plan-and-the-approve-defect]],
[[a-closed-pre-filter-reader-needs-its-own-reader]] (the earlier narrow
lesson this widening answered at the other end),
[[fleet-settings-abstraction-search]].
