---
name: fleet-tree-broken-half-edit-2026-11-05
description: "project 2026-11-05 — the fleet working tree carries a broken uncommitted edit (calib.rs non-exhaustive match) plus a usage-plan status flip, both stamped 12:32:33, likely a dead co-tenant session; keep-or-revert is the owner's unanswered call"
type: project
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

At the 2026-11-05 warmup the working tree was dirty and **does not compile**:
`src/calib.rs` had the `Row::Shape(_)` arm removed from the `op()` match while
the `Shape` variant still exists (calib.rs:652) and `ts()` still handles it —
a non-exhaustive match, so `cargo build` fails as the tree stands. Alongside,
the usage plan's frontmatter was flipped `draft → active` (its 10 criteria
were ticked and completed at 140c739), consistent with a co-tenant session
reopening it for a Shape-row rework and dying mid-edit — both files stamped
12:32:33 2026-11-05, no cargo/fleet processes running. Keep-or-revert was put
to the owner twice with no answer; on the owner's "implement goal as designed"
the session **stashed both files under a NAMED stash** (stash@{0}: "half-edit
found 12:32:33 … stashed to unblock the goal-task-pane build") rather than
discarding them — nothing lost, the build unblocked, and the stash message
carries the finding. The memories/*.md recall-count churn left in the tree is
the runtime's own edits, not touched.

Branch state at the same warmup: `face-plugin-host` 53 commits ahead of local
`master`, local `master` 8 ahead of `origin/master` — trunk sequencing is the
owner's call. The unattended-goals plan file showed 0 criteria ticked while
that day's commits (a4c785a/6cd5ba3/9d6bc4a) already landed the uncapped loop,
the goal tool, and the supervision marking — its checkboxes are behind the
tree ([[a-plan-never-ticked-is-not-a-status]] again).

**Resolution (2026-11-05, same session):** the gap closed on its own —
local `master` fast-forwarded to `de5d643` (the extraction close-out,
pushed by the parallel session) and `origin/master` caught up, so the
branch went from 53 ahead to ONE ahead. The stash was never reverted or
popped; it still sits at `stash@{0}` with its finding message, and
keep-or-revert remains the owner's unanswered call. **How to apply:** a
branch-ahead count from a warmup is a snapshot, not a debt — re-read it
before acting, because the co-tenant's pushes move it.
