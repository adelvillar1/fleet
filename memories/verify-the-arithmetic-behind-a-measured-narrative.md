---
name: verify-the-arithmetic-behind-a-measured-narrative
description: "feedback 2026-10-18 — a measured root cause can be arithmetically wrong; recompute the specific claim before building a plan on it, and say plainly when a fix moves the needle less than the narrative implied"
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

While writing the memory/cache fix plan I had a clean, measured
narrative: the history channel is saturated (67% of the corpus at or
above `RECALL_CEILING` 8, median recall-count 56), therefore a
never-recalled fact carries a 0.25 structural handicap, therefore "the
word channels, measured at 0.04–0.15 in practice, cannot overcome it."
The first three clauses were true. The fourth was false — I never
computed it. Full keyword coverage plus a 0.5 trajectory match scores
0.481 against a 103-recall wordless fact's 0.400. Direction still wins
comfortably.

The same draft then claimed R1 (a log curve over a wider ceiling) plus R2
(a trajectory tie-break) were "the recall win." Re-running the
twelve-instruction set said 7/12 → 8/12. One item. The dominant cause of
the measured failures was the phrase-vocabulary gap — recorded `phrases`
carry the resolution's vocabulary rather than the task's nouns, so the
trajectory channel scores 0.000 against a perfectly relevant instruction —
which no reweighting reaches and which a writer-side fix does.

**Why:** a measurement gives a narrative credibility it has not earned.
The saturated channel was real, so the handicap claim *sounded* derived
from it, and a plan built on it would have shipped a reweighting sold as
the fix. This is the second time in this project — see
[[blend1-is-the-latency-triad]], where "the real gap is layout geometry"
was asserted three times and was already measured. In both cases the
measurement was sound and the inference drawn from it was not.

**How to apply:** when a measurement produces a root cause, recompute the
one number the root cause rests on before writing it down — here, "can
the word channels overcome the standing term?" is one line of
arithmetic. And when a proposed fix is measured, report what it actually
moved rather than what the narrative promised; a fix worth 1 of 12 is a
correctness fix and should be labelled one, not sold as the win. Related:
[[your-test-harness-caps-are-not-the-models]] (the same family — a
confident conclusion outrunning its evidence),
[[fleet-recall-measured-state]] (the corrected numbers).
