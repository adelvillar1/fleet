---
status: completed
created: 2026-11-01
updated: 2026-11-01
slug: designs-are-project-knowledge
---

# Plan: design boards are project knowledge — the binding, the digest, the corpus, the facts, the context

## Context

The owner's direction, 2026-11-01, before the design-parity build
started: *"because we are already working on project folders and repos,
these design files belong to projects so they must be treated as such
and they need to be part of the knowledge, memories, context, etc. even
embeddings need to be taken into account."*

Fleet already works in project folders — the tower reads git facts per
project under `~/Projects`, workspace-history resolves projects by path,
every session records the workspace it was born in, and the corpus-
refresh cron jobs embed each project's docs into its own blend1 vector
store. The design canvas is the one artifact that sits outside all of
it: boards live at `<FLEET_HOME>/canvas/<slug>/` with no project
binding, no fact, no digest, and no path into any embedding. A board
someone designs for cruiserintelligence is invisible to every knowledge
channel that project run uses.

This plan binds boards to projects and puts them on every channel the
owner named: **knowledge** (a digest per board, deterministic and
bounded), **memories** (a fact per board lifecycle, through the same
writer onboarding uses), **embeddings** (the per-project corpus-refresh
embeds the digests — the KI-277 reconcile carries it to existing jobs),
and **context** (a run born in a project workspace carries that
project's design summary in its assembled context).

## Decisions

1. **2026-11-01 — the store stays home-flat; the binding is a field.**
   Boards keep living at `<FLEET_HOME>/canvas/<slug>/` — moving the tree
   per project would migrate the bundled sample board and every owner
   board for no channel gain (the digest path is the only consumer that
   needs a per-project shape, and it gets its own directory). The board
   document gains `project` — the absolute path of the folder it belongs
   to, or none. The digest lands at
   `<FLEET_HOME>/canvas/digests/<project-key>/<slug>.md`, where the key
   is the path's final segment — the same name the corpus-refresh jobs
   and the blend1 vector stores already key by.
2. **2026-11-01 — the digest is the embedding unit, not the pages.**
   Raw page HTML is the artifact, not the knowledge; the digest is a
   bounded markdown account of the board — name, project, every page's
   name/size/model/set, the token summary, the latest review's scores,
   the brand, the counts. Deterministic: same document, same digest, so
   a refresh that re-embeds an unchanged board drops nothing.
3. **2026-11-01 — the fact rides the substrate's own writer.** Board
   create/rename/delete writes or retires one fact per board
   (`design-board-<slug>`) through `memory::Remember` — the same path
   `remember_onboarding` uses, `type: project`, phrases carrying the
   project name and the design words so the six-channel recall ranks it
   for design instructions. No second memory system.

## Approach

**Bind at the document, digest at the single write funnel, embed at the
cron's own task, fact at the lifecycle ops, context at the assembly.**
Five small seams, each reusing a standing mechanism:

- **K1 — the binding.** `canvas::Project` gains
  `project: Option<String>` (serde-default, camelCase-aliased like the
  rest). The writer gains a `board-project` op (set or clear); the
  design head shows the binding as a picker over the tower's known
  project folders (the same root the boards rail reads for git facts —
  bounded, newest first), and clearing it moves the board to unbound.
- **K2 — the rail groups.** `paintDesignRail` renders the boards in
  sections: one per project (name, bound count), then an `unbound`
  section; the fold and pick behavior of the shared `sectionRail` is
  untouched — the grouping is the data it renders, not a second rail
  idiom.
- **K3 — the digest.** `canvas::save` — the one funnel every write path
  already crosses — regenerates the digest for a bound board and
  removes the file for an unbound one; `delete_project` (the parity
  plan's P2, landing with it) removes the digest too. The digest
  writer lives in `canvas.rs` beside the store, pinned over a fixture
  board: the exact markdown for a known document, and the unbound case
  writing nothing.
- **K4 — the corpus.** `cron::seed_task(project)`'s sources gain
  `"<FLEET_HOME>/canvas/digests/{project}/*.md"`; `seed_missing`'s
  existing reconcile (KI-277) carries the new wording to every already
  seeded corpus-refresh job on the next seed pass. A live refresh of
  one bound project embeds the digest items — the item count moves, the
  store's file list names them.
- **K5 — the facts.** The board ops (create, `rename`, `board-project`,
  delete) write through `memory::Remember`: one `design-board-<slug>`
  fact holding the board's name, its project, its page count; rename
  rewrites it, delete marks it superseded (the substrate's own edge).
  The fact's `phrases` carry `<project> design`, the board name, and
  `board`/`page` words — the recall channel that ranks it is the one
  every task already queries.
- **K6 — the context.** The run path, whose session records the
  workspace it was born in, assembles a bounded design section when that
  workspace's project has boards: one line per board — name, page
  count, last moved — read from the digests (bounded, newest first, the
  inventory that `designer_task` already proved necessary, at
  assembly-time for every run instead of compose-time for design runs).
- **K7 — close-out.** The gate by exit code, the ledger checker green,
  the contract docs carrying the binding, the digest, the corpus glob
  and the fact lifecycle.

## Acceptance criteria

- [x] **K1 — every board carries its project.** The document read
      carries `project` (a path or none); the design head's picker sets
      and clears it through the writer; the choice persists across a
      reload (pinned on the wire and in the store).
- [x] **K2 — the rail groups by project.** The boards rail renders one
      section per bound project with its boards, then the unbound
      section; pick and fold behave as before (harness over a two-
      project fixture — two items, never one).
- [x] **K3 — every save writes the digest.** A bound board's save
      regenerates its digest at the project-keyed path — deterministic
      markdown naming every page, the tokens, the review scores and the
      brand; clearing the binding removes the file; an unbound board
      writes none (unit pins over a fixture document).
- [x] **K4 — the refresh embeds the digests.** `seed_task` names the
      digest glob; a seed pass reconciles an existing corpus-refresh
      job to the new wording (the KI-277 path, pinned); a live refresh
      of a bound project reports the digest among its embedded sources.
- [x] **K5 — the lifecycle writes a fact.** Creating, renaming and
      deleting a board each move one `design-board-<slug>` fact through
      the substrate's writer; `fleet memories --recall` over a design
      instruction surfaces it (pinned on a fixture home, never the
      owner's).
- [x] **K6 — the run carries the design summary.** A session born in a
      project workspace whose project has boards gets the bounded design
      section in its assembled context; one born where no boards exist
      gets none (assembly pin, both directions).
- [x] **K7 — close-out.** The full gate green by exit code, the ledger
      checker green over the real file, and the contract docs carrying
      the new surface (design.md's store section, the cron seed's task
      wording, the memory fact lifecycle, the interface contracts).

## Files to be touched

`src/canvas.rs` (the `project` field, the digest writer and remover,
their pins),
`src/gui/bridge.rs` (the `board-project` op, the lifecycle fact writes,
the delete-path digest cleanup),
`src/gui/assets/index.html` (the head's project picker),
`src/gui/assets/app.js` (the picker's paint/write, the rail's
grouping),
`src/gui/assets/app.css` (the picker and the section grouping),
`src/cron.rs` (`seed_task`'s digest glob, the reconcile pin),
`src/memory.rs` (the assembly's bounded design section, if the assembly
site is here — found at implementation, named in the close),
`docs/features/design.md`, `docs/features/cron.md` (the seed wording),
`docs/known-issues.md` (anything the gate finds, per hard rule 10).

## Out of scope

- **Moving the canvas store under per-project directories** — the
  binding is a field; the digest directory is the only per-project
  shape (decision 1).
- **Embedding raw page HTML or assets** — the digest is the embedding
  unit (decision 2); a page's full text can join its corpus through the
  project's own docs when it is written there.
- **Design facts in every project's own `memories/`** — the fact
  substrate is the corpus the agent recalls from (fleet's own, resolved
  the way onboarding resolves it); per-repo facts stay the repo's
  business.
- **A second rail idiom or a projects browse page** — the grouping is
  data in the one `sectionRail` the split pages share.
- **Auto-binding existing boards** — unbound boards stay unbound until
  the owner picks; the rail's unbound section keeps them visible.

## Verification

The gate's four commands by exit code at close-out; each criterion's
own pin — unit pins in `src/canvas.rs` (the digest's exact markdown,
the unbound no-write, the clear-removes), a `src/cron.rs` pin for the
reconcile, an assembly pin for the design section both ways, harness
checks for K1–K2 (the picker's write, the two-project rail), and the
live leg for K4 (a seed pass reconciling the real job, then one refresh
observed embedding the digest); the ledger checker over the real file
as the last act before the commit.

## Linked artifacts

- `docs/features/design.md` — the board's project binding, the digest,
  the rail grouping, the fact lifecycle
- `docs/features/cron.md` — the corpus-refresh task's new source glob
  and the reconcile
- `TECHNICAL-DOCUMENTATION.md` § Interface contracts — the
  `board-project` op and the digest path contract
- `FUNCTIONAL-SPECIFICATIONS.md` § The surfaces — the design page's
  binding picker and grouped rail

## Risks

- **The digest must not drift from the document.** It is regenerated at
  `save` — the one funnel — so no path can update the board without
  refreshing it; the determinism pin catches format drift that would
  churn the vector store on every refresh.
- **The assembly change rides every run.** The design section is
  bounded (a few lines, digest-read, absent when no boards exist) and
  its absence case is pinned beside its presence — a channel that
  silently grows is a channel the budget forgets.
- **The fact writer is a home write.** It is the substrate's own locked
  writer (the same one onboarding uses); the pins run on a fixture
  `FLEET_HOME`, never the owner's corpus.
- **The reconcile rewrites the owner's cron tasks.** It rewrites the
  task string and tools only (the KI-277 contract — schedules and
  enabled flags are the owner's), and the pin asserts exactly that
  boundary.

## Dependencies

- The landed kernel: the canvas store (`src/canvas.rs`), the writer
  ops (`bridge.rs`), the memory substrate (`src/memory.rs`), the cron
  seed (`src/cron.rs`).
- `2026-11-01-design-parity` (active, this branch): K3's delete-path
  cleanup rides the parity plan's `delete_project` (P2); K2's rail
  grouping and the parity plan's head work share `paintDesignRail`'s
  neighborhood — the parity phases resume after this plan lands, and
  the shared files fold deliberately.
- The KI-277 seed-reconcile and the KI-339 explicit-globs lessons —
  both already in the tree, both load-bearing here.

## Landed

Every criterion confirmed. The Rust block landed `223e4a5` (K1–K6:
the binding field and the `board-project` op, the digest at the save
funnel, the seed glob with the KI-277 reconcile, the lifecycle facts
through the substrate's own supersession chain, the assembly's design
section with the `Record::Memory` field that keeps it reconstructable)
and the rail's grouping `67a4764` (K2 — a header row in the shared
`sectionRail`, one family per bound project, unbound last). Each
criterion's pins are green: the digest's byte-determinism, the fact
chain's single live account, the reconcile boundary over a pre-glob
job, the context section both directions, the two-project rail fixture.
The isolation seams are load-bearing and named: the bridge's fact
writer and the agent's design section both point at fixture roots
under test, because any other resolution would adopt the machine's own
corpus (the KI-087 walk in reverse).

