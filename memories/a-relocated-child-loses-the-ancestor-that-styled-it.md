---
name: a-relocated-child-loses-the-ancestor-that-styled-it
description: "feedback 2026-10-30 (KI-373) — moving the dock's cards into the pane dropped the `class="dock rail"` ancestor, so `.rail .card`'s padding stopped applying and the text rendered flush against the frame and under the resize grip; when dissolving a container, inventory every style rule that keyed off it, and measure the moved element's box"
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

2026-10-30 the owner reported the right pane's text "pinned to the left side …
parts seem to be being covered by some sort of edge or margin". It was a
regression from the previous day's pane unification (d20c257, see
[[the-right-pane-2026-10-29]]): the dock's element carried
`class="dock rail"`, and **`.rail .card`** supplied the cards' 16 px padding,
hairline, 12 px radius and ground. When the dock retired and its four cards
moved into the pane's run tab, the `.rail` ancestor went with it and the cards
rendered with **zero padding** — text flush to the pane frame, and the first
~5 px of every line under the resize grip (`#pane-grip` at `left:-4px; width:
9px`, which overhangs inside the pane).

Measured in Chromium before the fix: card left 1001 vs pane left 1000,
`padding-left: 0`, while the pane's head and tabs were inset 12 px. After:
`.rail .card, .pane .card` shares one box rule and `#pane-run { padding: 8px
12px }`; card 1013–1428 inside pane 1000–1440, grip ending 1006, card padding
16 px. Swept all four tabs — no text leaf in ANY of them starts left of the
grip.

**Why:** dissolving a container is not only a DOM move. Descendant-selector
styling is invisible at the moment of the move — the markup looks right, the
harness's DOM stub has no layout engine, and the only thing that notices is a
measured box on the real page. Same family as
[[a-layout-claim-is-measured-not-asserted]] and
[[the-screenshot-is-the-visual-review]]: a green harness plus plausible markup
is not evidence the pixels are right.

**How to apply:** before dissolving a wrapper (`aside`, `section`, a class-only
div), grep the stylesheet for `.thatClass` as an ANCESTOR of anything that
survives — every descendant rule keyed off it either moves to the new home
(declare the box for both: `.rail .card, .pane .card`) or is deliberately
retired. After the move, measure the relocated element's `getBoundingClientRect`
and computed padding against its new container's edge, in real Chromium, and
sweep every sibling tab/panel for the same class of defect before saying it is
fixed. Keep the rule pin in the harness as the regression guard — it proves the
shared rule exists, never that the pixels line up.

**Its second form, the same day (2026-10-30, the settings/bots rail): a copy is
not a share.** Writing a second rail for the section surfaces, I derived
`.section-rail-row` from `.surfaces-link` and drifted on every metric — height
34 px vs padding-driven, gap 10 vs 9, radius 8 vs 7, type 13 px/500 vs 12 px/400,
icon box 18 vs 16, transitions dropped — and **replaced the current-row
indicator** (raised ground + inset hairline + the 2 px accent bar at the left
edge) with an accent wash. The owner caught it: *"remember to keep consistency
on the design language of the app for both surfaces."* The fix is the same shape
as the ancestor case: declare one rule for both (`.surfaces-link,
.section-rail-row { … }`, `.surfaces-count, .section-rail-count { … }`) rather
than a parallel one, and extend the no-new-token harness assertion to the copy.
A DOM move and a hand-copy fail identically — the styles look present in each
file while the surfaces stop agreeing.
