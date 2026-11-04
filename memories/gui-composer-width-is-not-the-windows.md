---
name: gui-composer-width-is-not-the-windows
description: "2026-10-21 — the chat composer is narrower than the viewport whenever the rail shows (820 max, 650-800 in between), so fit decisions must be measured at the composer; the typing hint only fits at the 820 cap, which is why it is container-scoped"
type: project
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

The composer's width is `min(820px, calc(100% - 64px))` **inside the chat
column**, and the rail's appearance at one breakpoint makes the column
narrower than the window again: measured 2026-10-21, the composer is 820px
at a 900px window, 650px at 950, 700px at 1000, 800px at 1100.

**Why:** a viewport media query cannot answer "does this row fit". A
`@media (max-width: 900px)` rule hides or applies at widths where the
composer is at its widest, and misses 901-1099 where it is narrowest. The
typing hint demonstrated it: 177px of text fits at the 820 cap and at 860/900
windows, but collapses to a 7px ellipsis sliver at 950 — a string that
looks fine at the widths someone checks.

**How to apply:** put the fit rule on the container, not the window —
`.composer { container-type: inline-size; }` plus
`@container (max-width: 790px) { .composer-hint { display: none; } }`
(remember the query measures the content box, so the 790 threshold sits
inside the 820 cap). Field collapse stays viewport-scoped because the
owner's instruction was about screen size, but any *fit* decision —
showing a caption, a word, a column — is measured where the content lives.
Before claiming a layout fits, measure `scrollWidth` against `clientWidth`
at the widths the feature is for, including the dead zones between
breakpoints.

Related: [[a-media-query-adds-no-specificity]], [[verify-what-renders-not-what-commits]].
