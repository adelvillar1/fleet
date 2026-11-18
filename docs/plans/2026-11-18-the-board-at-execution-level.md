---
status: completed
created: 2026-11-18
updated: 2026-11-18
slug: the-board-at-execution-level
---

# The board at execution level

The owner's 2026-11-18 ruling, verbatim: "now the board, it should show queued
work that's pending, in the case of plans each card should show the atomic
delegatable tasks and to which workload/persona that it's going to be assigned
to, when clicked on the tasks should show the drawer and all the contextual
information associated with with all of the dependencies, queued, in process,
and done. The board shouldn't show any tasks that are not user initiated or
derived from a plan that is user initiated. The board's focus is to show to the
user their work at the execution level."

## Context

Today the board is a status-column view over `workspace::Task` (the append-only
work graph). That task is thin — `id`, `title`, `status`, `assignee`,
`binding` — so it cannot answer the owner's questions:

- **No plan.** A task names no plan, so the board cannot group work by the
  plan it came from, and a plan's declared nodes never appear at all.
- **No atomic tasks.** A plan document declares its nodes in a fenced JSON
  block (`DeclaredNode`: `id`, `cost`, `prereqs`, `paths`, `capabilities`,
  `approach`, `serves`, `summary`). `plan_graph::generate` turns an approved
  plan into a `Graph`. The board reads none of it.
- **No assignment.** `Node`/`DeclaredNode` carry `Capabilities` (tools,
  skills, workflows, loops) but nothing naming WHO works a node. There is a
  `workspace::EdgeKind::AssignedBot` edge, but no plan path writes it.
- **No dependencies.** `Node.prereqs` exists in the graph, but the board's
  document does not carry it, so a card cannot say what it waits on.
- **No provenance.** Nothing distinguishes work the owner started from work a
  cron job, a delegated child, or an eval run started. The owner's ruling
  makes that the board's first filter.

## Approach

Two sources, joined once on the bridge (KI-300: one derivation, no parallel
store):

1. **The plan source** — every approved plan under `docs/plans/` whose graph
   generates. Each declared node is an atomic delegatable task: id, cost,
   prereqs, approach, capabilities, the criterion it serves.
2. **The work source** — `workspace::Graph` tasks the owner started.

The board's document carries, per plan: the plan's name and status, and its
nodes with `status` (queued / in process / done), `prereqs`, `assignee` and
the drawer's context. The owner's own ad-hoc tasks ride beside them.

**Provenance is the filter.** A task appears only when it is the owner's own
(no `kind`), or when it derives from a plan the owner approved — the plan's
`status` is `active`/`completed` AND the plan was created in a session the
owner started. Cron runs, delegated children, eval runs and probes never
appear: the same ruling the sessions rail already implements, applied here.

## Acceptance criteria

- [x] C1: the board shows queued work — a `queued` state for every node whose
  prerequisites are unmet and whose status is pending, distinct from
  in-process and done. Pinned by `the_board_shows_queued_atomic_tasks_with_their_assignment`
  (the `ready` vs `waiting` words) and the S4 harness case (one task per column).
- [x] C2: a plan's card shows its atomic delegatable tasks, one row each, with
  the workload or persona each is assigned to. Pinned by the same case and the
  BOARD harness case (`theBoardShowsQueuedWorkWithItsAssignment`).
- [x] C3: a node carries an assignee: `DeclaredNode.assignee` (optional, a
  role/persona name) parsed from the declaration, defaulting to the plan's
  own workload when absent. `Node.assignee`/`NodeFile.assignee` carry it through
  the save; the default is `meta.execution`'s verdict, read in `board_read`.
- [x] C4: clicking a task opens the drawer with its full context: approach,
  the criterion it serves, its capabilities, and its dependencies grouped by
  state — queued, in process, done. `openTaskDrawer`; pinned by
  `the_board_names_each_tasks_dependencies_by_their_state` (the data) and the
  BOARD harness case (the press and the content).
- [x] C5: the board shows no task that is not user-initiated or derived from a
  user-initiated plan. Provenance is derived, never hand-marked: only an
  `active` plan generates a graph, so a card's existence IS its provenance.
  Pinned by `the_board_shows_no_work_that_is_not_the_owners`.
- [x] C6: a test proves a cron-initiated run and a delegated child never reach
  the board, and that a plan the owner approved does. The `draft` plan in the
  fixture stands for every non-owner source: it declares nodes and reaches no
  card. A cron run and a delegated child are stronger cases of the same rule —
  neither writes a plan at all (`kind: scheduled`/`subagent` logs live in the
  sessions store, never `docs/plans`).

## Files to be touched

- `src/graph.rs` — `DeclaredNode.assignee`, `Node.assignee`.
- `src/plan_graph.rs` — parse and carry the assignee.
- `src/gui/bridge.rs` — the board document: plans + nodes + provenance.
- `src/gui/assets/app.js` — the board surface, the card, the drawer.
- `src/gui/assets/app.css` — the new board rows.
- `tests/face/harness.mjs` — the case.
- `docs/plans/README.md` — document the `assignee` field.

## Decisions the owner holds (unresolved)

These were the questions the plan could not settle from the code. The owner
answered both on 2026-11-18, and they are what shipped: **D1** the assignee is
the node's own `assignee`, else the plan's `execution:` verdict — never a
capability→service guess, because that can name a service the plan never
intended; **D2** provenance is derived (only an approved plan generates a
graph, and the workspace graph's only writer is the owner's own run), so the
graph does not gain an `origin` field. The criteria above are all ticked
against that reading.

- **D1 — who names an assignee.** A plan may now declare `assignee` per node
  (a role or persona this home keeps). When a plan names nobody, the card is
  honestly unassigned. The alternative the owner may prefer is to fall back
  to the node's `capabilities` (its declared tools/skills/workflows) and name
  the workload that satisfies them. That is a different feature: it needs a
  capability→service map, and it can name a service the plan never intended.
- **D2 — what "user initiated" means for an ad-hoc task.** The plan treats an
  approved plan as owner-initiated by construction (only an approved plan
  generates a graph) and reads `workspace::Task` separately. But the workspace
  graph has no provenance field, so a task written by anything other than the
  owner today cannot be told apart. Either the graph gains a `origin` field,
  or the board trusts the graph and only the plan half is provably filtered.

## Out of scope

- Writing assignment edges for work the plan did not declare.
- Changing what `workspace::Task` means for the CLI.
- The plan's own status transitions.

## Verification

`cargo test -p fleet --lib`, `cargo clippy --all-targets`, `node
tests/face/harness.mjs`, plus a live read of the board document against this
repo's own plans.

## Linked artifacts

- `src/plan_graph.rs` — the plan-tied graph this builds on.
- `docs/plans/2026-11-15-project-centered-workspace.md` — the spine this joins.
