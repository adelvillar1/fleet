---
name: a-mid-build-park-can-be-committed-by-the-cleanup
description: "feedback 2026-11-02 — a co-tenant session stopped mid-build had its parked work committed by the 1.99.1 cleanup commit (eabe031, on the wrong branch), so the trunk-landing commit carried another session's half-finished increment; verify a parked commit's contents before building on it, and land the whole thing in one push"
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

The owner stopped the co-tenant session mid-plugin-host to fix the 1.99.1
toolchain; the cleanup commit `eabe031` therefore **carried the plugin host's
parked work** (the eval action, `src/plugins/eval.rs`, the face host's
reads/routes, the plan and docs, the demo fixture) — and landed it on
`face-plugin-host`, not the branch the work had started on. The follow-up
session then finished the plan and landed the whole thing at `c8dde93`.

**Why:** a stopped session's uncommitted tree is still work — and the next
person to run the gate or commit in that worktree inherits it, on whatever
branch the checkout happens to sit. A "toolchain cleanup" commit is not
necessarily a cleanup.

**How to apply:**
- Before building on a commit you did not write, read its `--stat` and its
  message: a commit that carries files outside its stated scope is a parked
  increment, and its contents may be **half-finished or wrong** (the stopped
  session's `array_chunks` / 2-tuple-vs-4-tuple / edition-2024 let-chain were
  all wrong — see [[rust-1991-toolchain-move]]).
- The honest landing is the whole thing in one push: the parked work plus the
  finish, with the plan's criteria ticked against the tree as it now stands.
- When the owner says *"make sure that the changes you are making are in line
  with the latest committed version"*, that is this check — read the branch's
  tip, re-run the gate on the committed state, and only then continue.
- A stash of the co-tenant's in-flight churn (preserved, not popped) is the
  safe way to test whether a red predates your work: `git stash push` → re-run
  → restore. Do not pop a stash you did not create.

Related: [[a-parallel-session-co-edits-your-file]],
[[a-green-gate-can-ride-an-uncommitted-hunk]],
[[a-parallel-session-can-reintroduce-your-fixed-lints]],
[[creator-mode-plugin-host-exploration-2026-11-02]].
