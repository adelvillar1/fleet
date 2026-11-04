---
name: fixed-height-button-clips-a-shrunk-label
description: "feedback 2026-10-30 — flex-shrink squeezed three buttons past their labels' width inside fixed-height buttons, so the text wrapped and the second line clipped; button labels need nowrap + flex 0 0 auto and the container needs flex-wrap"
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

Feedback 2026-10-30 (owner-reported: "the text is overflowing the buttons" on the calibration stream card) — the instrument's three actions sat in `.surface-tool-act { display: flex }` with **no flex-wrap**, so when the card narrowed, flex-shrink squeezed each button below its label's natural width; the labels wrapped INSIDE the fixed-`height: 28px` buttons and the second line clipped at the button's own edge. The visual read ("text overflowing the buttons") pointed at the card, but the mechanism was per-button.

**Why:** a fixed-height flex child with no `flex-basis` floor and wrapping text turns a container-width problem into a per-button text clip; the card looked fine at wide widths and only broke where three labels plus gaps exceeded it.

**How to apply:** button labels are atomic — `white-space: nowrap` and `flex: 0 0 auto` on the button, `flex-wrap: wrap` + `row-gap` on the container, and let the card grow (`height: fit-content` inside a grid head whose row height follows content). When the owner reports text "overflowing" a control, measure the CONTROL's own box at the failing width first — the defect was invisible at 1440 with two buttons and only the shared card's third action exposed it. Related: [[a-layout-claim-is-measured-not-asserted]], [[a-budget-grid-sizes-its-column-to-max-content]].
