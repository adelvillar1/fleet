---
name: fleet-spillover-floor-arithmetic-hole
description: "project 2026-10-18 — SPILLOVER_FLOOR's documented justification omits the history term, so a heavily-recalled fact with no direction can clear the floor that exists to detect exactly that; unfixed, wants a KI row"
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

`SPILLOVER_FLOOR` (0.50) is documented as "the point at which the
shape's own answer carries DIRECTION," justified by the claim that "the
most a fact with no direction evidence can score is 0.45." That
arithmetic assumes the history term is zero, and nothing in the
justification says so. The real no-direction maximum is

    0.15 (vector) + 0.15 (keyword) + 0.25×h (history) + 0.10 (freshness)
    + 0.05 (provenance) = 0.45 + 0.25×h

so a fact with full topical coverage, no recorded phrases, and enough
recalls to saturate the history channel reaches **0.70** at h = 1.0 —
twenty points above the floor. Under the log curve R1 shipped (ceiling
128), this corpus's 103-recall facts read h = 0.955, so 0.689. Either
way a directionless fact clears the floor comfortably, which is
precisely the case the floor claims to exclude.

R1 made the hole smaller, not closed. Under the old linear ceiling of 8,
two-thirds of the corpus sat at the h = 1.0 extreme, so most facts were
at the 0.70 maximum; the log curve spreads them across the range, but
the top of the corpus still clears.

**Why:** the floor's justification is a hand-checked sum that omitted a
term, and it survived because the omitted term's maximum was never
written down anywhere. The floor's *behaviour* is still defensible — it
is the stated reasoning that is wrong — so nothing reddened and no test
covers the justification.

**How to apply:** restate the floor's arithmetic with the history term
included (the true no-direction maximum is 0.70, so either the floor
moves or the justification is rewritten to "cleared by topical coverage
plus magnitude"), and file a KI row — this was named in the 2026-10-18
recap's open questions but never filed. Related:
[[fleet-recall-measured-state]] (the measured distribution the hole sits
on),
[[verify-the-arithmetic-behind-a-measured-narrative]] (the same shape —
a confident sum that skipped a term).
