---
name: orchestrator-walks-the-hard-chain
description: "the first model a run loads - the planner that assigns and supervises - always walks the hard category's chain whatever the task was judged; the sys1 judgment still runs and the route row records the judged label."
type: project
date: 2026-10-16
basis: confirmed-by
tier: durable
shape: [change]
phrases: [the orchestrator, the first model that loads, the planner model, hard category, strong model plans, who supervises the run]
missing: []
recall-count: 81
superseded-by:
valid-until:
---

The owner's ruling (2026-10-16): the orchestrator — the first model the router serves on a run, the one that plans, assigns the work, and supervises — must be a strong model, whatever the roster assigns to the `hard` category. In code (commit `69c7d0d`): `route::select` walks `ORCHESTRATOR_WORKLOAD`'s ("hard") chain instead of the judged workload's, while the sys1 judgment still runs and the decision row's `workload` field stays the judged label — `fleet judge` grades the judgment, and a relabeled row would corrupt the grading. The reason names both halves when they diverge; an owner override of `hard`'s candidates in `routing.json` governs the walk; the route-decision cache key names the walked workload, so pre-change caches serve nothing. Delegation children inherit the parent's model (hard rule 9), so the strengthened head is the whole run's.
