---
name: a-budget-grid-sizes-its-column-to-max-content
description: "feedback 2026-10-30 — `.budget { display: grid }` with no column template sizes the implicit column to max-content, so one long nowrap sentence widened every row past its card (6,200px row in a 604px card); minmax(0,1fr) fixes it, and the name takes a fixed basis so the value, not the name, is what shrinks"
type: feedback
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

Feedback 2026-10-30 (fleet, the skills/memory rail builds — caught by the
Chromium screenshots, not the harness, whose stub has no layout). The
row-budget wrapper was `.budget { display: grid; gap: 2px; }` — no
`grid-template-columns`. An implicit grid column sizes to MAX-CONTENT, so
a one-line `cfg-item` whose sentence ran 6,200px wide sized the column to
6,200px and every row with it, overflowing the 604px card. Settings' rows
never tripped it only because their values are short. Fix: one line,
`grid-template-columns: minmax(0, 1fr)`.

The second half: after the width fix, the long value out-competed the
name for flex shrinkage and names rendered as "accura…" — the row needs
`.lib-card .cfg-item-name { flex: 0 0 220px }` so the SENTENCE is what
shrinks, never the name.

**Why:** the whole chain looked right (min-width:0 on the value, minmax
on the grid column above) — the defect was one level up, in the budget
wrapper's own sizing context; only a live measurement (offsetWidth 6200
vs the card's 604) named it.

**How to apply:** a grid with auto-placed children and no template is a
max-content sizer — give it `minmax(0, 1fr)` when children carry
nowrap/flex content; in a name+value flex row, the name takes a fixed
basis. Measure the real chain's offsetWidth/scrollWidth when a screenshot
shows overflow — see [[a-layout-claim-is-measured-not-asserted]].
