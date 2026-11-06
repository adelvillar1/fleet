---
name: contract-docs-are-the-third-gate
description: "feedback 2026-10-31 — code landed, tested, and pushed while FUNCTIONAL-SPECIFICATIONS/surfaces.md/README still described the pre-change app; a user asking \"update all relevant docs\" found the destinations count, a whole missing surface passage, and a stale section count to fix"
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

**2026-10-31 (design canvas + provider discovery both landed):** four
deliveries went through the gate — fmt, clippy, build, 1,866 tests, the
543-check harness, a browser render, a push — and the **contract docs
still described the app from before any of it**. The owner's instruction
*"Update all relevant docs"* then surfaced, in one pass:

- `FUNCTIONAL-SPECIFICATIONS.md` said **eleven destinations**; the rail
  had thirteen, and the design surface had no passage at all (its own
  user flow — board, brief, tweaks, pins, review, exports — was nowhere).
- Its settings passage still described the *pre-discovery* provider
  editor (field editor over the roster), because the previous session's
  proposed passage had been left pending rather than applied.
- `docs/architecture/surfaces.md` claimed settings spanned **eleven
  sections** where the rail has **eight** — a drift predating both
  sessions.
- `README.md` carried the twelve-destination count and no design bullet.
- `TECHNICAL-DOCUMENTATION.md`'s route list was missing all four design
  routes.

**Why it happens:** the feature doc (`docs/features/design.md`) is
written by the increment that builds the surface, so it stays current;
the *aggregate* documents are written by nobody, and they are the ones a
reader and the owner actually open first. A green gate says the code is
right, not that the product description is.

**How to apply:** a landing's doc pass is not one file — it is the
feature doc **plus** the aggregates the surface touches:
`FUNCTIONAL-SPECIFICATIONS.md` (the destinations count and a passage in
the owner's flow language), `docs/architecture/surfaces.md` (a section
in its own what/what-you-do/why/state shape, and the dated count),
`README.md` (the ships-today bullet), `TECHNICAL-DOCUMENTATION.md` (the
routes and the module entry). When a doc **proposes** a passage rather
than applying it, that proposal is debt: carry it to the next delivery
or apply it — [[provider-models-settings-plan-2026-10-31]]'s FS passage
sat pending across a whole session until this pass took it.

Two rules that came out of writing them: a **known-gaps row** is the
honest place for a partial proof (the canvas's review loop: pieces
pinned, never walked end to end by a model run), and **counts in prose
drift** — write "thirteen" only beside the rail it counts, and correct
the neighbouring stale count you find on the way rather than leaving it.

Related: [[a-plan-marked-met-can-hide-missing-behaviour]] (a plan is not
the product), [[docs-gallery-refresh-2026-10-30]] (the gallery is part
of a revamp's doc pass), [[design-canvas-native-plan-2026-10-31]].
