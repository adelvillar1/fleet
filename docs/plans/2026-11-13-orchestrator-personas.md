---
title: the orchestrator — personas carry workloads, own their threads end-to-end, and carry their own work graphs
date: 2026-11-13
status: completed — verified against the shipped code and closed 2026-11-16; C7's deploy done locally
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

- [x] C1: `Role.workload: Option<String>` — flat frontmatter, parsed with
  the known keys, validated against `route::JUDGED_WORKLOADS` at parse time
  (unknown = named refusal); `write` preserves it; `fleet roles` cards
  render it.
- [x] C2: the route's named-workload walk arm — the walked workload is the
  named one when the chain resolves, else the judged workload (named
  degradation, never a mid-run refusal); the judgment always runs; the
  decision row records the judged label AND the walked workload
  (`walked_as`).
- [x] C3: the ownership index (`sessions-roles.json`, written at every
  dispatch under a role — subagent, room members, the orchestrator
  assignment) + the role's view reading it end-to-end (live, ended,
  archived alike) + **the role's work graph** — the spine's shape scoped to
  what the role owns, derived from the index and the workspace graph's
  edges, rendered beside the owned threads in the persona section and the
  role card.
- [x] C4: the conversation's orchestrator assignment — the Start record
  carries the orchestrator persona; `drive_run` resolves the lead persona's
  workload → the named walk; no assignment → `Walk::Leader` → `hard`,
  byte-identical to today; the orchestrator writes its own ownership entry.
- [x] C5: the face — the roles document carries workload + owned-thread
  count + the work graph section; the settings role card and the persona
  authoring door (a workload picker) show it; the conversation's identity
  surface names its orchestrator.
- [x] C6: the docs name the orchestrator's duties (delegating to role bots,
  convening rooms, authoring + convening personas — existing machinery,
  re-homed under the orchestrator identity).
- [x] C7: the gate; commits on master, push; local deploy (rm-then-cp →
  face relaunch); no release steps.

## Closed 2026-11-16 — the verification, as measured

The plan's seven criteria stood open with the work shipped (`8ae2065`); a
sweep verified each against the tree and closed the two real gaps:

- **C2's row half** was missing: `RouteDecision.walked` existed but the
  calib row did not carry it. `RouteRow.walked_as` now rides beside the
  judged `workload` (absent when they agree), pinned in
  `the_decision_row_carries_the_devdec_vocabulary`. **Live proof:** the
  probe dispatch's parent row reads `workload quick · walked_as hard` with
  the reason "the orchestrator walks `hard`'s chain", and the child's row
  (the role's workload) reads `quick` with no `walked_as` — the two halves
  of the pair, in one run.
- **C3's graph half** was missing: the ownership index and the owned-thread
  list existed, there was no graph. `roles::work_graph` now derives the
  nodes from the index and the delegation edges from each child's own
  `start` parent (`the_work_graph_is_the_index_and_the_childrens_own_start_records`),
  and the role card renders it (root, nodes, nesting; `its work graph`).
- **C6's docs** were missing: `docs/features/orchestrator.md` names the
  duties and the records; `FUNCTIONAL-SPECIFICATIONS.md` carries the
  paragraph.
- C5's orchestrator on the identity surface: `state.orchestrator` (the
  lead persona off the log's `start`) drives a `orchestrator · <name>` chip
  on the conversation head, and a room's lead chip carries the word
  (`theOrchestratorIsNamedOnTheConversation`).

**Live run, 2026-11-16 13:07-13:08:** a probe role with `workload: quick`
was authored, the owner's task dispatched a `subagent` under it — the
child's log names ROLE: probe, the ownership index gained the thread, the
roles document read `probe · workload quick · owned 1`, and the role card
rendered its work graph with the thread as its node. The probe role and
its index entry were then retired.

Found while verifying, filed not fixed: **KI-552** — the settings detail
pane and the connections column paint over each other at 1440 width with a
role card open (text over text), measured in the screenshot of the live
face; the open-state stacking needs a real-browser reproduction before a
fix is chosen.

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
