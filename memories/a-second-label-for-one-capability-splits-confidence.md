---
name: a-second-label-for-one-capability-splits-confidence
description: "I dismissed the owner's 2026-10-31 router-label request by arguing design already walks frontend's chain — WRONG: the Leader walk is pinned to hard, design runs served GLM-5.3-Flash; the owner caught it, and the correction is that an empirical routing claim must be read off a real run, never inferred from the preference table"
type: feedback
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 2
superseded-by:
valid-until:
---

**CORRECTED 2026-10-31 — the earlier "no, it doesn't earn its keep" answer was wrong and the owner called it out** (*"that's why I asked a few turns ago … you dismissed my request and pedantically said it needed to earn its keep, is it earned now?"*).

**The error, precisely:** asked whether site design should be a router workload, I argued it already lands on `frontend`'s chain (`deepseek-flash → GLM → ollama`) because that label's description *is* "interface surface". I read that off `default_preferences()` **without checking the override that supersedes it**: the owner's standing ruling (in `CLAUDE.local.md`, and `route.rs`'s `Walk::Leader` branch) is that **an unpinned leading run walks the ORCHESTRATOR workload (`hard`) whatever the task was judged** — the label is recorded and then ignored.

**The evidence that settles it:** every design run's `end` record read `served_model: "GLM-5.3-Flash"` — i.e. `hard`'s chain fell through stepfun and mimo (no keys on this machine) to GLM, a large reasoning model. That is why a board "ask" burned minutes and explored the console with `browse`; the design work was never routed as design. (The early `routed` records carry only the execution head, the label/chain loss is a separate logging gap.)

**The lesson (the durable part):** a claim that a feature "already routes correctly" is an *empirical* claim — verify it from a real run's `end`/route row, or from the code path that actually decides (here the Leader-walk override), never from the preference table alone. When the owner asks for a routing category and you answer from memory of the table, you are guessing at the one thing they can see from their logs.

**OUTCOME (2026-10-31, later):** none of the three owed items shipped as written. The owner ruled the router must not route the designated design models at all — routing comes from the design page, which names one of three and sends a pair pin — so the `design` workload was built, then reverted whole (`b46383d`). The lesson above stands; the "design label + orchestrator exception" is superseded by [[design-routing-comes-from-the-design-page]].

**What was owed at the time, in the order it should be built** (both are the answer to "is it earned": the routing decision is earned by the observed misrouting; the label alone is not the fix):
1. **A design pin from the composer** — the design composer already passes `pin: face.providerPin` into `/api/run`, so giving it a design-model default bypasses the orchestrator override with no ruling change.
2. **A `design` label *plus* an orchestrator exception** — the label only earns its keep beside a change to the 2026-10-16 ruling ("may the leader walk be judgment-driven for design-shaped tasks?"). That needs the vendored sys1 label, `JUDGED_WORKLOADS`, the settings' unjudged marking, and `profile_rank`.
3. **The replay I owe regardless:** ~10 real design briefs through live sys1, to confirm the label comes out distinct from `frontend` *before* it steers anything.

Still true and worth keeping from the first answer: labels split confidence at fuzzy boundaries ([[curator-judgment-plan-2026-10-30]] showed asking too fine a question degrades the vote), and the capability axis (mixture/swarm) plus a per-run pin are the cheap levers. That reasoning is fine — it was the *premise* that was false. Related: [[fleet-frontend-workload-2026-10-27]] (how that label was actually ruled), [[design-canvas-complete-plan-2026-10-31]], [[a-task-must-carry-what-the-model-would-have-to-hunt]] (the same misrouting's cost, seen from the task side).
