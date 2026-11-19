---
name: the-contextual-pane-and-the-delegation-tree-landed
description: "All four 2026-11-18 plans are completed — the pane carries a session-bound delegation tree and is rooted at the conversation's own project folder, and a plan node settles strictly on its child's completion with one source of truth for which plan tree anything reads."
type: change
date: 2026-11-18
basis: confirmed-by
tier: durable
shape: [rendering, state-sync, resolved]
phrases: [readDelegation takes a session, workTabAvailable, paintPaneWork, effective_folder, bridge tree session, settle verifies only its own node]
missing: []
premises: []
recall-count: 2
superseded-by:
valid-until:
source:
---

The owner's 2026-11-18 rulings on the delegation tab and the contextual pane, implemented; all four 2026-11-18 plans are now `completed`.

DELEGATION TAB (D1 conversation tree / D2 current work only / D3 plain tree). `readDelegation(force, session)` now names the session — the route already honoured `?session=` and the face always omitted it, so every read asked for the launch session. A new `work` pane tab appears only while `workTabAvailable()` (a live run or a declared graph), painted by `paintPaneWork`: indented rows with a state mark, the run's name and status, and an "after <prereq>" line. It draws NO svg — the owner's D3 superseded the plan's original reuse of `treeCard` ("show just as a tree ... not visually fancy but effective for a pane on the drawer"), and the TREE harness case asserts the panel holds zero svgs.

CONTEXTUAL PANE (D1 the folder IS the project / D2 session-scoped artifacts / D3 one source of truth). `Bridge::tree` now takes the session and roots the read at `effective_folder` — the same resolution the state document hands the face as `folder` and the run uses as its working directory — instead of `self.cwd`, the launch directory. The route passes `asked_session`, and the face sends `&session=`. The project's name rides the FILES pane above the tree, not the head: the head already carries eight tabs at a 300px floor. `made here` needed no new store — `threadPaneRows` already read the session's own files, children, facts and tasks, which is exactly D2.

THE CWD THREADING that closed C5 also fixed two real defects: `dispatch_node` and `mark_node_failed` read `std::env::current_dir()` while `settle_completed_node` took a parameter, so a run launched from one directory and working in another dispatched against the WRONG tree; and both dispatch and settle skipped a plan whose runtime file did not exist yet — which is every plan's FIRST dispatch, since only `dispatch` writes that file. Both now generate from the declaration when the file is absent, `PlanGraph::scan`'s own order.

A REAL BUG THE C5 TEST CAUGHT: `settle_completed_node` originally called `verify::verify_all`, which sweeps the WHOLE graph — so one child's completion marked its SIBLINGS verified, and the two-node order test failed with ("verified","verified") where it expected ("verified","pending"). It now verifies only its own node (`gate.verify(node, dependents)` + `mark_graph.mark_verified`).

TEST TRAPS, all recorded because each cost time: a macOS temp path resolves `/var` to `/private/var`, so a root assertion must compare canonical paths; the face harness `page.IDS` has no `IDV`; a probe that edits a template literal must keep it syntactically valid or the whole bundle fails to parse (a syntax error reads as a failing case, which is not evidence about the feature); and restoring from a `/tmp` backup silently no-ops if the backup was already deleted — re-read the file to confirm the restore landed.

Gate: 2360 Rust tests (2 new tree tests, 5 settle tests), face harness 1457 checks across 204 cases (TREE and CTX new, both proved to bite), fmt and clippy clean, `plan_check.py` green.
