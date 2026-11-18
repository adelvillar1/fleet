---
name: collapsed-rail-packs-right-and-fits-its-badge
description: "The collapsed rail now packs every icon to one right edge and is 90px wide — sized once for icon + gap + a reserved 34px badge slot, so a count arriving never moves the icon and no row sits off-centre."
type: change
date: 2026-11-18
basis: confirmed-by
tier: durable
shape: [rendering, fixed-width-clip, resolved]
phrases: [the rail when collapsed show the icons that have number badges aligned to the right, collapse to the size that contains space for the badges, align the icons to the right and not centered, rail-width-icons, "justify-content: flex-end", surfaces-count flex 0 0]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

The owner's 2026-11-18 ruling: "the rail when collapsed show the icons that have number badges aligned to the right while the rest is centered, the rail should account for that to begin with and collapse to the size that contains space for the badges and align the icons to the right and not centered."

THE CAUSE, by measurement. `.surfaces-count` carries `margin-left: auto`, which wins over the collapsed rule's `justify-content: center`. So a row WITH a count had its badge pushed to the far right and its icon shoved left, while a row with no count sat dead centre — one rail wearing two alignments. And the arithmetic never fit: the old `--rail-width-icons: 66px` minus 16px section padding and 12px row padding left a 38px content box for a row needing icon (18) + gap (10) + a two-digit badge (~62) — the badge overran the row.

THE FIX, three parts:
1. `--rail-width-icons: 66px` -> `90px`. The arithmetic is in the comment and is checkable: section padding 2x8 + row padding 2x6 = 28px, plus icon 18 + gap 10 + badge slot 34 = 90px, leaving exactly a 62px content box for the 62px row.
2. The collapsed alignment is ONE grouped rule packing to `justify-content: flex-end` for `.mark`, `.surfaces-link`, `.surfaces-palette` and `.surfaces-toggle` — every row, badged or not, hangs off one right edge. (Centring was the bug.)
3. The badge's slot is RESERVED rather than a minimum: in icons mode `.surfaces-count` is `flex: 0 0 34px; min-width: 34px; box-sizing: border-box`, and a `[hidden]` count becomes `display: inline-block; visibility: hidden` so its box stays and the icon beside it never moves when a number arrives. That is the owner's "account for that to begin with" read literally — the column is sized for the badge once, not re-laid-out per row.

The only consumers of the variable are `--rail-actual` and `body[data-rail="icons"] .work-col { left: ... }`, so the widening propagates everywhere by construction; no stray 66px remains.

TESTING NOTE that matters for any future layout work: the face harness's stubbed DOM has NO layout engine — `offsetWidth` is a fixed 320 and there is no cascade — so a pixel claim in a case would be fiction. The honest pin is the CSS text plus the arithmetic derived FROM it: the harness case `theCollapsedRailAlignsOnOneRightEdgeAndFitsItsBadge` reads `--rail-width-icons`, the section/row padding, `.surfaces-icon`'s width, the base row's gap and the badge's flex basis out of app.css and asserts `content >= icon + gap + slot`. Two traps found while writing it: `ruleBlock(selector)` looks for `selector + ' {'` so it MISSES a comma-grouped rule (read the group by index instead), and an earlier `.surfaces-link { display: none; }` answers `ruleBlock('.surfaces-link')` first, so the base rule's gap must be read from its `display: flex` block specifically.

The checks were proved to bite: setting the width back to 66px fails with "content=38px need=62px", and restoring `justify-content: center` fails the alignment check.
