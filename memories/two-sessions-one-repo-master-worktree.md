---
name: two-sessions-one-repo-master-worktree
description: "2026-11-01 — the design-parity session shares the fleet worktree; my commits go through the fleet-master-wt second worktree on master"
type: project
date: 2026-11-05
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The design-parity session works in the SAME repo checkout
(/Users/alejandrodelvillar/Projects/fleet) and flips its HEAD to a
`design-parity` branch mid-session — twice my commits landed there
because I checked HEAD but did not gate on it, and once their K1-K6
commit swept my uncommitted cron.rs edit into their history. The
standing answer (2026-11-01): **all my commits go through
`/Users/alejandrodelvillar/Projects/fleet-master-wt`** (a git
worktree on master); their branch keeps its own history and their
mid-flight breakage (the Record::Memory.design pattern, a clippy
lint) gets folded with credit when it blocks my gate.

**The design session's branch is now MERGED AND PUSHED** (owner
reported it 2026-11-01; `origin/master` was 14 commits ahead of my
worktree, merge `ecbf280`, recap `6012b76`). So the rule changed: my
worktree is no longer a quiet island — **fetch and merge `origin/master`
before committing**, expect their hunks in `app.js`, `canvas.rs`,
`bridge.rs` and the harness, and re-run the whole gate on the merged
tree (their merge touched 20 files / +3,130 lines). My merge
`2858e11` was conflict-free.

Session state as of `f6e2d15`, **now PUSHED** (`6012b76..f6e2d15`,
origin/master in sync thanks to the merge `2858e11`): both 2026-11-01
plans complete (the seven open-calls rulings, the corpus-refresh
contract's five workstreams; recap `02f90e8`); the ledger reads **392
rows / 7 open**. The settings-ux plan is **partly built** — models open
on the MODEL, connect→discover→pin proven live, money gone from the
plan — see [[settings-ux-plan-2026-11-01]]; U1–U5/U9/U10 remain. Merge
gate on that tree: fmt/clippy/build 0, test 1,917/0, harness 615/56.

**HEAD is `b03520b`, pushed** (`e448b0c..b03520b`): the reload-every-
provider action. A push was REJECTED once (`behind 1`) because another
session landed `737dadc` between my fetch and my push — fetch, merge,
re-gate the merged tree (test 1,918/0, harness 618/56), then push. The
supervised face now points at the master worktree
([[fleet-gui-is-a-launchagent]]), so the one checkout that matters is
this one.

**Untracked-and-dirty noise to leave alone:** the shared home keeps
rewriting `memories/*.md` (`recall-count` bumps from ordinary use) —
22 files were dirty in this worktree; never commit them. `git rm` of
anything needs approval; the destructive gate blocks it.

**The rule, restated:** master is shared now. Fetch, merge
`origin/master`, re-gate the merged tree, push `my branch → master`
in a gated act. A shell-launched face is not delivery — relaunch it as
a tracked background task, and check which checkout the supervised
face (:3080, LaunchAgent `com.fleet.gui`) actually points at before
telling the owner a rebuild is live.
See [[gate-on-head-and-rebuild-tables-with-the-parser]].
