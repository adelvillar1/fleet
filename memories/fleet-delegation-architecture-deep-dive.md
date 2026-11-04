---
name: fleet-delegation-architecture-deep-dive
description: "2026-10-19 deep dive — fleet has no task tree (by ruling, not by gap), delegation is flat, and the dependency graph was complete and well-tested but had no production driver; the owner's design direction was planned and FULLY IMPLEMENTED the same day (P0-P7, commit bf07611, gate 1252)"
type: project
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

> **Status update, end of 2026-10-19: COMPLETE.** The plan this deep dive led
> to ([[fleet-plan-tied-dependency-graph-plan-drafted]]) was implemented the
> same day, all eight increments, committed as `bf07611` and pushed. The gate
> is green at 1252 passed. The graph now HAS a production driver: declared by
> a `plan` tool into an approved plan document, persisted in a gitignored
> sibling, verified by a three-gate fact-gate in `src/verify.rs`, dispatched
> on zero dependency score, with summary expansion (`DeltaReason::NotAtomic`
> + `merge_declared`) and cross-plan dependencies (`PlanRegistry`) both
> landed. The write-path lock closed the silent cross-session race. There is
> still no task tree — but that is now the ruling, not a gap. The diagnosis
> below is the history of how that was reached and is still accurate as of the
> morning.

The 2026-10-19 deep dive of delegation, task tree, dependencies, and the
dependency graph (`src/subagents.rs`, `src/delegate.rs`, `src/graph.rs`,
`src/roles.rs`) found that three of the four exist and one does not.

**There is no task tree.** Delegation is flat: one `subagent` tool call =
one child = one result. The tool's schema carries `goal`, `tools`,
`max-turns`, `context`, `output`, `approach`, `role` — no dependency
parameter anywhere in the schema, request, brief, or scope. A parent
expresses "B waits for A" only as English prose inside B's `goal` string,
and the harness has no idea that is a dependency. The two tree-shaped
things are records, not plans: `Chain` (the session lineage, reconstructed
after the fact by walking `parent_of` up the logs) and the `Delta` tree
(`children: Vec<Delta>`, which rides up the chain verbatim through log
text).

**The dependency graph was a finished component with no integration
point.** `graph.rs` is the strongest engineering in the area — correct
score arithmetic, named errors, cycles refused at declaration, a
writer-conflict resource edge, a fact-gated `settle`, and a replan loop
with four priced delta categories. And nothing drove it: the mutation
API (`add`, `dispatch`, `settle`, `fail`, `record_delta`, `replan`,
`revise_cost`, `add_prerequisite`) had no production caller — only the
module's 37 tests and two `eval.rs` scenarios. `Graph::save` had exactly
one caller in the whole codebase, `src/eval.rs:4589`, a test. `graph.json`
was only ever *read* and had never been written by production, so
`fleet graph` rendered a file nothing produces and the boards delegation
panel was always empty. The plan that commissioned it described nodes
"declared before dispatch" but no doc named any surface that creates a
node.

**What actually orders sibling work** was `plan_batch`'s writer-conflict
wave plan: waves capped at `DELEGATION_CONCURRENCY = 3`, the one
structural rule being that two writes to one path cannot share a wave,
with delegations as `Prepared::Serial` at the head of their wave. That is
a file-locking discipline, not a dependency graph.

**Disambiguation that matters:** the "knowledge graph" in the
2026-10-18 memory-and-cache plan is the memory *recall* graph
(`recall_graph.rs`), an unrelated subsystem. Do not conflate them.

**The graph's hazards, all eight now fixed by P1** (they were named here
first): `dispatch` had no status guard so it could demote a `Verified`
node back to `Running` and raise every dependent's score; `replan`'s
falsified filter keyed on reason and a non-empty `instead` but never on
`contradiction`, so the one category meaning "don't trust this" was
trusted to rewrite a real approach; `save` was a bare `fs::write` with no
temp-and-rename; `record_delta` was an unconditional overwrite so a
second report could *lower* `diff_score` and un-fire a replan;
`REPLAN_CAP` was enforced only at replan time, so a hand-edited file with
three revisions loaded and replanning was permanently dead; `peak` was
not serialized and degraded across a round trip; the score summed over
*paths* rather than over the closure (see
[[fleet-graph-score-double-counts-shared-deps]], fixed by P2); and a
`Failed` node blocked its closure forever with no cancel edge — P1 added
`is_terminally_blocked` to name it.

**Delegation hazards, still open.** The delta parser takes the *first*
block in a child's message, splits on any line exactly equal to a marker,
and never verifies a `status: followed` claim at all (`verify` runs only
for `Deviated`); nothing reaps an orphaned child, and the boards surface
renders a log with no `end` record as `"running"` forever;
`SubagentResult.goal` is fed the *composed* brief so the report's `goal:`
preview renders `goal: --- TASK ---…`; and the whole orchestration half of
`subagents.rs` past the brief parse is untested.

**Doc drift found alongside, still open.**
`docs/features/subagents.md:151` documents `SUBAGENT_MAX_DEPTH = 1` when
the code says 2; `:236-237` states "Not concurrent. The parent's turn
waits" absolutely when D4 landed concurrent execution; the
admissible-scope enum omits `web`; two source pointers point at docs that
never mention the subject (`FUNCTIONAL-SPECIFICATIONS.md:25` →
`calibration-loop.md` for the delegate beat,
`TECHNICAL-DOCUMENTATION.md:175` → `cron.md` for graph.rs); and the
delegation-steering plan contradicts itself on its own evidence (line 303
"ten grandchild spawns observed" vs line 764 "no depth-2 spawn
observed").

**The owner's design direction (given after the dive):**

1. **Declaration comes from a `plan` tool**, from the root node down to
   atomic-level tasks. This superseded the minimal `id`/`depends_on` shape
   I first recommended.
2. **The depth problem is the crux** — the hierarchy goes two levels deep
   but atomic tasks may sit lower. The owner's two candidates: promote a
   sub-subagent to sub-agent, or make the hierarchy deeper.
3. **Dispatch is for tasks with a zero dependency score** — atomic
   regardless of level. This was already the implementation.
4. **The fact-gate is three machine-checkable gates**: functional, unit
   tests, and nomenclature.
5. **The graph must be persistent as the feature contract** and remain
   after completion, for traceability.
6. **Maybe no tree is needed** — the graph probably already satisfies the
   work-breakdown role.
7. **Demotion may be needed** when a node's work does not integrate.

**Promotion beats a deeper hierarchy, and the reason is arithmetic.**
`turn_cap_for_depth` is `(SUBAGENT_TURN_CAP >> depth-1).max(5)`, so depth
1 → 20, depth 2 → 10, depth 3+ → 5. A four-level hierarchy whose bottom
two levels get five turns each can barely act. So the orchestrator
re-dispatches a returned sub-plan at the top rather than spawning deeper
— which is exactly what P6 C4 built.

**Why:** the owner asked for a deep dive expecting four subsystems; the
answer is that the most impressive one was undriven and the one the
subsystem is named after did not exist. The graph was never the missing
piece — the seam where a parent's decomposition becomes nodes had never
been built. That seam is now built, and the owner's ruling settled its
shape.

**How to apply:** as of the evening of 2026-10-19 the graph IS driven
(plan-declared, plan-persisted, three-gate verified, zero-score dispatch,
summary expansion, cross-plan edges). Do not re-derive the design from
this dive — start at
[[fleet-plan-tied-dependency-graph-plan-drafted]]. The one thing still
missing is the wiring of `admit_now` into `execute_calls`: the wave loop
still runs a plan computed once from the calls the model emitted, rather
than re-consulting the graph after each completion. The delegation
hazards and doc drift listed above are also still open. Related:
[[fleet-graph-is-one-per-plan]] for the scoping ruling the plan is built
on, [[fleet-cross-session-dependency-problem]] for the problem P0 closed,
and [[fleet-unlock-signal-is-arithmetic-not-a-judgment]] for why sys1 is
the fact-gate's functional half and not the unlock signal's generator.
