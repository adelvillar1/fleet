---
name: judgment-beats-plan-built
description: "2026-10-25 — the judgment-beats-one-lane plan is built and completed (e4d3719): one lane session per beat run, execution head read and steering judged walks"
type: project
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

2026-10-25, commit `e4d3719`, plan `docs/plans/2026-10-23-judgment-beats-one-lane.md`
completed (6/6 criteria). What landed:

- **C0/C5** — `LaneSession` in `src/judge.rs`: open once (spawn, handshake, tools
  check), ask per row, close on the loop's one exit path; lazy open before the first
  row that will ask; a failed open is kept and named, and the remaining rows refuse by
  it with no second spawn. `delegate.rs` adopts judge's `LaneSource` and the same
  session (one implementation). The test seam is `LaneSource::Counting` +
  `CountingLog` (cfg(test)): N rows → ["open", N×"ask", "close"], one judged row per
  input row.
- **C1** — `parse_workload` reads both heads into `RouteJudgment`; `RouteDecision`,
  calib `RouteRow`, the route-cache `Verdict`, and `UiEvent::Routed` all carry
  `ExecutionHead {label, confidence}`; the reason names the head whether it steers or
  not. Absent head = recorded none, never invented.
- **C2** — a `Walk::Judged` whose head read mixture/swarm at or above the gate's floor
  walks ORCHESTRATOR_WORKLOAD's chain ("planner tier"); single/sub-floor/absent walk
  the judged workload's own.
- **C3** — the prompt's one compiled-in line comes from the run's OWN route decision
  at assembly time (not AgentOptions — the plan's suggested carrier was superseded by
  the single-source design, named in the plan) and rides a `Record::Routed` beside the
  event so the reconstruction replays it.
- **C4** — router board's live card, transcript route strip, three docs.

Two traps met: the scripted-lane mock (`scripted_lane`) served ONE classify entry —
under the session it must serve one per ask (grew an `asks` parameter); and the tree
was being co-edited live by another session's layerfs WIP — my commit staged
per-file blobs (reversing their hunks on copies, `git update-index --cacheinfo`) and
verified in a detached worktree. See [[shared-file-commits-can-sweep-another-sessions-work]].
