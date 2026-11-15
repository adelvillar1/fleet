---
title: the project-centered workspace
date: 2026-11-15
status: completed
---

# The project-centered workspace

The 0.8 rebuild (U44) collapsed the twenty-panel rail to seven lenses and
moved everything else to Settings → Advanced. The deeper audit found the
burial hid real capability: the owner could not find the second brain
("Health" was named only in the ninth settings section, and the palette
called it "second brain", so searching "health" returned nothing), and the
desk could not reach a project's design deliverables — the composer
promised "its sessions, its design, its facts" and the face broke the
promise. This plan brings the capability back **into the spine**: visible,
connected to work items, without returning to the dashboard.

## Acceptance criteria

- [x] C1: the desk reaches design deliverables. A design board is a
  deliverable — `bridge.projects()` and the spine carry canvas boards
  beside desk deliverables (namespaced `design:<slug>`, `source:design`),
  joined by the binding path. The desk's project column and shelf render
  them; they open the canvas, and a tissue design node pivots to it.
- [x] C2: the second brain is a visible Health lens. It leaves Advanced
  for the rail, the palette finds it by "health", spineCards ride above
  the atlas, and the `brainEvidence` name collision that emptied every
  attention row's evidence is fixed. The lanes' live probe (`/api/lanes`,
  `health_all()`) and the doctor's cheap checks (`/api/doctor`) reach the
  face beside the atlas; the inbox "open doctor" dead-end opens Health.
- [x] C3: routing and helpers weave into the lenses. Reviews gains a
  routing section (the live decision + tier chains, the Routing surface's
  own cards). The spine reads a delegated child's parent from the start
  record and emits a `delegates_to` edge. **Amended 2026-11-15 (U51):**
  the edge stays, but only as the parent goal's own process — the child
  rides as a node under its parent and never as a work item.
- [x] C4: the Advanced surfaces answer to their plain names. The palette
  search carries the doors' plain names — secrets→vault, assistants→bots,
  versions→workspace history, helpers→delegation, routing→router.
- [x] C5: harness pins for each cluster; the gate green; commits on
  master; local deploy. No release steps.

## Deliberate deferrals

- **The spine health aggregate** (a `health` key in `/api/workspace`):
  deferred. A live lane probe on every pivot would regress the spine read
  (the probe takes seconds; the spine is read on every pivot). Health
  probes on arrival instead — the right place for an expensive read.
- **The landing strips** (greet/stats/since/offers): stay retired. U49's
  "the landing is the question" ruling holds; the counts already live in
  Intelligence. Discoverability, not clutter.

## Delivery

2026-11-15. P1 `a1c5ff4` (design deliverables), P2 `c6f5c9b` (Health
lens), P3 `286b232` (routing + delegation lineage), P4 `a7c3a88` (plain-name
discoverability). U44 amended in [`UI-DECISIONS.md`](UI-DECISIONS.md).

## Verification

The face harness (174 cases) pins each cluster: the desk reaches a design
board and opens the canvas; the Health lens is visible and findable and
renders the lanes and doctor; the Reviews rail carries routing; the
Advanced surfaces answer to their plain names. A spine test pins the
delegation-parent extraction. The gate green throughout (fmt / clippy /
build / test).