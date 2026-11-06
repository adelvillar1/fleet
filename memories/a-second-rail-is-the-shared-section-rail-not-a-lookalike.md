---
name: a-second-rail-is-the-shared-section-rail-not-a-lookalike
description: "feedback 2026-10-31 — told \"a rail like sessions does\", I hand-wrote a bespoke .design-rail with work-row rows and a head picker; the owner corrected it to \"the same design language of this and the other pages that have the second rail\" — a page with a second rail renders the shared sectionRail(), never a twin"
type: —
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

**2026-10-31 (the design canvas's boards rail).** The owner asked for
*"a rail like sessions does, where it should show all the designs the
user has done"*. I built a **lookalike**: a `.design-rail` aside with
`work-row` rows borrowed from the work list, its own head with a
`+ new` button, its own 232px column and CSS, plus a `<select>` board
picker in the page head. Then the correction landed:

> *"it should be on the same design language of this and the other pages
> that have the second rail."*

The pages with a second rail — settings, calibration, memory, skills,
bots, workspace-history — all render **one component**: `sectionRail()`,
which carries the fold chevron and its remembered state, `aria-current`
on the open item, arrow-key roving between rows, the icon mark, and a
count badge. The rewrite is that call plus a `.stage-canvas` split
layout that **extends** the shared `.stage-split` / `[data-folded="true"]`
selectors rather than cloning them.

**Why the lookalike happens:** "like sessions does" is a request about
*which page has the behaviour*, and the nearest visible precedent is the
work list's rows. But a page with a second rail is not the work list; it
is a split page, and the rail on a split page is a shared contract.

**How to apply:** when a surface gains a second rail, reach for
`sectionRail(container, declaration, {current, folded, store, onPick,
onFold})` and give it a per-page `store` key — do not hand-roll an aside,
do not import another surface's row class, and do not keep a second
switcher (a head `<select>`) beside it: two switchers for one choice is
the dupe the correction is about. One metric may differ and must be
**justified in the CSS**: `.stage-canvas` uses 216px rather than 188
because rows carry the owner's data (board names) rather than one-word
section labels, and the first render at 188 clipped "Ember & Oak —
Prici…". Row metrics (34px, 13px, 10px gap) never differ.

**Prove it with a measurement, not an assertion:** the Chromium probe
read `getComputedStyle` on the new rail's row and got the siblings'
numbers (34px / 13px / 10px), plus the fold toggle's presence and the
rail width — that is what makes "the same design language" a fact rather
than a claim. See [[a-layout-claim-is-measured-not-asserted]] and
[[the-pane-card-is-live-the-board-pair-was-dead]].
