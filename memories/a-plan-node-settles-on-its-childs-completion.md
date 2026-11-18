---
name: a-plan-node-settles-on-its-childs-completion
description: "A plan node now settles when its child completes — verified strictly through the declared gates, with an unverified completion surfaced as a named owner decision instead of a node that silently blocks every dependent forever."
type: change
date: 2026-11-18
basis: confirmed-by
tier: durable
shape: [state-sync, drops-the-row, resolved]
phrases: [settle_completed_node, verify_all, remaining_at skips only Verified, the orchestration model already knows when a deliverable is completed, MAX_NODE_REDISPATCHES]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

The owner's 2026-11-18 rulings on a plan node's completion, implemented.

D1 STRICT: `verify::verify_all`'s declared gates run against the artifact. "The child said it completed" is not verification.
D2 DECISION CARD: an unverified completion is a named decision, not a silent state. Verified from the code why neither alternative works: `Graph::remaining_at` skips ONLY `Status::Verified`, so a node left `Running` blocks its dependents forever, and a node marked `Failed` blocks them just the same (it still carries its cost into the score). The returned sentence names the failing gates and the three ways out (absorb, respawn under `MAX_NODE_REDISPATCHES`, re-plan).
D3 ORCHESTRATOR SIGNAL: the completion report is the signal that updates the pane. No poll, no second liveness clock.

WHAT LANDED. `subagents::settle_completed_node(cwd, node)` runs on the `Outcome::Completed` branch of `run_subagent` — the branch that previously returned `None` and did nothing, which is why every node stayed `Running` forever. It loads the plan graph (generating from the declaration when no runtime file exists yet — `PlanGraph::scan`'s own order, or a settled node reads as "no plan carries it"), runs `verify_all`, saves, and returns one sentence. On the face, `applyDelegation` rereads the board when a delegation reaches `done` (`if (face.boardRead) readBoard(true)`), so the execution view moves off the event rather than a clock.

THREE TEST-SURFACE TRAPS, each cost real time:
1. `PlanGraph::load` fails when no runtime file exists (only `dispatch` writes one), so a settle test must `generate` first or the node reads as absent.
2. The nomenclature gate passes vacuously in a temp dir — it derives its vocabulary from the repo's own terminology docs. A test that needs a gate to FAIL must provoke one that does not depend on the tree: a CONTRADICTED DELTA fails the functional gate, and `Graph::record_delta` is how to land one.
3. A face harness case that pushes stream events needs a CONVERSATION route (`#/sessions/<id>`, like the queue's own case) — `#/chat` is the landing page with no conversation, so `apply()` paints nothing and every push is inert; and a delegation event needs `parent` set to the face's session, because the reducer passes a child only when this conversation raised it.

VERIFIED: 2357 Rust tests (3 new `settle_tests`), 1448 face checks across 202 cases (1 new D3 case), fmt and clippy clean. The D3 case was proved to bite: with the reread removed it fails with `before=1 after=1 rows=1 events=1` — the event lands, only the refresh is missing.

STILL OPEN: C5's two-node ORDER test (dispatch → settle → next), which needs a scripted child the current test surface does not expose. The plan stays `draft` for that reason.
