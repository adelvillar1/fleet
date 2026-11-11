---
name: route-judges-the-work-implemented
description: "project 2026-11-06 — the route-judges-the-work plan 9/10 on origin/master (eb57d8f): the plan vocabulary's `judged` status and `execution:` frontmatter verdict, sys1's `plan_judge` and `goal_work` tasks, `judge_plan` (the spawn + the frontmatter write + the draft→judged flip), the work_brief resolver (the goal + the drafted plans + the named or recency plan + the verdict + the criteria), the route's judgment input is the brief, the plan's verdict outranks the fresh head, the legacy on-the-spot path, and C7'…"
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

The route-judges-the-work plan (`docs/plans/2026-11-06-route-judges-the-work.md`)
is **9/10 on origin/master at `eb57d8f`** — the mechanism the owner's rulings
produced after the live goal run never delegated (see
[[why-the-goal-run-never-delegated]] for the diagnosis and the rulings).

**What landed (commits `cf9a90e` + `6a75b60` + `6cfd0b8` on
docs-install-tui, carried through the merge):**

- **The plan vocabulary** (`src/plan_graph.rs`): `PlanStatus::Judged`
  (draft → judged → active → completed) and `ExecutionVerdict`
  (Single/Mixture/Swarm) parsed from the frontmatter's
  `execution: single|mixture|swarm` line. `read_meta` carries it.
- **sys1's two new tasks** (`services/sys1/src/sys1/tasks.py`):
  `plan_judge` (one execution head over the plan's tasks and criteria) and
  `goal_work` (the implied work shape for a non-plan goal: one thread /
  independent parts / unclear). Both registered and verified.
- **`judge_plan(plan)`** (`src/route.rs`): spawns the sys1 lane, calls
  `sys1_classify({task: "plan_judge", text: <slug + criteria>})`, parses
  the execution head, writes `execution: <label>` into the frontmatter
  after `status:`, and flips a draft to `judged`. The legacy path: a plan
  with no verdict is judged on the spot in `prepare_walk` before the
  route's own judgment runs (C3).
- **`judge_goal_work(goal, task)`**: calls `goal_work`, maps the label to
  the execution verdict. The non-plan half of the ruling (C6).
- **The work brief resolver** (`src/work_brief.rs`):
  `resolve(home, plan_root, session, task) → WorkBrief` — the session's
  goal, the plans the session drafted (its log's write-class calls with
  docs/plans/ paths, most recent last), the named plan (the slug in the
  ask) or the most recent drafted, the plan's execution verdict, and the
  checkbox criteria. `render()` composes the brief the judge reads.
- **The route wiring** (`prepare_walk`): the leader's judgment input is
  the brief's render, not the bare task; the plan's own verdict overrides
  the fresh execution head at confidence 1.0 (the reason names the plan as
  the source); the child's brief passes through verbatim (Walk::Judged
  skips the resolver).
- **C7's live proof**: the proof face drove
  `POST /api/run?session=20261106T033200Z-14f5a714` with the task
  `implement the open-ledger-closeout plan`. The resolver found the plan
  the session drafted, the legacy judgment fired, and the verdict —
  `execution: swarm` — landed in the plan's frontmatter. The named
  session's newest routed record reads
  `{"execution": {"label": "swarm", "confidence": 1.0}}` — the plan's
  verdict riding the route, not the fresh `single` at 0.85.

**Pins:** three resolver pins in `src/work_brief.rs`'s tests (the verdict
riding the brief, the recency default, the bare render). The gate 1921/0
lib, clippy 0, fmt 0.

**Open: C12** — the four decision rows (KI-406/412/404/414) are the
owner's by the plan's own text. The open-ledger plan's status on master
is now `completed` (the burn-down flipped it in the merge resolution)
and carries `execution: swarm` from the C7 live proof — C12 remains the
owner's call even though the plan's status says completed.

**How to apply.** The defect was the leader's judgment INPUT, not the
nudge's threshold: any task phrased as a pointer gets a `single` verdict
that is defensible about the sentence and wrong about the work. The
child's brief already did the right thing. The `plan_root` parameter
exists because a process cwd is a global two parallel tests race — inject
it in tests. Same family as
[[delegation-during-goal-execution-deep-dive]] and
[[why-the-goal-run-never-delegated]].
