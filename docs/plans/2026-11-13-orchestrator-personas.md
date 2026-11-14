---
title: the orchestrator — personas carry workloads, own their threads end-to-end, and carry their own work graphs
date: 2026-11-13
status: active
---

# The orchestrator: personas carry workloads, own their threads end-to-end, conversations carry orchestrators

The owner's design, in four rulings:

1. **"Just assign a workload to a persona"** — a persona/role carries a named
workload, and runs under that persona route through that workload's chain.
2. **"Any conversation gets an orchestrator workload/bot assignment — this
orchestrator performs the duties of the router, and can ask other bots to
join or spin up other bots for any given role."**
3. **"The role owns all the threads assigned to it, even if the dispatched
conversations/tasks used a thread that's already been disposed of — the role
owns the thread end-to-end."** Ownership is by assignment, permanent, and
lifecycle-independent.
4. **"Each role carries the equivalent of a persona work graph with it"** —
the spine's own shape, scoped to what the role owns.

The routing mechanism half-exists twice (`Walk::Leader` → the `hard`
orchestrator workload at route.rs:2178; `carries_image` → the forced-vision
walk — both "route by name, judgment recorded but not deciding"), and the
persona dispatch seams are mapped (subagents.rs:2067 holds the resolved role
before `prepare_child`; bridge.rs:14053-14074 is the conversation's route
site; the log already carries the lead persona).

**The load-bearing rules**: the named workload is a **standing default the
judgment still runs under** (sys1 judges every task, the row records the
judged label — no bypass of the closed set); and **ownership is by
assignment** — a dispatch under a role writes a durable attribution that
outlives the thread's own lifecycle. Default behavior is unchanged: a
conversation with no assignment walks `hard` exactly as today.

## Acceptance criteria

- [ ] C1: `Role.workload: Option<String>` — flat frontmatter, parsed with
  the known keys, validated against `route::JUDGED_WORKLOADS` at parse time
  (unknown = named refusal); `write` preserves it; `fleet roles` cards
  render it.
- [ ] C2: the route's named-workload walk arm — the walked workload is the
  named one when the chain resolves, else the judged workload (named
  degradation, never a mid-run refusal); the judgment always runs; the
  decision row records the judged label AND the walked workload
  (`walked_as`).
- [ ] C3: the ownership index (`sessions-roles.json`, written at every
  dispatch under a role — subagent, room members, the orchestrator
  assignment) + the role's view reading it end-to-end (live, ended,
  archived alike) + **the role's work graph** — the spine's shape scoped to
  what the role owns, derived from the index and the workspace graph's
  edges, rendered beside the owned threads in the persona section and the
  role card.
- [ ] C4: the conversation's orchestrator assignment — the Start record
  carries the orchestrator persona; `drive_run` resolves the lead persona's
  workload → the named walk; no assignment → `Walk::Leader` → `hard`,
  byte-identical to today; the orchestrator writes its own ownership entry.
- [ ] C5: the face — the roles document carries workload + owned-thread
  count + the work graph section; the settings role card and the persona
  authoring door (a workload picker) show it; the conversation's identity
  surface names its orchestrator.
- [ ] C6: the docs name the orchestrator's duties (delegating to role bots,
  convening rooms, authoring + convening personas — existing machinery,
  re-homed under the orchestrator identity).
- [ ] C7: the gate; commits on master, push; local deploy (rm-then-cp →
  face relaunch); no release steps.

## Verification

The full gate + harness, then live: author a persona with `workload:
quick`, dispatch to it — the row records the judged label, the quick chain
walks, the ownership index gains the thread, the persona's work graph
renders it; archive/dispose the thread — the persona's view still lists it
end-to-end; an unassigned conversation walks `hard` byte-identical to
today.

## Out of scope (named, not built)

Per-turn re-preparation for per-speaker chains inside one room run; the
router's judgment becoming an LLM opinion (sys1 stays the closed-set
source); auto-spin-up without the owner's seat; retroactive ownership for
the 815 archived conversations (ownership begins with this feature's
dispatches).
