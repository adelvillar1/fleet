---
name: fleet-router-deep-dive-2026-10-21
description: "2026-10-21 read-only deep dive on route.rs — sys1's route task has only 5 workload labels (composed unreachable, execution head discarded), the orchestrator ruling collapses assignment to hard's chain (step-5-preview served 409/409 runs), owner gate raised to 0.72"
type: project
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Read-only deep dive (2026-10-21, owner-requested) over `src/route.rs` (4,306 lines),
`src/route_cache.rs`, sys1's route task, the live `~/.fleet` config, and four days of
the live decision stream. Extends [[fleet-routes-through-its-own-router]] and
[[orchestrator-routes-through-hard]] with the measured consequences.

**Pipeline (one session = one decision):** `route::prepare_with_pin` → `select` (exact
`provider/model` pin skips judgment; else sys1 `sys1_classify` task `route` via MCP →
confidence gate → workload's preference list) → the orchestrator ruling walks `hard`'s
chain whenever the table has one → `Router::chat_stream` failover (commit on first
token; mid-stream failure is fatal, no silent restart; 400/404 never failover) → one
`calib::RouteRow` per session with attempts, `how`, `judged_from`, and a reason
assembling every clause (gate, orchestrator, demotion, calibration skip, lane words).

**Structural facts worth remembering:**
- sys1's route task vocabulary is FIVE labels — `quick`, `standard_code`, `hard`,
  `prose`, `deep_context` (services/sys1/src/sys1/tasks.py:519). It can never emit
  `composed` or `vision`, so the blend1 lane candidate is unreachable in production
  except by manual pin — this IS the open leg of docs/plans/2026-10-15-router-enforcement.md.
- sys1 also judges an execution head (`single`/`mixture`/`swarm`) that
  `parse_workload` discards — the natural hook for future swarm/mixture work, unused.
  UPDATE 2026-10-23 evening: the owner reports the router now HAS the agent
  fan-out execution type (swarm) — the head is being consumed by the
  judgment-beats plan's build ([[fleet-judgment-beats-spawn-per-row]]).
- The workload chains are also the nearest thing fleet has to a named
  WORKFLOW REGISTRY: seven labels in a preference/tier table, selected per run
  and recorded per run in `RouteRow`. 2026-10-23 the owner pointed this out
  when a plan declared workflows unmeasurable; sizing them by selection
  frequency is the natural basis for masking a workload nobody selects
  ([[fleet-tool-availability-gaps-2026-10-23]]).
- The vision workload never goes through judgment: `Work::Vision` builds a side-router
  over `workload_decision("vision")` (GLM-5.3-Flash → step-5-preview → gemma4:31b),
  reading the COMPILED-IN table, not `routing.json` overrides (named in code).
- DeepSeek is structurally protected: no default workload targets it, so it serves
  only as a failover tail or an owner pin.
- Owner's live config: `routing.json` raises `min_confidence` to 0.72 (default 0.6);
  config.json has no provider overrides, pin empty.

**Live stream, Oct 18–21 (409 route rows):** step-5-preview served 409/409; one real
failover (rate-limit ×2 → GLM-5.3-Flash). Fresh judgments 274 (median ~1.1 s, max
8.2 s), cache hits 62, no-verdict 73 — 65 in one Oct-20 stretch (a silent sys1 outage
the fallback absorbed with zero failed runs). Demotion (3 consecutive failures) and
calibration skips (fit cell ≥8 rows, span ≥0.30, ≥50% weighted-wrong, shape-scoped)
never fired in the window. 17/39 cached verdicts sit below the 0.72 gate, which is why
408/409 rows record `standard_code`.

**Why:** the router's most expensive component is decorative for model assignment —
the judgment runs and is recorded but never moves the walk (the orchestrator ruling is
deliberate), so honest claims are "always hard's chain, failover on failure."

**How to apply:** never claim the judgment steers which model serves; it only labels
rows and scopes calibration cells. The named levers if the owner wants the ~1 s back
or wants assignment to vary: a sixth sys1 label (composed), consuming the execution
head, or letting `quick` downgrade within the planner tier. Related: [[blend1-is-the-latency-triad]].
