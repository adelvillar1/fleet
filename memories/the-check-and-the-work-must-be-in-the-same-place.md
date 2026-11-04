---
name: the-check-and-the-work-must-be-in-the-same-place
description: "feedback 2026-10-19 — a guard placed away from the work it guards passes while the work is wrong; two first drafts in one session had the check and the work in different places, and only the pinning test caught them"
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

Two bugs in my own first drafts, in one session, same shape: the check and the
work were in different places, so the check passed while the work was wrong.

**The guard inside the recursion did not guard the cost at the call site.** I
wanted a set sum — each node in the closure counted once — and put
`if seen[index] { return 0 }` at the top of the recursive walker, then added
`total += nodes[dep].cost` at the call site *before* recursing. The guard
prevented re-walking a node but not re-counting it, so a diamond's shared base
was still counted once per path. The diamond read 11 where the target was 9.
The test that pinned 9 failed, which is the only reason it was caught.

**The validation inside the loop let earlier items land.** The merge of a
returned sub-plan validated each child through `Graph::add`, which refuses a
cycle — but a cycle between two children is not visible until the second one is
added, and by then the first has already landed. The fix was to rehearse the
whole sub-plan against a scratch graph first and only touch the real one when
the rehearsal builds.

**Why the tests are what caught both:** I wrote the test pinning the target
arithmetic before claiming each increment, and both drafts passed their own
narrative — the code looked right, the reasoning was sound, the surrounding
tests passed. The pinning test is the only thing that disagreed.

**Third instance, 2026-10-20 (TUI).** A source-grep guard over a fixed module
list (`delegate/tests.rs`, copied via `include_str!` into explain and judge
tests) refused any mention of the delegation module, and the new TUI's control
writes tripped it. Loosening the guard so the TUI is a DRIVER was the wrong
move: that would wave through the whole module, beat included. The fix reads the
guard's actual rule — the beat must be unreachable from a turn path — and
allows exactly the one symbol that satisfies it (`delegate::write_control`,
which writes a file and spends no lane ask), leaving every other name refused.
Same lesson in a stricter form: make the check express the property rather than
the file.

**How to apply:** when a check guards work done somewhere else, ask whether it
actually guards it. A guard inside a callee does not guard a side effect at the
caller. Per-item validation inside a loop does not make the batch atomic — if
item 2 can fail because of item 1, validate the whole set before applying any
of it. And when implementing a new mechanism, write the test that pins the
target number first; it is the only reviewer that disagrees with a confident
first draft. Related:
[[a-regression-test-must-fail-when-the-bug-returns]] for the sibling discipline
of proving the test can fail, and
[[prove-the-target-is-unreachable-before-testing-expansion]] for checking the
fixture's baseline before trusting the expansion's result.
