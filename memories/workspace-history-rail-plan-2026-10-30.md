---
name: workspace-history-rail-plan-2026-10-30
description: "project 2026-10-30 — the workspace-history revamp LANDED (2cbc626): the project rail with the six-week window, files+store in the pane, path-based project resolution (the owner's steer), the runtime-record `created` as the never-committed store's activity; 2,384px/30ctl → 1,307px/5ctl; harness 458/48, suite 1,820"
type: project
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Owner direction 2026-10-30: the same rail pattern for workspace history,
the fourth knowledge-group surface — *"we can display the workspace
histories for the different projects… only… projects that have had
activity in the past 6 weeks"* — and *"we don't have line items but the
visual components, on the right side rail we can show 'the files' and
'the store' cards"*. **LANDED whole** as `2cbc626`, all ten criteria
H1–H10 ticked with evidence in the plan file.

**What landed:**

- **The rail document** (`workspace_projects` in layerfs_view.rs): every
  registered folder read once, cached 30 s in the bridge; one project's
  full view answers by its registered folder's PATH
  (`workspace_history_for_registered`) — the owner's mid-plan steer:
  *"the control tower and second brain use paths to resolve projects"* —
  not by name. Route `#/workspace-history/<encoded path>`.
- **The six-week measure is two-sided:** the newest `CommitView.time`,
  and — for a store that has never committed — the substrate's own
  runtime record (`<FLEET_HOME>/layerfs-home/.runtime/projects/<name>.json`
  carries `created`; new reader `layerfs_tools::runtime_created`). The
  fleet store restarted EMPTY when its volume was re-created, so
  commits-only would hide the owner's own project; the store file's
  birthtime CANNOT serve because the store path is a Docker-internal
  volume the host cannot stat. Pinned by
  `the_rail_document_filters_to_the_six_week_window` (active pair +
  imported-store case + 50-day-absent + dead-read-named).
- **The board** keeps the visual components (strip + tree + work, one
  project at a time) with the calls card machine-wide; **the pane** holds
  one object card with two sections — the store (nine figures, Init,
  per-commit costs) and the files (ref tree, path, text/blame/revisions/
  diff) — and the tree's node click lands there at the ref. The file and
  blame instrument reads re-render the OPEN card when they land (a drawer
  rendered before its read shows "reading…" forever otherwise).
- **Dead reads are named, compactly:** the five dead LayerFS test
  fixtures (the Docker reset took their volumes) render as ONE card
  after the section, lines clipped at the substrate's first sentence —
  the first cut rendered five unbounded script-output walls.
- **Measurement:** 2,384 px / 30 controls / five cards on one board →
  1,307 px / 5 controls for the section with the two cards in the pane.
  Harness 458 checks / 48 cases (the C5-family instrument case now
  drives the pane; new WORKSPACE case); suite 1,820 green.

**How to apply:** when a surface's reads resolve their object from cwd,
make the object an explicit parameter before applying the rail pattern;
resolve projects by path (the house identity); a six-week window is
measured from data the read carries plus the substrate's own records —
never file mtimes (WAL touches) or host stat on Docker-internal paths;
an instrument card rendered inside a drawer must re-render when its
async read lands.

Related: [[skills-rail-plan-and-the-approve-defect]],
[[memory-rail-plan-and-the-substrate]] (the pattern's other two
landings), [[an-overflowing-card-turns-the-frame-into-the-scroller]]
(the pane defect the build caught), [[layerfs-history-instrument-2026-10-26]].
