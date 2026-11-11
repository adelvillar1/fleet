---
name: a-budget-per-store-beat-clean-up-after
description: "owner ruling 2026-11-09 — fleet must be a RESPONSIBLE citizen, not a good one; every store gets a DECLARED budget measured and reported, never allowed to run to near-exhaustion and then cleaned; fleet must also be AWARE at all times of what it holds (disk, memory, its own process tree)"
type: feedback
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

Owner, 2026-11-09: *"I want fleet to run as lean as possible, we cannot have this
pattern of using resources until near exhaustion to clean up. it needs to be a
responsible citizen, not just a good citizen."* Then, going further: *"fleet
needs to be aware of what it is using at all times, disk, system memory, even
processes that has spawned, and then determine if it's being responsible about
them, whether that's a sys1 decision or some deterministic approach, that's a
mechanics choice."*

**Why:** everything I had built that day was a *rescue* — the sweep cleaned a
lane once it went cold, the release preflight refused below a disk floor, the
maintenance capped a lane by size. All act **after** the fact, and all let a
store grow until a threshold was crossed; that is the shape that took the drive
to 1.6 GiB free. What was missing was the **declaration**: no store had a number
it was supposed to stay under, so "over" could only ever be discovered by
measuring too late (KI-505). And declaring budgets is not the same as being
*aware*: the owner wants live knowledge of disk, memory and the processes fleet
spawned, not a nightly audit.

**How to apply:**
- Every store gets a budget in ONE place, measured and reported against it:
  `scripts/footprint.sh` (target 20 GiB, one lane 20, all lanes 24, `~/.fleet`
  12; env-overridable), exits non-zero when a store is over, `--record` writes
  `~/.fleet/footprint.json`.
- The audit rides the routine: `daily-maintenance.sh` leg 9 runs it and logs the
  verdict; `fleet doctor` reports `caches ok 17.0 GiB of 20 GiB (measured 0m ago)`
  from the **sidecar** — a doctor must not walk 15 GiB on every call, and a
  budget is only real if it is reported.
- Live awareness has its own verb: **`fleet footprint`** reads `ps -o
  pid=,ppid=,rss= -ax`, walks the tree from its own pid (run, lanes, child
  shells), sums RSS and counts processes against declared ceilings (2 GiB /
  80 processes), names the heaviest pid when one holds most of the weight, and
  exits 1 on a breach. Output measured 2026-11-09: `memory ok 13 MiB of 2048 MiB
  (10 of 13 in pid 84087)`, `processes ok 2 of 80`.
- **Mechanics split, the owner's explicit licence to choose:** DETERMINISTIC for
  the bounds (a store over budget, a tree over its memory/process ceiling is
  arithmetic — a model adds nothing), **sys1 judgment for the ambiguous** ("is
  this run holding 3 GiB because it is working or stuck?"), made where the run
  already asks for judgments, never inside the measurement module.
- `src/footprint.rs` is pure functions over facts (parse_ps, tree_of,
  memory_kib, evaluate_tree/render) so the grader is hermetic and pinned (3
  tests); the `Footprint` CLI variant carries no options.
- The writers keep to the bound (the release preflight's floor and lane cap are
  the declared numbers; the gate lane is released the moment a cut publishes); a
  sweep is the fallback, never the plan.
- Open next step, stated not done: a beat that runs `fleet footprint` during a
  run — deterministic breach stops the run, ambiguous reading goes to sys1 with
  the numbers attached. The owner then ruled the fuller form — measure, admit,
  work, re-measure, CURTAIL — and licensed the sys1 call when it prevents a
  crash: see [[the-resource-contract-admit-work-settle-curtail]] (plan drafted).
  Also: the 2 GiB / 80-process ceilings are my numbers and should be the
  owner's (per-machine); `~/.fleet/venvs` (3.4 GiB) has no write-time cap;
  `target/debug`'s 15 GiB allowance is generous.

Companion rule from the same stretch (in [[build-lanes-in-tmp-eat-the-drive]]):
keep only what the NEXT build or run will reuse — and that line is drawn by
success and failure alike.

**Extended 2026-11-10 to the session/plan level:** activating the
grounded-selection plan, the owner ruled *"cleanup after each independent
section is done, we cannot keep killing the available hard drive space, be a
very responsible citizen, no leftovers and no gaps from the plan."* Now
ENFORCEABLE as criterion **C20** in `docs/plans/2026-11-10-grounded-selection.md`
(committed 06d15a7): every phase's landing must record, in the plan's own
**Cleanup ledger**, the act with df before/after — worktree pruned, build
lanes released (`cache-sweep.sh --gate-lane`), sweep run, nothing left. A
dirty phase shows as a missing ledger entry, a stalled one as an open box.
First entry (activation): 124 GiB free before and after, stale
`/tmp/fleet-cut-0622` registration pruned, sweep clean — net zero.

Related: [[checkpoints-unbounded-72g-disk]],
[[house-evidence-discipline]].
