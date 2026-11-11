---
name: hard-chain-leader-walk-and-the-400-no-failover
description: "project 2026-11-06 — the live proof's stepfun 400 was a SCRATCH-HOME artifact (no routing.json → compiled defaults walked, stepfun hardcoded at route.rs:616-624); the owner's real chain (opencode first, zero stepfun) REPLACES per workload; the account-400 no-failover defect stands regardless"
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

During the goal-task-pane live proof (2026-11-06, scratch home :3099) the
warm-up run died: `API error 400: you have no active step plan subscription`
from the hard chain's first candidate (stepfun/step-5-preview) — **the walk
stopped there instead of handing to the deepseek tail**. Root cause, read in
`src/route.rs:2655-2675`: `failure_class` deliberately classes 400/404 as
`client-error`, not failover-eligible, "every candidate would fail them the
same way" — with one exception when the body names the MODEL unavailable. But
a subscription/account 400 is provider state, not a bad request: the same body
served fine on deepseek once pinned. **Defect FILED as KI-451** (account-400 no failover; the cure is one arm
beside `names_model_unavailable`). Workaround used: the run pin
(`{"pin":"deepseek"}` rides POST /api/run and persists in
`state.provider_pin`, which the goal auto-start reads), then deepseek-flash
served.

The owner asked **"why do we have a hard chain, the router should decide"**
— answered from the tree: `ORCHESTRATOR_WORKLOAD = "hard"` (route.rs:722) is
the owner's own 2026-10-16 ruling narrowed 2026-10-21 — the LEADER
(planner/supervisor) walks the strong chain whatever the task was judged;
children and side-routes already walk the judged chain; the execution-head
steer (route.rs:2053-2066) lets the router escalate a judged walk to hard
when the head names mixture/swarm confidently.

The owner's follow-up — **"stepfun is no longer in the chain, that means
that it's hardcoded somewhere"** — was RIGHT, and the 400 turned out to be a
scratch-home artifact, not the owner's config being ignored:
- stepfun IS hardcoded in the compiled fallback table:
  `default_preferences()` (`src/route.rs:616-624`) opens `hard` with
  `("stepfun", "step-5-preview")`, frozen at the 2026-10-16 migration state
  (the CLAUDE.local.md note "the compiled table still matches the kit's live
  one" is now stale).
- A home's `routing.json` **REPLACES each workload's chain wholesale** —
  `load_routing` does `slot.candidates = candidates` (`src/route.rs:1228-1234`),
  no union with defaults; NotFound → the compiled defaults whole
  (`src/route.rs:1202-1206`). The owner's real `~/.fleet/routing.json` has
  zero stepfun: hard = `opencode/deepseek-v4.1-flash →
  ollama-cloud/glm-5.3-flash → xiaomi-mimo/mimo-v2.6-pro`.
- The scratch home had NO routing.json → the compiled fallback walked;
  the key was live because `~/.fleet/.env` still carries STEPFUN_API_KEY
  and the server env was sourced from it.
**Lessons:** a live proof's scratch home must carry the owner's
routing.json (else it walks a chain they retired); `~/.fleet/.env` keys
outlive their provider's roster exit. Open owner calls: refresh or slim the
compiled fallback, amend hard rule 9 or not. The 400-classification defect
stands regardless of which chain walks.

The live proof resumed after the pause and completed — see
[[goal-task-pane-plan-drafted-2026-11-05]].
