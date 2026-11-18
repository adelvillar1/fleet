---
status: completed
created: 2026-11-18
updated: 2026-11-18
slug: a-plan-node-finishes-when-its-work-does
---

# A plan node finishes when its work does

The owner's 2026-11-18 report, verbatim: "the collapsible pane is not showing
the live status of the execution of the plan's tasks as it should, it only
showed the first task as active and did not move throught the rest of them as
they were being executed."

## Context

The board and the pane both read `Status` off the plan graph, and the graph
has four transitions. Measured against the code, only two of them have a
writer on the delegation path:

| transition | writer | called from a run? |
|---|---|---|
| `Pending → Running` | `Graph::dispatch` | **yes** — `subagents::dispatch_node`, when a child is bound to a node |
| `Running → Failed` | `Graph::fail` | **yes** — `subagents::mark_node_failed`, when the child fails |
| `Running → Verified` | `Graph::mark_verified` | **no** — its only caller is `verify::promote`, and `verify::` has no caller anywhere in the crate |
| `Running → Pending` | `Graph::demote` | only from a re-plan |

So a node that a child works successfully **stays `Running` forever**. The
board reads that as `in process`; the pane shows it as the active task. The
owner saw the first task go active and nothing after it — not because the
pane failed to repaint, but because no later node can ever acquire a status
that differs from `Running`, and the first one never leaves it.

Two further facts that shape the fix:

1. **The pane polls nothing.** `board_read` is a fresh read, but the face
   refreshes it only on arrival and on a 4-second cache expiry
   (`readBoard`). A run that takes minutes would still need a repaint to
   move a card, even once the statuses are right.
2. **The verification gates already exist and are unused.** `verify::promote`
   reads a node's artifact against its gates and calls `mark_verified` for
   each node that passes. It is written, tested, and orphaned.

## Approach

Two halves, and both are needed — a status that is written but not repainted
looks exactly like this bug, and a repaint over an unwritten status looks the
same.

**1. Write the completion.** When a delegation bound to a plan node completes,
the node is settled by the same rule a failure already uses:
- the child's outcome is `Completed` **and** its own log verifies →
  `mark_verified`;
- `Completed` without verification → the node stays `Running` and the child's
  report names why, because a node claimed done without evidence is the one
  thing the graph's `done` is defined to exclude;
- `MaxTurns`/`Failed` → the existing `mark_node_failed` path, unchanged.

`verify::promote` is the gate to use rather than a new one: it already reads
the artifact against the node's gates and is the module's stated purpose.

**2. Repaint while a run works.** The pane and the board reread their document
on a bounded interval while a run is in flight — the same shape the run strip
already uses for liveness, not a second clock. A settled run stops the poll.

## Acceptance criteria

- [x] C1: a node whose child completed and whose log verifies is `Verified`,
  and the board/pane show it as `done`. Its dependents stop reporting it as a
  prerequisite that has not landed. `settle_completed_node` runs
  `verify::verify_all` on the completion path; pinned by
  `a_completed_child_settles_its_node_done_when_the_gates_pass`.
- [x] C2: a node whose child completed without verification is a named
  DECISION (the owner's D2), not a silent state: the sentence names the failing
  gates and the three ways out. Pinned by
  `a_completed_child_that_does_not_verify_becomes_a_named_decision`.
- [x] C3: as a run advances the board moves cards without a manual reload: a
  delegation reaching `done` rereads the execution view (D3 — the
  orchestrator's own report is the signal, not a poll). Pinned by the D3
  harness case.
- [x] C4: there is no poll at all. The reread rides the delegation event and
  is skipped when the board was never opened, so a settled board costs
  nothing.
- [x] C5: a test drives two nodes through a scripted child (dispatch, verify,
  verify) and asserts the document reports `in process` then `done` in that
  order, with the second node never showing `in process` before the first is
  `done` when it declares a prerequisite. Pinned by
  `two_nodes_move_through_the_order_a_prerequisite_imposes`, which asserts the
  graph's own `unlock` refuses the dependent while its prerequisite runs and
  offers it once it verifies. This needed `dispatch_node` and
  `mark_node_failed` to take their root as a parameter, like
  `settle_completed_node` — `set_current_dir` has no callers in this crate by
  design, so a fixture home was unreachable through the process cwd.

## Files to be touched

- `src/subagents.rs` — call the verification on the completion path, beside
  the existing `mark_node_failed`.
- `src/verify.rs` — the gate, if its signature needs a home rather than a
  `cwd`.
- `src/gui/bridge.rs` — nothing, unless the board document needs to report a
  run in flight so the face knows to poll.
- `src/gui/assets/app.js` — the bounded repaint while a run is in flight.
- `tests/face/harness.mjs` — C3's case.

## Decisions the owner holds

**Settled by the owner, 2026-11-18.**

- **D1 — what "verified" means: STRICT.** The node's declared gates pass
  against the artifact, through `verify::promote`/`verify_all` — not "the
  child said it completed". The graph's own law already says done is earned,
  never asserted.
- **D2 — an unverified completion is surfaced as a DECISION CARD.** The node
  does not silently stay `Running` (dependents would block forever) and is
  not silently marked `Failed` (dependents still block, because
  `remaining_at` skips only `Verified`). It becomes a card the owner decides
  on: absorb, respawn (bounded by `MAX_NODE_REDISPATCHES`), or re-plan.
- **D3 — the completion signal is the orchestrator's own.** The orchestration
  model already knows when a deliverable is completed; that signal is what
  updates the execution on the pane. Not a face-side poll of the graph, and
  not a second liveness clock: the run reports, the graph records, the pane
  reads.

## Out of scope

- Re-planning (`demote`) and the delta/replan machinery.
- What a plan's own acceptance criteria mean.
- The desk's end gate, which is a `deliverable`'s verification, not a node's.

## Verification

`cargo test -p fleet --lib`, `cargo clippy --all-targets`, `node
tests/face/harness.mjs`, and a live run of a two-node plan whose cards move
`queued → in process → done` while the pane is open.

## Linked artifacts

- `src/graph.rs` `dispatch`/`fail`/`mark_verified`/`demote` — the four
  transitions.
- `src/subagents.rs` `dispatch_node`/`mark_node_failed` — the two that are
  wired.
- `src/verify.rs` `promote` — the orphaned gate.
- `docs/plans/2026-11-18-the-board-at-execution-level.md` — the surface that
  reads these statuses.
