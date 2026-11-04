---
name: a-flag-that-conflates-a-tree-fact-with-a-machine-fact
description: "feedback 2026-10-19/2026-10-23 — a boolean that is set by two different conditions reports the wrong one under load; `Vocabulary::truncated` mixed a content bound with a wall-clock budget, so a busy machine reported the tree as not fitting its bounds. Growing a test suite raises the trip rate that surfaces it — and the 2026-10-23 return showed the split was half done, the test's fix was itself a clock claim, and the sound shape is an IMPLICATION plus a SEPARATION that name no measurement"
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

`Vocabulary::truncated` was set by three conditions in one `if`: two content
bounds (`MAX_SOURCES`, `MAX_TERMS`) and one wall-clock budget
(`DERIVE_BUDGET`, 5 seconds). All three set the same flag.

A content bound is a fact about the tree and is stable across runs and
machines. The budget is a fact about the machine's load at the time.
Conflating them meant a loaded machine reported "this tree does not fit
inside every bound" — which is false — and the bound test failed with that
message three runs in four while passing alone every time.

The fix split them: `truncated` carries only the content fact,
`budget_exhausted` carries the machine's, and the test asserts the
*reporting* (`budget_exhausted == elapsed > DERIVE_BUDGET`) rather than the
machine's speed. Six consecutive full-suite runs passed after.

**Why this surfaced now:** the test suite grew from 1065 to 1112 tests in one
session, and the extra parallel load pushed the tree walk past its 5-second
budget often enough to make a latent misreporting into a gate-blocking flake.
A load-sensitive assertion is a *detector* for conflated flags — when one
starts tripping, check whether it is measuring the machine rather than the
code before assuming it is a flake.

**2026-10-23 — the split was HALF done, and the test's own contract was
unsatisfiable.** The gate went red on this test (`elapsed` 5.10s against a 5s
budget) though `budget_exhausted` was false. Two independent causes, both mine
to fix:
(a) the loop broke on the clock without setting `truncated`, but a trailing
`if !truncated && paths.next().is_some() { truncated = true }` promoted the
cut-short walk into the tree fact anyway — so the split documented above was a
lie for exactly the case it existed to fix;
(b) the post-split assertion `budget_exhausted == (elapsed > DERIVE_BUDGET)`
is false **by construction**: the flag is read at the loop's last check, while
`elapsed` is read after `sink.finish()`, the index build, and struct assembly,
so a walk finished inside the budget can still total more than it. It only
passed while the margin was wide; a suite that grew to 1,372 tests closed it.

**The lesson to add: "assert the reporting, not the clock" was itself a clock
claim in disguise.** The sound shape is the implication plus the separation —
`budget_exhausted ⇒ elapsed > DERIVE_BUDGET` (the stop really was past the
budget) and `budget_exhausted ⇒ !truncated` (a clock stop never makes a tree
too large, the property the split exists for). Neither side names a
measurement the machine can move.

**How to apply:** when one boolean is set by several conditions, ask whether
they are the same *kind* of fact. A tree fact and a machine fact are not.
Split the flag, and have the test assert the reporting's internal consistency
rather than a wall-clock measurement — a timing claim in a parallel suite
measures the load, not the code. Then check the cleanup paths too: after
splitting a flag, grep for every later write to it (the trailing promotion
survived the split and undid it). Related:
[[fleet-memories-tests-need-their-own-home]] for the sibling lesson that a
flake passing in isolation can still have a nameable mechanism, and
[[fleet-gate-exit-code-trap]] for reading the gate's own exit code.
