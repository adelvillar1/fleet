---
name: collapsed-rail-icons-align-left
description: "Owner ruling: the collapsed rail's ICONS align LEFT (flex-start) with badges pushed to the row's far end by margin-left: auto — my flex-end reading was wrong and the face check that pinned it had to be rewritten, not kept."
type: feedback
date: 2026-11-18
basis: confirmed-by
tier: durable
shape: [rendering, resolved]
phrases: [the rail icon alignment when collapsing it, flex-start not flex-end, "margin-left: auto on the badge", the icons are what the owner scans, left alignment]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

The owner's correction, verbatim: "the rail icon alignment when collapsing it, I asked you right alignment when what I wanted was left alignement (apparently I can tell left from right)".

What I built from the 2026-11-18 ruling was `justify-content: flex-end` on the collapsed rail's grouped row rule (`.mark`, `.surfaces-link`, `.surfaces-palette`, `.surfaces-toggle`). The ruling as quoted talked about the badge-carrying rows lining up and the icons sharing an edge; I read "share an edge" as the RIGHT one and wrote a whole face case that pinned it, with the owner's words in the comment — so the defect became permanent and cited them for it.

The correct design: the ICONS share the LEFT edge (`flex-start`) because the icons are what the owner scans; the counts ride the far end of their own row, pushed by `margin-left: auto` on the badge, not by a fixed gap. With a fixed gap every row carrying a count pulls its icon off the shared edge — the same defect wearing a smaller number.

The generalizable failure: when an ask names a direction, the direction belongs to the SUBJECT the owner is scanning, not to whatever element happened to be misaligning. And a check written against the wrong reading is worse than no check — it must be REWRITTEN, not kept, when the direction is corrected.

Measured in obscura at the collapsed width with a badge slot present: every icon at x=14 after the fix, all of them at x=57 under `flex-end`.
