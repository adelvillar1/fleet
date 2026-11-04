---
name: a-parallel-session-can-reintroduce-your-fixed-lints
description: "feedback 2026-11-02 — a co-tenant session's merged commits reintroduced clippy lints I had already fixed in my own branch (the same six, in the same files); a shared tree's gate is only as clean as the last merge, so re-run the gate on the COMMITTED state before claiming green, and fold the co-tenant's lints with credit rather than re-fixing them as your own discovery"
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

Mid-plugin-host, `cargo clippy --all-targets -- -D warnings` went red with
**the exact six lints I had already fixed that morning** — the unused
`Arc`/`Mutex` import in `explain.rs`, the unneeded `mut` in `anomalies.rs`, the
duplicate `Row::Shape` arm in `calib.rs`, and three `boards.rs` sorts — because
the co-tenant session's commits (`5e83299`, `ca9a5cd`) had landed on top and
reintroduced them. The same tree had been green at `140c739`.

**Why:** with two sessions in one worktree, a clean gate is a property of the
tree at a moment, not of anyone's branch — a merge silently reverts another
author's fixes, and the next session to run the gate owns the red.

**How to apply:**
- Re-run the gate on the **committed state** before claiming green or landing;
  a green from an hour ago is a snapshot, not a status (the owner's *"make sure
  that the changes you are making are in line with the latest committed
  version"* is the ruling form of this).
- When the red is a co-tenant's, **fold the fix with credit** in the commit
  message ("two folds with credit — …") — the house's standing pattern, and it
  keeps the trunk green without a rebase war.
- Distinguish the co-tenant's lints from your own by the file: a lint in a file
  you have not touched this session is theirs.
- The 1.99.1 lint family (`sort_by_key`, `is_multiple_of`, `as_chunks`,
  `filter_next`, `collapsible_match`) is mechanical — fix and move on; the two
  `result_large_err` closures are the judgment calls, already allowed with a
  reason.

Related: [[a-toolchain-move-is-a-shared-default]] (the move that created the
lints), [[rust-1991-toolchain-move]] (the 1.99.1 move's full record),
[[a-parallel-session-co-edits-your-file]], [[a-green-gate-can-ride-an-uncommitted-hunk]].
