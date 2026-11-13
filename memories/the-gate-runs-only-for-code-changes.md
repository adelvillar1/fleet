---
name: the-gate-runs-only-for-code-changes
description: "owner ruling 2026-11-01 — the four-leg fleet gate runs before every push that carries CODE changes (Rust, JS, CSS, HTML, scripts, tests); a docs-only push skips it (no compile surface); hard rule 2 amended, the exit-code rule stands for every run that runs, and the owner asks \"does it make sense to re-run?\" when the change has no compile surface"
type: feedback
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 56
superseded-by:
valid-until:
---

2026-11-01: before pushing three repos I proposed re-running fleet's
four-leg gate (fmt/clippy/build/test), even though the last commit with
a recorded green gate (`be4488a`, 1,896/0) was followed only by my
docs commits (plan file, ledger row) and a parallel session's already-
gated O-numbered fixes. The owner's check: *"you have already run the
gate, does it make sense to re-run it?"* — and then the standing
amendment: *"amend whatever rules you have about the gate to only
apply for code changes."*

**Why:** the rule as written ("run it before every push") taxes
markdown with a full Rust gate — there is no compile surface in a `.md`,
and the owner had already called reflexive gates on non-code changes
"theatre". The evidence-based no came from an inventory of
`git diff be4488a..HEAD --stat`: docs and memories dominate; the code
deltas were someone else's protocol-gated increments, and re-gating
them is verifying another author's completed work.

**How to apply:** CLAUDE.md hard rule 2 now carries the scope (landed
`314a7f4`, pushed): the gate runs before every push **that carries code
changes** (Rust, JS, CSS, HTML, scripts, tests); a docs-only push skips
it. The **exit-code rule (KI-014) stands** for every run that does run
— scoping when the gate runs never scoping how it is read. The CI-
budget line's "every change" was amended to point at the same rule.
Before proposing any verification, inventory what changed
(`git diff <last-gated>..HEAD --stat`) and name what the step actually
requires — the sibling discipline is
[[verification-depth-matches-the-change]] (what a step needs, run only
that) and [[a-green-gate-can-ride-an-uncommitted-hunk]] (the gate
judges the working tree, so the *pushed* tree is what the scoped rule
protects). Related: [[gate-the-staged-tree-not-the-working-tree]].

**2026-11-02 — the exit-code rule earned its keep again.** Running
`cargo clippy --all-targets -- -D warnings 2>&1 | tail -60` reported
"exit code 0" while clippy had exited **101** with 41 errors; the pipe
ate the code and the first reading was a false green. Re-run without the
pipe (`> file 2>&1; echo $?`) to get the true code. The same turn showed
the gate's other half: a **toolchain move is a shared default**, so a
green gate is only ever green for the toolchain that produced it — see
[[rust-toolchain-moved-to-199-and-clippy-red]] and
[[a-toolchain-move-is-a-shared-default]].
