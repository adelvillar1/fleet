---
name: a-repaint-that-reassigns-under-the-handlers-orphans-the-clicks
description: "feedback 2026-11-01 — the model chips' toggle worked in isolation but every click after a repaint mutated a DEAD array, because the paint reassigned the state AFTER building handlers that closed over the old one; mutate the live array in place and filter/reassign only before handlers exist"
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

The fan-out chips (`paintDesignModels`) did:
`var picks = face.designModelPicks;` → build chips whose click
handlers `picks.push/splice` → then at the end of the SAME paint
`face.designModelPicks = picks.filter(available)` plus a default
fallback. The reassignment makes `face.designModelPicks` a NEW array
no handler holds, while every handler closes over the array from the
paint that built it — which is dropped on the next paint. Net effect:
clicks after any repaint (i.e. every click, since the click itself
triggers a repaint) mutate an orphan; the availability filter ran on
the stale array. The harness caught it as "several models press
together — 1 pressed" and "K pins" = the same pin twice. The fix:
filter/reassign at the TOP of the paint (before `var picks = ...`),
handlers mutate that one live array in place, and nothing reassigns
it until the next paint rebuilds the handlers over the new value.

**Why:** a closure captures the binding, not the property —
`var picks = face.designModelPicks` copies the REFERENCE once, and any
later `face.x = somethingElse` does not move the handlers. This is the
classic render-loop bug imported into a hand-rolled repaint: state
must be a single mutable cell the handlers share, or every handler
must read `face.designModelPicks` at click time (the alternative fix).

**How to apply:** in any repaint-style painter that wires closures
over state, either (a) mutate the state object in place and never
reassign it inside the paint, or (b) have handlers re-read the live
field (`face.designModelPicks`) on every event. If a paint "sanitizes"
its input (filter/dedupe/default), sanitize BEFORE capturing, and pin
the multi-step flow (press A, press B, assert both) — a single-toggle
pin passes against the orphan because the FIRST click's array is still
the live one. Sibling forms: [[a-rewrite-that-leaves-the-old-definition-hoists-it-last]]
(JS hoisting keeps the last definition), [[a-fixture-that-never-rules-is-invisible]].
