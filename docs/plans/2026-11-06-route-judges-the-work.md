---
status: draft
created: 2026-11-06
updated: 2026-11-06
slug: route-judges-the-work
---

# Plan: the plan carries its execution verdict

## Context

The leader's route judgment is handed the literal task text: the bridge
passes `&task` to `prepare_with_pin` (`src/gui/bridge.rs:11406`), and the
router sends it as the sys1 `text` (`src/route.rs:2552`). A delegated child
is judged over its brief (`src/subagents.rs:1970`) — the actual work. So
for a plan-implementation task the work is invisible, and the execution
head judged `single` (the live run's third routing, 0.8371), switching off
the only delegation nudge (`src/agent.rs:1979-1994`). The owner's ruling:
a plan should carry its own execution verdict — a `judged` status, set at
draft time over the plan's tasks and acceptance criteria — so when the
session picks it up the verdict is already there and the only decision the
model makes is how the delegation happens. The non-plan case (the goal's
implied work) is in scope.

## Approach

1. The plan lifecycle gains `judged` (draft → judged → active →
   completed). The frontmatter carries `execution: single|mixture|swarm`.
2. At the draft, a new sys1 task `plan_judge` (GLiDE, the route cascade's
   first provider) judges the plan's tasks + acceptance criteria; the
   verdict is written to the frontmatter and the status flips to `judged`.
   The `plan` tool's `judge` action calls it.
3. The run-time resolver (`src/work_brief.rs`) reads the goal + the task,
   resolves the plan (the named / the most recent drafted), and reads the
   plan's execution mode. A plan with no verdict — a legacy plan — is
   judged on the spot, before execution begins, and the verdict is stored.
4. The route judgment's execution head for a plan goal is the plan's
   verdict; the workload head still judges.
5. The delegation nudge fires from the plan's verdict; the model decides
   how to delegate.
6. For a non-plan goal, a new sys1 task `goal_work` (GLiDE) expands the
   goal to its implied work; the brief carries it.

**Decision-model check.** Two sys1 jobs: `plan_judge` (one per plan, heads
= the execution label + confidence, escalation = the cascade's own) and
`goal_work` (one per non-plan goal, heads = the implied work items +
confidence). Both pay accuracy — the execution head sees the real work.
The plan resolution is a deterministic read, not a judgment.

High-level steps:

1. The plan lifecycle + `plan_judge` + the `plan` tool's `judge` action.
2. `src/work_brief.rs`: the resolver.
3. `src/route.rs`: the brief; the execution head from the plan.
4. The non-plan expansion.
5. The live proof: on a scratch home with the lane up, the open-ledger plan
   is judged to its execution verdict, then a goal "implement plan as
   designed" routes with the delegation nudge firing.

## Use cases

- [ ] A: The owner sets a goal that refers to a plan the session drafted,
      and the route judgment carries that plan's execution verdict.
- [ ] B: The owner names a plan (full or partial) that the session did not
      draft, and the judgment carries that plan's verdict.
- [ ] C: The owner sets a goal that is not a plan, and the judgment carries
      the work the goal implies.

## Acceptance criteria

These are the contract. Each one is a verifiable assertion. The work is "done" when every box is
checked. **Order is identity** — never reorder after gating.

- [ ] C0 — the plan lifecycle gains `judged`, and a plan's frontmatter
      carries `execution: single|mixture|swarm`.
- [ ] C1 — a plan is judged by sys1 over its tasks + acceptance criteria;
      the verdict is stored and the status is `judged`.
- [ ] C2 — the resolver reads the plan's execution mode for a plan goal
      (the named / the recency default).
- [ ] C3 — a legacy plan with no verdict is judged before execution begins,
      and the verdict is stored.
- [ ] C4 — the route judgment's execution head for a plan goal is the plan's
      verdict, not a fresh judgment.
- [ ] C5 — the delegation nudge fires from the plan's verdict.
- [ ] C6 — a non-plan goal's implied work rides the brief.
- [ ] C7 — the live proof: the open-ledger plan's judgment routes its
      implementation with the delegation nudge.
- [ ] C8 — the specs describe the plan judgment, the goal expansion, and
      the route input.
- [ ] C9 — the resolver, the judgment input, the plan judgment, the legacy
      path, and the non-plan expansion each carry a pin in
      `tests/integration.rs`.

## Files to be touched

**Backend / API:**
- `src/work_brief.rs` — new: the resolver (goal, plan resolution, the
  plan's execution mode, the non-plan expansion).
- `src/route.rs` — the judgment input; the execution head from the plan.
- `src/tools.rs` — the `plan` tool's `judge` action.

**Services:**
- `services/sys1/src/sys1/tasks.py` — the `route` spec; the `plan_judge`
  and `goal_work` tasks.

**Tests:**
- `src/work_brief.rs` `mod tests` — the resolver's fixtures.
- `tests/integration.rs` — the judgment-input, the plan judgment, and the
  expansion pins.

**Documentation:**
- `docs/plans/README.md` — the `judged` status.
- `docs/features/routing.md` — the leader's judgment input.
- `TECHNICAL-DOCUMENTATION.md` — the routing section.
- `FUNCTIONAL-SPECIFICATIONS.md` — the run behavior.

## Out of scope

- The child's route (already judged over its brief).
- The delegation nudge's labels and thresholds (unchanged).
- The plan's drafting surface beyond the `judge` action.
- The route cascade's providers and chains (unchanged).

## Verification

**C0/C1 — the plan judgment.**
```bash
cargo test plan_judge
```
Expected: a fixture plan's criteria judge to a stored `execution:` and a
`judged` status.

**C2 — the run's resolution.**
Expected: a mock lane receives the brief carrying the plan's verdict.

**C3 — the legacy path.**
Expected: a plan with no `execution:` is judged during route preparation,
before the run's loop, and the verdict is stored.

**C4/C5 — the execution head and the nudge.**
Expected: the execution head is the plan's verdict; the nudge fires.

**C6 — the non-plan expansion.**
Expected: a `goal_work` answer rides the brief when no plan is resolved.

**C7 — the live proof.**
On a scratch home with the lane up, the open-ledger plan is judged, then a
goal "implement plan as designed" routes with the delegation nudge.

**C8 — the specs.**
Expected: the `route`, `plan_judge`, and `goal_work` descriptions name the
brief and the verdict.

**C9 — the gate.**
```bash
cargo fmt --all --check && cargo clippy --all-targets -- -D warnings \
  && cargo build --release && cargo test
```
Expected: all four exit 0.

## Linked artifacts

- `docs/plans/README.md` — the `judged` status and the `execution` field.
- `docs/features/routing.md` — the leader's judgment input is the brief;
  the execution head is the plan's verdict.
- `TECHNICAL-DOCUMENTATION.md` section "Interface contracts" — the route
  judgment's input and the plan's verdict.
- `FUNCTIONAL-SPECIFICATIONS.md` section "Run behavior" — the nudge's
  evidence.

## Risks

- **The plan's judgment is a lane call at draft time.** It must degrade
  when the lane is absent: the plan keeps its status without a verdict, and
  the run falls back to judging the brief.
- **The plan signal.** The `judged` status is the verdict, not the
  resolution; the resolver still resolves the plan by the named / recency
  rule, and the log's write-class calls are noisy.
- **The non-plan expansion.** It adds a lane call to every goalless run; it
  must degrade to the goal text.
- **The legacy judgment.** A legacy plan's on-the-spot judgment adds a lane
  call before the run; it must degrade to judging the brief when the lane
  is absent.

## Dependencies

- `docs/plans/2026-11-05-open-ledger-closeout.md` — C6's live case.
- `services/sys1` — the home of the `route`, `plan_judge`, and `goal_work`
  tasks (C1, C5, C7).

## Notes

- The live session's log proves the signal: rec 840 (`write_file` the
  plan), rec 1159 (the status flip to `active`).
- The route task's spec already expects a compact state; the brief is that
  state plus the plan's verdict.
