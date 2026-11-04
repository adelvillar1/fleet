---
name: a-correct-order-makes-difference-negative
description: "feedback 2026-11-01 — a first fix on signed-difference code inverted the guard: days(earlier) − days(later) is NEGATIVE for a correct pair, so "non-negative means valid" returned None on every real call and reddened eight existing pins in one run; read the existing pins' direction before writing the sign check"
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

Closing KI-383 (a reversed `days_between` pair must be `None`, not an
`unsigned_abs` age), the first cut wrote `(difference >= 0).then_some(...)` —
which is exactly backwards: `days(earlier) - days(later)` is **negative** for a
correct pair (2026-10-01 − 2026-10-23 = −22). Every real call returned `None`,
ages computed as 0, and the whole lifecycle module went red at once: eight
tests failed, including four that had pinned the original function for months.
The correct form is `(difference <= 0).then_some(difference.unsigned_abs())`.

**Why:** a sign check on "which argument is later" is a claim about the
arithmetic's existing direction, and that direction is not guessable from the
parameter names alone — the original code took `unsigned_abs`, which hides the
sign on purpose and leaves the reader to infer it from the callers. The tell
was instant and total: one green module → eight failures the moment the guard
went in, not a subtle one-test flake.

**How to apply:** when adding a sign/ordering guard to existing arithmetic,
read the existing pins (or the callers) FIRST to learn which sign the correct
order produces — they are the only witnesses to the convention. Then expect a
wrong-direction guard to fail loudly and broadly, and treat that breadth as
the diagnosis rather than patching tests one by one. Related:
[[a-load-sensitive-test-is-a-defect-in-the-code]],
[[a-green-gate-can-ride-an-uncommitted-hunk]].
