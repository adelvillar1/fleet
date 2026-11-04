---
name: designs-are-project-knowledge-plan-2026-11-01
description: "project 2026-11-01 — the designs-are-project-knowledge plan LANDED (223e4a5 + 67a4764) and merged to master at ecbf280: the binding field, the digest at the save funnel, the seed_task glob riding the KI-277 reconcile, lifecycle facts through the substrate's SUPERSESSION chain, the assembly's design section, the rail grouped by project; test seams keep fact/section writes on fixtures or they walk into the repo's real corpus"
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

2026-11-01, the owner's ruling — *"these design files belong to projects
so they must be treated as such and they need to be part of the
knowledge, memories, context, etc. even embeddings need to be taken into
account"* — planned then BUILT the same day on `design-parity`:
`223e4a5` (K1–K6) + `67a4764` (K2's rail grouping), completed `8bb7925`.

**What landed:**
- **K1** `canvas::Project.project: Option<String>` (serde-default,
  store stays home-flat — a move would migrate the sample board for no
  channel gain); `board-project` op sets/clears it; the head's picker.
- **K3** the digest regenerates inside `canvas::save` — the single
  write funnel — at `canvas/digests/<project-key>/<slug>.md`,
  deterministic (pages, sorted bare-name token union, review scores);
  a clear (or the delete op) removes the old key's file; unbound
  boards write none.
- **K4** `seed_task`'s sources gained
  `<FLEET_HOME>/canvas/digests/<project>/*.md`; **KI-277's
  `seed_missing` reconcile carries the new wording to already-seeded
  jobs** — no migration (pin over a pre-glob job). The parallel
  session rewrote seed_task to the `fleet mcp call` wording (KI-369)
  mid-build; the glob folded into THEIR wording, not over it.
- **K5** board create/rename/bind/delete each move one
  `design-board-<slug>` fact through `memory::Remember`: create
  remembers (ConfirmedBy), rename/bind supersede forward
  (versioned names, phrases slug+project-keyed so inheritance can't go
  stale), delete retires (the substrate's own chain — it REFUSES a
  silent overwrite with "the modify path is `supersede`"). A test seam
  (`TEST_BOARD_FACT_ROOT`) points ops at a fixture corpus: production
  resolution (home-first then cwd walk) under test would write the
  REPO's real memories (the pin's first pre-seam run leaked
  `memories/design-board-harbor.md` — removed).
- **K6** `canvas::design_context_section` + `prepare_memory` appends
  the bounded design section (cwd's final segment keys the digest
  dir); `Record::Memory` gained `#[serde(default)] design: Option<String>`
  (rule 5 — old logs replay None); a `TEST_DESIGN_CONTEXT` seam keeps
  test runs off the machine's canvas.
- **K2** `sectionRail` gained a header row; `paintDesignRail` groups
  boards by project (unbound last; a single unbound group shows no
  headers).

**Lessons:** the substrate's supersession chain is the ONLY modify
path for facts (design the lifecycle as a chain from the start);
phrases must be stable across renames (slug-keyed); any
memory-writing code path needs a root seam BEFORE the first pin run.
Both plans merged to master whole at `ecbf280` — see
[[design-parity-plan-2026-11-01]] for the merge folds. Related:
[[an-async-writer-makes-a-pin-race-itself]],
[[fleet-2026-11-01-corpus-refresh]].
