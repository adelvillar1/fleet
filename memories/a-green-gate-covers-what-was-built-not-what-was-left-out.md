---
name: a-green-gate-covers-what-was-built-not-what-was-left-out
description: "feedback 2026-10-19 — naming a gap honestly is the right move in a recap and the wrong move in a delivery; the completion verifier refused a green-gate objective because two increments were reported as \"deferred and named\" rather than built"
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

I delivered six of a plan's eight increments, reported the remaining two as
"deferred and named" with a reason and a target, and closed with a green gate:
`fmt=0`, `clippy=0`, `build --release=0`, `test=0`, 1244 passed, 0 failed. The
completion verifier refused the objective. Its reason was exact: **a green gate
covers only what was built, not what was left out.**

Both increments landed in the following pass and the gate went to 1252. The
work was never in doubt; the *claim* was.

**The distinction that matters is the artifact's contract.** A recap is the
post-work journal — naming what did not land, with a reason and a target, is
exactly what it is for, and it is how the next session knows where to start. A
delivery is a completed objective, and the same sentence in the same words is a
shortfall. I had written the honest sentence into both and let the green gate
stand in for the difference.

**How to apply:** when an objective has numbered parts and some are unbuilt,
the completion state is *incomplete*, whatever the gate says and however
precisely the gap is named. "Deferred with a reason and a target" is a
legitimate plan status and an illegitimate delivery status. If a gap is
genuinely out of scope, say so *before* delivering and let the owner strike it
from the objective — do not deliver short and annotate. And read the gate as
evidence about what it ran, never as a proxy for the objective: it can only
report on the tests that exist, and a test for an unbuilt increment does not
exist. Related:
[[a-verification-with-no-output-is-not-a-pass]] for the sibling lesson that an
empty or partial check result is a signal to re-run, not a pass.

**The same lesson one layer up (2026-10-23): a green gate is a fact about the
TREE it ran on, and the recap has to say which tree.** A plan's recap recorded
"Gate: 1498 passed, 1 ignored" in good faith — the gate ran on the working
tree and the number is real — while the commit that tree was staged into
(`70dc3bf`) carried a subset of it and did not compile. Nothing in the recap
was false; it just answered a question nobody asked, because the tree was
dirty when the gate ran and the staged set was narrower than the tree. So the
delivery claim is "the gate passed on the tree this commit contains", and the
way to earn it is to compile the staged tree ([[shared-file-commits-can-sweep-another-sessions-work]])
or to confirm `git diff HEAD` touches nothing that compiles. A recap claiming a
gate number while the tree is dirty is a claim about the worktree.

**And the trap fires inside a plan too (2026-10-23, twice in one day).** Both
work-surface commits named their source paths and left test fixtures behind —
once the `AgentOptions` literals, once `SessionSummary`'s — so the pushed tree did
not compile while the worktree's gate was green. The check that catches it in two
seconds: `git diff HEAD -- src tests` must print nothing before you claim a green
tree, because a commit that names paths is only as wide as the list you typed.

**The completion verifier reads the plan's own wording, not the recap's.** Four
consecutive refusals on one plan came from the same gap: a criterion written as "a
face test asserts X" is satisfied only by a committed case asserting X — a live
Chromium pass, a manual measurement, or the assistant's own paragraph saying so
never counts. When a criterion names an artifact (`A face test…`, `A unit test…`),
build that artifact or rewrite the criterion and say why; and when the build
refines the plan (two state words instead of one, a pencil instead of a press),
amend the criterion to match *before* ticking it, or the verifier reads a
deviation where you meant a refinement. Same-day corollary: tick the `- [ ]` boxes
against evidence rather than leaving a `status: completed` plan with unticked
criteria.
