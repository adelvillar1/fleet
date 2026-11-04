---
name: a-fixed-width-row-pushes-the-pane-not-a-line
description: "Feedback 2026-10-31 — four knobs + send in a 319px chat panel rendered 668px wide because the row could not wrap, pushing the pane out of the viewport; flex justify broke before it wrapped, and natural widths then clipped labels"
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

**Feedback 2026-10-31 (caught by the 1440×900 render, not by the harness).** Adding the design model picker made the composer's single `.design-chat-row` four selects + a button. Measured in Chromium: the row was **668px inside a 319px panel** — `display:flex` with no `flex-wrap`, so it could not break a line and pushed the pane out of the viewport rather than growing taller.

**The two failed fixes before the right one:**
1. `flex-wrap: wrap` + `select { flex: 1 1 96px }` made the row wrap — but `justify` (no `margin-left:auto`) swallowed the free space so the selects broke onto three lines and `1 direction` clipped to `1 direct…`.
2. The *right* shape: **natural width first, shrink only as a fallback** (`flex: 1 1 auto; min-width: 0`), the model on **its own line** ("the model · deepseek-flash · send") because it is the run's route and a clipped model name is unreadable, and the send `flex: 0 0 auto`.

**Measurement that proves it:** `getBoundingClientRect()` of each select vs its `scrollWidth` (clipping) plus `document.documentElement.scrollWidth > innerWidth` (panes pushed out). Ends at 307px rows, model select 196px showing `deepseek/deepseek-flash`, `overflowX: false`, zero page errors.

**Third form, same session (T2):** a brand picker joined the model's row and both labels clipped again — `model · deepseek…` and `brand · no br`. The fix that held: **one concern per row** (brand on its own line, `model · deepseek-flash · send` on the next) and `flex: 1 1 120px` so a lone select in a row still shrinks before overflowing. Final measurement: five selects, every one `scrollWidth == clientWidth + 2`, rows 34/72/34/34px inside 900px, `overflowX: false`.

**Fourth form, 2026-11-02 (the settings account card's walk rows):** the same squeeze showed up in the PANE rather than the composer — three acts in a plain `display:flex; justify-content:space-between` row next to a long value made the workload name clip to `stand…` and `stop` stack into `st p`. The fix held the same way: a **grid** (`.cfg-walk`) with the name on its own full-width line, the order beneath it, and the acts in their own `auto` column (`flex: 0 0 auto` on each button), plus a `cfg-walk-add` wrapper class so the add row's two pickers wrap instead of pushing the pane. Caught only by reading the screenshot — the nine new harness checks were green against the clipped layout.

**How to apply:** in a narrow panel, a row of N controls is a wrapping problem before it is a sizing problem; check `scrollWidth` vs `clientWidth` per control, not only that the row exists — and the harness (`page.rows`, `page.IDS`) can never see a layout overflow, only the render can. Related: [[the-screenshot-is-the-visual-review]], [[a-layout-claim-is-measured-not-asserted]], [[fixed-height-button-clips-a-shrunk-label]].
