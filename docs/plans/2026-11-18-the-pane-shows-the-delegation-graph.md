---
status: completed
created: 2026-11-18
updated: 2026-11-18
slug: the-pane-shows-the-delegation-graph
---

# The pane shows the delegation graph

The owner's 2026-11-18 ruling, verbatim: "when a plan is being executed the
pane should also carry a tab that shows the delegation graph."

## Context

The delegation data already exists and is already read; it is just on the
wrong surface.

- **`/api/delegation`** (`server.rs:444`) → `bridge.delegation(session)` →
  `boards::read_delegation(home, session)`. The document carries `live` (the
  run in flight, as a `DelegationNode` tree), `history` (past runs), `graph`
  (the plan's declared dependency graph, flattened with edges) and `refused`.
- **`DelegationNode`** (`boards.rs:885`) is `{id, model, kind, depth, goal,
  status, stop_reason, turns, compactions, children}` — a tree with the
  parent ids already resolved.
- **`treeCard(root, title, note, cap, wide)`** (`app.js:6929`) already draws
  that tree as an SVG plot. It is a pure function of the node, so it can be
  called from any surface.

Two facts that decide the shape of the work:

1. **The route is already session-scoped.** `asked_session` reads
   `?session=` and falls back to `bridge.launch()`. The face's
   `readDelegation` omits the parameter and therefore always asks for the
   launch session — which is why the pane cannot simply reuse the call as
   written.
2. **The pane has no delegation tab.** Its tabs are context, detail, run,
   made here, files, preview, web. The one thing the owner asked to see
   while a plan executes is on a different lens three clicks away.

## Approach

Add a delegation tab to the pane, reading the same document the Delegation
lens reads, with the session named.

- `readDelegation(force, session)` takes the session it is asked about; the
  Delegation lens passes nothing (today's behaviour, the launch session), and
  the pane passes `face.session`.
- The pane's tab appears only when there is something to draw — a live run or
  a declared graph — following the `detail` tab's rule: a tab that leads to a
  blank is a door to nothing. The owner's phrasing is conditional too ("when
  a plan is being executed").
- It renders `treeCard(live, …)` when a run is in flight and `graphCard`/
  `graphTiles` for the declared graph, reusing both rather than writing a
  third drawing of the same facts.
- The tab is contextual, so `contextualPaneTab()` may consider it — but only
  below `detail`: an inspected object is more specific than the run's tree.

## Acceptance criteria

- [x] C1: while work is delegated, the pane carries a `work` tab that draws
  the live run's tree, each row carrying its status. Pinned by the TREE
  harness case.
- [x] C2: the tab reads the CURRENT conversation's delegation, not the launch
  session's — `readDelegation(force, session)` names it and
  `openPane` asks with `face.session`. Pinned by the TREE case's first
  assertion (`session=this-one` on the request).
- [x] C3: when no run is in flight and no graph is declared the tab is
  absent — `workTabAvailable` gates it, the same rule as `detail`.
- [x] C4: SUPERSEDED by the owner's D3 — the pane gets neither drawing.
  "Show just as a tree that shows the delegatable work status and depends on
  not visually fancy but effective for a pane on the drawer." `paintPaneWork`
  draws rows; the TREE case asserts the panel holds no `svg`.
- [x] C5: the tab follows a running plan without a manual reload — it
  repaints when the delegation read lands, which the existing `done`
  reread triggers (the node-completion plan's D3 signal). No poll of its
  own.

## Files to be touched

- `src/gui/assets/index.html` — the tab and its panel.
- `src/gui/assets/app.js` — `readDelegation`'s session parameter, the tab's
  gating and paint, `contextualPaneTab`'s order.
- `src/gui/assets/app.css` — the panel's inset if the pane needs one.
- `tests/face/harness.mjs` — C1/C2/C3.

Nothing in Rust: the read, the route and the drawing all exist. If C2 needs
the request to carry a session the route already accepts, that is a
JavaScript change only.

## Decisions the owner holds

**Settled by the owner, 2026-11-18.**

- **D1 — the CONVERSATION's tree, session-bound.** Not a plan-slug-keyed
  read. It covers the current plan being executed AND ongoing work that is
  not plan-related, because in both cases the delegation belongs to the
  session that spawned it. "That's the signal" — one read, one scope.
- **D2 — only the current work.** The live run's tree plus the graph of what
  is delegatable. Not `history`: past runs belong to the Delegation lens, and
  a pane is not the place for a scroll of them.
- **D3 — a tree, not a picture.** Rows of delegatable work with their status
  and their dependencies, sized for a pane. Effective over fancy: no SVG
  plot, no unfurling graph — `treeCard`'s chart is for the lens, and the pane
  gets a compact indented tree a few hundred pixels wide can actually hold.

## Out of scope

- Changing `read_delegation`'s tree-building or depth cap.
- The Delegation lens itself.
- Any new Rust read, unless D1's second option is chosen.

## Verification

`node tests/face/harness.mjs` for the three cases; `cargo test -p fleet
--lib` and `cargo clippy --all-targets` unchanged (no Rust edit); a live run
of a two-node plan with the pane open to confirm C1 and C5.

## Linked artifacts

- `src/boards.rs` `read_delegation` / `DelegationNode` — the read.
- `src/gui/server.rs:444` `/api/delegation` — the route, already
  session-scoped.
- `src/gui/assets/app.js:6929` `treeCard` — the drawing to reuse.
- `docs/plans/2026-11-18-the-board-at-execution-level.md` — the execution
  view this joins.
- `docs/plans/2026-11-18-a-plan-node-finishes-when-its-work-does.md` — the
  statuses the tree renders, and why they currently never advance.
