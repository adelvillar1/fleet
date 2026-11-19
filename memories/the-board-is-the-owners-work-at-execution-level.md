---
name: the-board-is-the-owners-work-at-execution-level
description: "The board is now an execution view — one card per approved plan listing its atomic declarable tasks in queued/in-process/done columns with the persona each is assigned to, a drawer of full context and dependencies by state, and provenance derived so nothing not the owner's can appear."
type: change
date: 2026-11-18
basis: confirmed-by
tier: durable
shape: [rendering, drops-the-row, resolved]
phrases: [the board should show queued work that's pending, atomic delegatable tasks and to which workload/persona, the board shouldn't show any tasks that are not user initiated, board_read, plans_read, DeclaredNode.assignee]
missing: []
premises: []
recall-count: 4
superseded-by:
valid-until:
source:
---

The owner's 2026-11-18 ruling, implemented per `docs/plans/2026-11-18-the-board-at-execution-level.md` (status: active, all six criteria ticked with evidence).

THE MODEL. A plan's fenced ```graph declaration is the board's source. Only `PlanStatus::Active` generates a graph (`generates_a_graph()` matches `Active` alone — `draft` AND `completed` generate nothing), so "a card's existence IS its provenance" is not a heuristic: a cron run, a delegated child, an eval run or a probe never writes a plan, so none can reach the board. That is C5, derived, never hand-marked.

THE DOCUMENT. `board_read(home, root)` in src/gui/bridge.rs returns `{type:"board", counts:{queued,in-process,done,plans}, plans:[...], tasks:[ad-hoc], refused:[]}`. Each plan card carries `{id, plan, status, updated, workload, queued, tasks}`; each task carries `{id, title(approach), column, state, status, cost, assignee, approach, serves, capabilities, paths, depends_on:[{id,state}], waiting_on, reason}`. `plans_read` walks `docs/plans/*.md`, skips anything `generates_a_graph` refuses, loads the runtime file when one has landed and falls back to the declaration otherwise. `ad_hoc_tasks` reads `workspace::read(home)` separately — a plan node and a standalone task are different objects.

THE ASSIGNEE (the one real data gap). `Node.assignee`, `NodeFile.assignee` and `DeclaredNode.assignee` are all new, all `#[serde(default)]` so pre-assignee runtime files read back unassigned instead of refusing. `plan_graph::generate` carries it through. `board_read` defaults an unassigned node to the plan's own `execution:` frontmatter verdict (`single`/`mixture`/`swarm`) — the plan's own declared workload — rather than inventing an owner. `docs/plans/README.md` now documents the whole declaration format, which it had never done.

THE UI. `paintBoard` was replaced wholesale: one `planCard` per plan, three `BOARD_STATES` columns (queued / in-process / done), each task a `taskRow` — the press opens `openTaskDrawer` with approach, who-it-is-for, the criterion it serves, size, capabilities, paths, and dependencies grouped by state. The rail was repointed from project scope to PLAN scope (`boardSections`). Removed with it: `boardSelected` (dead), the drag/move/bulk callers. `boardCardTask` STAYS because the desk's `paintDeskWork` still uses it.

A TRAP WORTH KEEPING: `self.cwd` in the bridge is the PROCESS directory, not a test home, so a fixture home cannot drive `board_read` through the live bridge — the fixture must be exercised by calling `board_read(&home, &root)` directly (that is what the four new Rust tests do), and the live check must be run from inside the fixture root.

HARNESS: `/api/board` had no stub, so every unlisted route fell to `opts.reply || {}` and zero rows rendered — the same gap `/api/doctor` had earlier. Three old cases pinned the board I replaced (project-scoped rail, drag-and-settle moves, spine-fed board); all three were REWRITTEN to the new truth rather than deleted, and the S6 spine claim was repointed to the inbox, which still reads the spine.

Gate: 2354 Rust tests (4 new), face harness 1435 checks across 200 cases (1 new case), fmt and clippy clean. Checks proved to bite: removing the drawer press fails three drawer assertions; dumping every task on the queued column fails both column assertions.
