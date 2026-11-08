# Session recap — 2026-11-07: the plan-worktree machinery

**Summary.** The owner's ruling — plan-based worktrees with a hygiene
cycle, the machinery scripted so resources are used wisely — landed
whole: `docs/plans/2026-11-07-plan-worktree-machinery.md`, **9 of 9
criteria ticked**, on its own worktree (`plan-worktree-machinery`,
forked from master with the plan file cherry-picked in — the convention
the script now enforces, exercised by the build that made it).

## What was built

- **`scripts/plan-worktree.sh`** — the four verbs. `add <slug>
  [--base] [--floor]` creates the lane with the disk guard first (the
  floor 20 GiB, refused with the numbers named), prints the three
  rules — absolute paths, the worktree's own gate, one deployment at
  a time — at the moment they matter. `remove <slug> [--force]
  [--push]` refuses uncommitted work and unmerged branches without
  `--force` (nothing is lost), prunes the lane, reports the bytes
  freed. `status` lists every lane with merge state, target size, and
  commit age, and names non-plan worktrees as not its business. `sweep
  [--dry-run] [--live]` is the cycle: prune `target/` on merged stale
  lanes, remove only merged-AND-21-days-stale ones, hold unmerged
  lanes by name.
- **The hygiene cycle** — `daily-maintenance.sh` gained the sweep leg
  (`--dry-run`, logged like the other legs): the discipline runs
  whether or not anyone remembers. The naming convention is the closed
  list (`fleet-<slug>` beside the repo) — the repo-hygiene discipline,
  unknowns named and never touched.
- **The smoke fixture** (`tests/scripts/plan-worktree.sh`) — its own
  bare origin and main checkout in a temp dir: the add's rules and
  guard, the remove's two refusals, the status report, the sweep's
  dry-run naming and live pruning, the unmerged hold, the maintenance
  leg — all green.

## The verification

- **The smoke fixture**: all checks green on its own scratch git (no
  network, no machine state — KI-426's lesson priced in).
- **The four-command gate**: fmt 0, clippy 0, release build 0,
  `cargo test` **2,143 passed / 0 failed** — the change carries no
  Rust, run whole anyway per the plan.
- **The live proof** (the real repo): `add pw-proof` — the lane, the
  guard ("36 GiB free against the 20 GiB floor"), the three rules.
  `remove` with the lane's own commit — **refused** with the rule
  named ("branch pw-proof is not merged into master — nothing is
  lost"), exit 1. `remove --force` — the lane gone, the bytes
  reported. `status` showed the real lanes (owner-inbox's 8.8 GiB
  target the machine's actual cost; the main checkout named as not
  this script's business). The sweep dry-run: nothing stale — the
  lanes are all live.

## Found on the way

- **The double-prefix bug the fixture caught**: the closed list
  emitted the directory name (`fleet-three`) as the lane's name, and
  the live removal deleted the branch `fleet-three` — not found —
  while the worktree had already gone: the sweep reported "nothing
  needed doing" over a removal it had half-performed. The slug is
  emitted once, stripped of its prefix; every consumer names the
  branch correctly.
- **`var waiting` in a function scope**: the inbox arm's re-declared
  `waiting` shadowed the sessions arm's — the harness's N3 test
  caught the sessions badge going empty. Renamed; the harness is the
  guard for the whole app, not just the feature under test.
- **Git's date parsing**: `22 days ago` is not a git date format
  (neither `--date` nor the env accepted it in these contexts) —
  the fixture computes an absolute date; the smoke caught the
  invalid-format error the first run.

## Where it stands, honestly

- The machinery is on **`plan-worktree-machinery`** (origin) —
  `master` untouched, the merge the owner's call.
- `shellcheck` is not on this machine; C8 amended to name the
  absence and pin `bash -n` plus the smoke fixture as the substitute.
- The real lanes (`owner-inbox` at 8.8 GiB target, `remote-access`)
  stay — merged and live; the cycle will prune them when they go
  stale, `--live` the owner's explicit act.
