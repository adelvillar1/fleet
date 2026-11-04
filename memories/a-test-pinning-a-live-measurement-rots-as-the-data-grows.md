---
name: a-test-pinning-a-live-measurement-rots-as-the-data-grows
description: "feedback 2026-10-26 — a test asserting a measurement of live data reddens when the data grows; the history-channel test's live-corpus fraction decayed under this session's own memory writes with no defect anywhere"
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

`the_history_channel_separates_the_corpus_that_saturated_it` asserted
"more than half the corpus sits above the old ceiling of 8" against the
LIVE memory corpus — a measurement true when R1 was designed. This
session's own memory writes grew the corpus from ~61 to 66 facts, and
new facts carry low recall counts, so the fraction decayed to 28 of 66
and the test failed with no defect in the code. The corpus grows
through ordinary use — every session writes memories — so a
corpus-pinned measurement is a booby trap the setting session can trip
itself.

The repair separates what the test held together: the property the
measurement stood for belongs to the CHANNEL (it discriminates a
motivating shape), so it is pinned synthetically — a count spread with
more than half above the old ceiling must separate, no two counts
sharing a term — while the live read stays only for the durable
invariants (strict monotonicity below the ceiling, saturation at it),
which hold at any corpus size. The stale number moved into a comment
("28 of 66 on 2026-10-26").

**Why:** a snapshot presented as an assertion fails when the world
changes, and the failure looks like a regression — this one cost a
diagnosis cycle mid-gate on a session that was fixing something else
entirely. The complementary trap is
[[a-fixture-corpus-flatters-a-retriever]]: a threshold must be measured
on realistic live data, while an invariant must not depend on the
data's current size. Both rules are the same rule — name what your
assertion is about.

**How to apply:** when a test's assertion is a count or fraction over
live data, ask whether it names an invariant or a snapshot. Snapshots
become comments carrying the number and the date; the assertion moves
to a synthetic shape that cannot rot. If the live read stays, assert
only what holds at any size. Related:
[[fleet-memory-corpus-dirties-the-tree]] (the corpus grows through
ordinary use), [[a-flag-that-conflates-a-tree-fact-with-a-machine-fact]].
