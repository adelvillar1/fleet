---
name: memory-rail-plan-and-the-substrate
description: "project 2026-10-30 — the memories revamp LANDED (737af35) — the rail, needs-you = memory::preview (the cycle's own filters), supersede and run-the-cycle in the pane, archive/today sections; categories from the substrate's OWN type words; measurement 7,207 px/339 controls → largest section 1,493 px/34"
type: project
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Project 2026-10-30 — "use the exact same pattern to achieve consistency on
how we manage memories" ([[skills-rail-plan-and-the-approve-defect]] is its
sibling). **LANDED 2026-10-30 as `737af35`** after the skills plan, plan
status landed, all ten criteria B1–B10 ticked with evidence.

**What landed:**

- **M0:** `memory::preview(root, today)` — consolidate's four filters
  (compress / supersede / untouched / kept) EXTRACTED into shared
  candidate helpers so preview and pass cannot drift, pinned by
  `the_preview_names_what_the_cycle_would_do` (four fixture shapes; the
  sets before the pass equal the actions after). Routes
  `POST /api/fact/supersede` (the click carries `approved: true` — the
  permission model; justification required by `Supersede::parse`) and
  `POST /api/memories/consolidate`; the read carries preview, archive
  (`load_archive` over `.archive/`), and each fact's `section`
  (`section_of`: type word with the KI-059 synonyms mapped,
  everything-else the fallback).
- **M1/M2:** 10-section rail (needs-you → user/feedback/project/
  reference/episodic/everything-else, empty types never render →
  retired/archive/today), `#/memory/<section>` remembered, one-line
  cfg-item rows with basis + recalls marks under the budget, the pane
  card = the unchanged in-place edit form PLUS the supersede form
  (justification required; replacement optional = fold, empty = forget).
- **Live proof:** the real corpus read matches the deep dive exactly —
  user 2, feedback 7, project 97, reference 1, episodic 2,
  everything-else 5 (the `state-sync`-typed facts), retired 34, archived
  13; the queue honestly EMPTY (every inferred fact recalled or
  confirmed).
- **Measurement:** 7,207 px / 339 controls on one board → largest
  section (project) 1,493 px / 34; 10 rail rows; one section-body at
  every address. Screenshots read as the review (tmp/shots-memory-*).
- **The build's own catch:** the dead-code sweep that retired the old
  board took `memHits` with it and the today section threw at runtime —
  restored, and the harness now OPENS EVERY SECTION so a missing builder
  cannot regress silently. The stub's selector engine also learned
  `[name="…"]` attributes (production code reads form fields that way).
- The two CSS lessons the probes forced:
  [[a-budget-grid-sizes-its-column-to-max-content]].

Substrate facts that stand: memory.rs 10,870 lines; shape axes 7 structure
+ 6 failure + 2 outcome; `Memory::supersede(request, approved, stream)` is
the permission model; the face reads the launch tree's corpus via
`resolve_root`.

**How to apply:** never copy `consolidate`'s filters — extract them; a
face click on a validated writer carries the approval flag; the third
surface proves the rail idiom generalizes — the fourth should reuse it
without asking.
