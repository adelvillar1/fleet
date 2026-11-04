---
name: a-layout-claim-is-measured-not-asserted
description: "feedback 2026-10-29 — the queue-text overflow fix: the textbook CSS cure (min-width: 0) passed every rule pin while the layout stayed broken, because the ROOT was the chip's stale align-items: center sizing the column card's list to content; a layout claim is proven by a Chromium measurement of the real chain, not by CSS-rule assertions"
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

2026-10-29 the owner reported queued-message text "overflowing the message
and extending outside of… the composer" (commit `f8ce79b`). The fix took
three passes, and the first two were wrong:

1. The textbook cure — `min-width: 0` on the text (`.queue-text`) — is
   necessary but NOT sufficient: the chain list→row→text is ALL flex
   items, and EACH link's auto minimum re-inflates the width. The list and
   the row needed it too.
2. Even with the whole chain at `min-width: 0`, the row still measured
   **4418 px** inside a 506 px chip. Computed-style measurement found the
   root: `.queue-chip` still carried **`align-items: center`** from its
   one-line-pill days — the component was redesigned into a column card
   and the old rule survived, and a NON-stretching column-flex item sizes
   to its content. `align-items: stretch` was the fix.

**Why:** CSS-rule pins in the DOM harness were green through all of it —
they prove a rule exists, never what the pixels do ([[gui-client-verified-
by-a-vm-harness]]'s own law, confirmed the hard way: my first instinct was
the textbook fix and it did NOT fix it). A component that changes shape
carries its old layout rules silently; they fight the new design exactly
where no selector war is visible.

Third form, 2026-10-30 (KI-373): the pane's cards lost their whole box (0 px
padding, text flush to the frame and under the resize grip) because the dock's
`class="dock rail"` ancestor did the styling — see
[[a-relocated-child-loses-the-ancestor-that-styled-it]]. A DOM move is a style
change: measure the moved element's rect and padding against its new container.

**How to apply:** for any layout defect the owner can SEE, verify with a
real-Chromium measurement of the actual chain — `page.setContent` with the
real stylesheet plus a DOM stand-in, then getBoundingClientRect/scrollWidth
per link (probe shape: `tmp/queue-overflow-probe.mjs`; playwright from
`~/Projects/design-canvas/node_modules`, chrome-headless-shell as
executablePath). State the measured chain in the commit. Keep the CSS-rule
pins as the regression guard, but never as the proof. And when a flex
container's child overflows: check `align-items` for a stale non-stretch
value, then `min-width: 0` on EVERY item in the chain, not just the one
holding the text.
