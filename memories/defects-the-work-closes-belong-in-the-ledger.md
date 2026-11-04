---
name: defects-the-work-closes-belong-in-the-ledger
description: "feedback 2026-10-20 — a plan's named defects and the ones verification found must be filed as KI rows, not only recorded in the plan doc; the owner asked whether I had and I had not"
type: feedback
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

I finished a 22-commit port that closed ten defects — six the plan named
as "must not be carried" and four that re-verification found — recorded
every one in the plan doc's landing notes, and filed **zero** KI rows.
The owner's question ("did you add to the known issues ledger?") was the
only reason it surfaced. `docs/known-issues.md` stopped at KI-256 while
KI-257..262 sat referenced in two plan documents and closed in the code.

**Why:** the plan doc is the record of what *this slice* did, and it is
read by whoever is doing the slice. The ledger is the record of what the
machine *is*, and it is read by every later triage. A defect that exists
only in a plan is invisible to the next audit — and a plan that references
rows nobody filed is worse than no reference, because it looks answered.

**How to apply:** when work closes a defect — especially one a plan named,
and doubly one verification found that the plan did not expect — write the
ledger row in the same commit or the one after it. Severity, the issue as
the owner would recognize it, the evidence (file and symbol, or the live
check that found it), status, and next action. Leave a row **open** when
the shape is not understood: I left KI-268 open with the reproduction to
attempt rather than loosening an assertion on a guess, which is the same
honesty [[a-green-gate-covers-what-was-built-not-what-was-left-out]]
demands in the other direction.

Mechanical: a literal `|` inside a cell splits it — write `\|` — and
`scripts/ki_ledger.py` is the validator to run afterwards, not the eye.
See [[a-validator-that-pads-hides-the-shift]].
