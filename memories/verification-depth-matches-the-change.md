---
name: verification-depth-matches-the-change
description: "feedback 2026-10-29 — the owner cancelled a full 1,700-test cargo run started as over-thorough pre-restart verification and asked \"why are you stuck on this?\"; match the gate's depth to the change, and long suites go to the background or don't run"
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

2026-10-29: after a JS-only face change (icons), I ran fmt + clippy +
release build (fast, all green) and then started the FULL `cargo test`
suite (~1,700 tests, minutes) in the foreground as extra verification
before restarting the gui. The owner cancelled it and asked "why are you
stuck on this?".

**Why:** the gate-whole rule exists for commits and pushes, not for every
state change. A face restart's hard requirement is a clean release build;
the JS face's covering test is the harness (already green). A slow
foreground suite that the step doesn't need reads to the owner as being
stuck — and it was the second time a long gate ran foreground
([[react-to-cancelled-foreground-calls]] already recorded "long
gates/test suites belong in the background from the start").

**How to apply:** before running a verification, name what the step
actually requires and run only that; anything beyond it goes to the
background or is offered, not assumed. A change's own covering tests
(the face harness for GUI work) satisfy the increment; the full gate
waits for the commit it guards. When a cancellation lands, say what was
skipped and what that leaves unverified, then continue the step.

**2026-11-01 — the rule is the rule now:** the owner turned the
practice into CLAUDE.md hard rule 2 (`314a7f4`): the four-leg gate
runs before every push **that carries code changes**; a docs-only push
skips it. So the check before any push is an inventory —
`git diff <last-gated-commit>..HEAD --stat` — and the exit-code rule
(KI-014) still binds every gate that does run. See
[[the-gate-runs-only-for-code-changes]].
