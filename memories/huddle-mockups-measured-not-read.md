---
name: huddle-mockups-measured-not-read
description: "project/feedback 2026-11-03 — the four huddle mockups were verified by in-page measurement (overflow, clipping, 44 px targets, mark contrast) rather than by reading the PNG, because the session's model cannot see images; the method caught three real defects, and the owner has since READ them — \"not particularly blown away, but we will iterate later\" (C1 ticked, iteration deferred to the owner)"
type: feedback
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 2
superseded-by:
valid-until:
---

Phase 0 of the huddle plan ([[huddle-plan-drafted-2026-11-03]]) drew four
static mockups and verified them **programmatically**, because the session's
model cannot read images — so the PNGs were rendered but never visually read.

**The method that worked:** inject a small script into each page (via
`--dump-dom`, reading a `<title>`/`<pre>` the script writes), then measure
`getBoundingClientRect()` for every element against the viewport — nothing
past the right edge, no `overflow:hidden` element whose `scrollWidth` exceeds
its `clientWidth`, no row or primary action under 44 px. Persona-mark contrast
was computed from the computed `color` against the composited disc background.

**Three real defects it caught that a bare render would not have:**
1. **The stylesheet silently did not load.** The measurement copies lived in
   a `tmp/` subdirectory, so the relative `mockup.css` link 404'd and every
   measurement ran against unstyled HTML — rows read 18 px tall and the body
   at 16 px. The tell was `document.styleSheets[0].cssRules.length` being 0
   and the row's computed padding being `0px`. **A measurement pass whose
   stylesheet is missing reads as a passing layout.**
2. **The persona marks were all one colour.** `.mark[data-p="x"]` set only
   `--tint` (the disc wash) and never `--tint-ink`, so every glyph fell back
   to the accent — blue on blue, and colour stopped being a recognition
   channel. Fixed with a per-persona ink; measured 7.0–9.2:1 on the opaque
   disc.
3. **Primary controls were under 44 px.** `.press` was 9 px padding (~33 px
   tall) and the utility pills 21–22 px. Raised to a 44 px minimum, which
   also tightened C1's wording from "controls ≥44 px" to "every row and
   primary action ≥44 px" — a disclosure link is not a touch target.

**The limit, stated plainly:** measurement proves no overflow, no clipping,
no undersized target, and AA contrast. It does **not** prove the numbers
cohere, the element sits under the element it belongs to, or the copy reads
right — the exact defects
[[a-visual-coherence-check-is-reading-the-png]] found in the settings
mockups. **The owner has now read all four screens** and their verdict is
*"not particularly blown away, but we will iterate later"* — so C1 is ticked
with the read recorded, and the composition's iteration is the owner's to
direct rather than a defect this session should guess at.

**Why:** the session could not see the images, so the honest move was to
measure what is measurable and say what is not — rather than claim a visual
pass. **How to apply:** when a session's model cannot read images, verify
layout/contrast programmatically and hand the PNGs to the owner as the
visual gate; never report a mockup as visually reviewed when only geometry
was checked. And when a measurement pass reports clean, first confirm the
stylesheet loaded — a missing CSS makes every layout assertion vacuous.
Related: [[a-visual-coherence-check-is-reading-the-png]],
[[the-screenshot-is-the-visual-review]],
[[chrome-screenshot-fires-at-load]] (the capture traps this session's
`--screenshot` run avoided by using static files).
