---
name: prove-the-target-is-unreachable-before-testing-expansion
description: "feedback 2026-10-18 — when testing a retrieval-expansion mechanism, first prove the target is unreachable through the baseline, and compute the admission bar against the measured score distribution; three of my graph fixtures had neighbours that cleared the floor on their own, and a discount I invented made admission arithmetically impossible"
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

Three failures while building the recall graph's tests, all the same
shape:

1. **The neighbour was already reachable.** My first fixtures gave the
   target a description sharing a stopword and the shape tag with the
   instruction, so it scored 0.21 on its own — above the floor, in a pool,
   and `taken` was true. The edge never fired and the test failed with
   `via: None`. The fixture was not testing the graph at all.
2. **The admission bar I invented was unreachable.** `GRAPH_DISCOUNT`
   (0.5) against `MIN_RECALL_SCORE` (0.20) meant a source needed a total
   of 0.40 to admit anybody — and the measured sources score 0.30. I never
   computed the bar against the distribution before shipping it, so the
   feature was arithmetically impossible on the corpus it was built for.
3. **The guard blocked the case the feature existed for.** An early return
   on `primary.is_empty()` gated the retired-source path — and the
   retired-source path is precisely the case where the pool is empty.

**Why:** an expansion mechanism's whole value is reaching what the baseline
cannot, so a fixture where the baseline already reaches it proves nothing,
and a threshold chosen without the distribution is a coin flip. This is
[[a-regression-test-must-fail-when-the-bug-returns]] in a second register:
the test must fail when the mechanism is *absent*, not merely pass when it
is present.

**How to apply:** before writing the test, compute the target's baseline
score and assert it is below the floor; then compute the admission bar
against the measured source distribution. For the graph this meant
descriptions sharing no stem with the instruction (standing alone, 0.1875,
under the 0.20 floor) and an admission bar that is the source's own total
against the same floor everything else faces. See
[[fleet-recall-measured-state]] for the numbers.
