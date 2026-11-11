---
name: a-shell-inside-the-board-grid-needs-to-span-it
description: "feedback 2026-11-09 — the face's `.board` is a twelve-column grid, so a full-width shell dropped into it lands in ONE column and squeezes to a twelfth; the harness has no layout and only a screenshot read catches it"
type: feedback
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

Feedback 2026-11-09 — the desk/learning redesign shipped 0.6.14/0.6.15 with
`.pw-shell { display: grid; grid-template-columns: 216px 1fr 340px }` appended
as a child of `#desk-board` / `#learning-board`. The board is
`display: grid; grid-template-columns: repeat(12, minmax(0, 1fr))` — so the
shell became **one grid item in column 1**, a twelfth of the width: the section
chips stacked vertically and the thread overlaid the work column.

**Why:** every pin was green. The face harness is a flat id-map with no
geometry, so a broken layout is invisible to it; the defect surfaced only when
the served page was screenshotted after the cut.

**How to apply:**
- Any element dropped directly into `.board` must carry
  `grid-column: 1 / -1` (the way `.section-body` already does) or declare its
  own spans.
- After a layout change, **read the live page's screenshot** — not just the
  DOM dump — and compare the columns to the mockup's measurement (the mockup
  measured 216 / 633 / 340 at 1265px, which is what the shipped shell should
  produce).
- The same-class lesson already exists for the mockups' `[hidden]` rule: a CSS
  file copied or appended must carry the house's structural rules too.

Related: [[measuring-a-mockup-by-reading-the-png]],
[[desk-learning-project-surfaces-drafted-2026-11-09]],
[[the-release-is-the-owner-s-no-op]] (the fix cost a full extra cut).
