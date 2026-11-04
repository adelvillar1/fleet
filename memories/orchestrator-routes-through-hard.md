---
name: orchestrator-routes-through-hard
description: "Owner ruling 2026-10-16 — fleet's first model per run (the planner/supervisor) always walks the hard category's chain; the sys1 judgment still runs and the row records the judged label"
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

The owner ruled (2026-10-16): the orchestrator — the first model the router serves on a run, the one that plans, assigns the work, and supervises — must be whatever the roster assigns to the `hard` category (`stepfun/step-5-preview` today). Implemented in fleet commit `69c7d0d`: `route::select` walks `ORCHESTRATOR_WORKLOAD` ("hard") instead of the judged workload's chain; the judgment still runs, the decision row keeps the judged label (so `fleet judge` grading stays honest), the reason names both halves, a `routing.json` override of `hard` governs the walk, and the route-decision cache key carries `walk=hard` so pre-change caches serve nothing.

**Why:** the coordinating model's strength matters for every task shape; per-workload heads let quick tasks plan weak.

**How to apply:** any future routing work keeps the judgment/row semantics separate from the walk; never relabel the row. Delegation inherits the parent model (hard rule 9), so this head is the whole run's. Repo records: `docs/plans/2026-10-16-orchestrator-routes-hard.md`, `memories/orchestrator-walks-the-hard-chain.md`. Related: [[fleet-routes-through-its-own-router]], [[canonical-skills-library-agents-skills]].

**NARROWED by the owner 2026-10-21:** "the orchestrator, the session that leads
should be set to hard, everything else the router rules." The `hard` walk now
belongs to the LEADING session only; a delegated child gets its own route judged
over its brief (amending hard rule 9's inherit-the-parent-model), a provider pin
narrows any walk to that provider's own models, deepseek-flash is every walk's
universal failover tail, and no nominal model is hardcoded (start record carries
the route's model; `--model` is a pin). BUILT AND PUSHED 2026-10-21 in `4bd2f18`
(`Walk::Leader` / `Walk::Judged` / `route::prepare_child` / `route::with_tail`),
plan completed — see [[fleet-router-rules-everything-else-plan]] and
[[fleet-router-rules-everything-but-the-leader]].
