---
name: turn-budget-shape-is-a-deferred-discussion
description: "Deferred design discussion — whether the run's cap should stay turn-driven or move to a token/context budget that degrades like the gauge measures"
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

Recorded 2026-11-03 in `docs/plans/2026-10-31-quota-failover-and-compaction-count.md`
(decision 3's amendment): the owner ruled turn count is meaningless to end users —
the composer's turns-left field is retired, and the context gauge is the one
degradation signal a user sees. The turn count stays internal.

**The deferred discussion:** whether the run's *cap* itself should stay
turn-driven. The owner's framing — 50 turns of 2,000 tokens each is a whole
different case from 20 turns, or from 50,000 tokens — so a turn cap is a crude
proxy for what actually matters (context degradation). When the owner returns
to this, the question to answer: should the cap move to a token/context budget
that degrades the way the gauge already measures? The machinery
(`runs.max_turns`, `DEFAULT_MAX_TURNS`, the `turn_budget` clamp) stays as it is
until that is decided. Related: [[self-containment-is-a-standing-contract]].

**2026-11-04: the discussion arrived, and the SURFACE half landed.** The owner
asked for context health measured and shown on the conversation beside the
working spinner, steered "the compaction should be a function of the main model
context window length and not hardcoded," and the whole thing LANDED — see
[[context-health-research-2026-11-04]]. The turn count no longer faces the user
anywhere (the working line reads occupancy/folds/turns-to-fold). **Still
deferred:** whether the turn-budget MACHINERY (`runs.max_turns`,
`DEFAULT_MAX_TURNS`, the `turn_budget` clamp) itself retires now that context is
the visible signal — the plan named it out of scope and the owner has not ruled.
