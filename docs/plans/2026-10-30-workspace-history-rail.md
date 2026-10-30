---
status: active
created: 2026-10-30
updated: 2026-10-30
slug: workspace-history-rail
---

# Plan: the workspace-history revamp — the project rail, the visual components, and the files and store in the pane

## Context

The workspace-history surface is the knowledge group's third long page,
and the only one still showing a single view: the launch folder's own
registration, five cards stacked on one scroll — the tree, the work, the
calls, the files, and the store — **2,384 px tall with 30 controls**,
measured today at 1440×900 on the face as it serves. The substrate
already knows better: the machine has **six registered folders** (fleet,
zcode-router-kit, and four LayerFS test fixtures), and
`layerfs_view::workspace_history(&cwd, …)` resolves the project from the
launch directory alone — every other project's history is unreachable
from this surface. The deep dive (2026-10-30) established what the
revamp stands on:

- **The registrations are enumerable.** `layerfs_tools::registrations()`
  reads `<FLEET_HOME>/config.json`'s `layerfs.projects` map — folder →
  `{name, store}` — cached once per process.
- **The activity measure already rides the read.** Every `CommitView`
  carries `time` — wall-clock millis — so a project's newest commit is
  the read's own data; no second source, no mtime games (a SQLite read
  can touch a store's WAL; the commit time cannot lie about activity).
- **One project's read is warm and bounded** — 56–165 ms for the whole
  tree, and the `since` version answers `unchanged` in 1 ms. The rail
  document can afford one read per registered project, cached.
- **The visual components are the point of the page** — the tree (bands,
  lanes, fork nodes, graft stems), the work join, the calls feed — and
  they stay the board's content. What moves is the two cards that are
  *browsing and accounting*, not viewing: the files and the store, into
  the right pane, the object-card idiom the settings/skills/memory
  revamps landed.

The owner's direction, 2026-10-30: the same rail pattern, one section
per project — **only projects with activity in the past six weeks** —
the visual components on the board, and "the files" and "the store"
cards on the right side.

## Decisions (the owner's, dated and quoted)

1. **2026-10-30 — the pattern.** *"let's use the same pattern for
   workspace history … if we use the rail pattern, we can display the
   workspace histories for the different projects"* — the shared
   `sectionRail`, one section at a time, one address per section.
2. **2026-10-30 — the six-week window.** *"we would only need to show
   the history for the projects that have had activity in the past 6
   weeks"* — a fixed window, the project's newest commit inside it.
3. **2026-10-30 — the composition.** *"we don't have line items but the
   visual components, on the right side rail we can show 'the files' and
   'the store' cards"* — the board keeps the visual components; the
   browsing and accounting cards move into the pane.

## Approach

**The rail is the registrations, filtered.** One section per registered
folder whose newest commit falls inside the six-week window, ordered by
newest activity first, named by the registration's `name`, badge = its
branch count. A project with no recent activity is simply absent — the
filter is the rule, not a hidden row. The rail document
(`workspace_history` gains the enumeration) reads every registered
project's history once, caches the answer server-side for thirty seconds
(the home document's own pattern), and carries the per-project summaries:
name, folder, branches, commits, layers, the storage figures, the newest
commit time, and the active flag. The existing single-project read
becomes `workspace_history_for(folder)` — the same function, the folder
made explicit — and the section's full view arrives through it.

**The board keeps the visual components.** The strip (the selected
project's branches/commits/layers/MB/dedupe), the tree, and the work —
one project's components at a time, the height problem solved the same
way skills and memory solved it. The calls card stays on the board and
stays machine-wide: the substrate's trace feed is not project-scoped, and
the card already names traces-off rather than reading it as no activity.

**The pane carries the two cards.** A section's press opens the right
pane's object card with two sections — **the store** (the nine figures,
the Init record, the per-commit costs) and **the files** (the ref's
tree, the path, and a file's instruments: text, blame, revisions, diff)
— the same `openDrawer` idiom the fact and skill cards use, tabs stood
down. The tree's node click, which today "points the Files pane at that
ref", lands in the pane's files card at that ref instead — one card, one
ref, one press away from the tree.

High-level steps:

1. **W0 — the rail document.** The enumeration, the six-week filter, the
   per-project summaries, the thirty-second cache, and the explicit
   folder in the single-project read.
2. **W1 — the rail and the sections.** The stage-split markup, the
   shared `sectionRail`, the hash route `#/workspace-history/<project>`,
   the section painter over the tree and the work.
3. **W2 — the two cards in the pane.** The store and the files move
   into the object card; the tree click lands there at the ref; the
   board drops the two cards.
4. **W3 — every interaction proven.** A harness case over a fixture with
   two active projects and one outside the window; the tree click; the
   files tabs and instruments; the store figures; the measurement and
   the screenshots.
5. **W4 — the docs and the gate.**

## Increments

Each lands whole — code, checks, docs — and each is green on its own.

### W0 — the rail document

`layerfs_view::workspace_history` enumerates `layerfs_tools::registrations()`
and, for each folder, runs the existing history read (the folder made
explicit through a new `workspace_history_for(folder, since)` seam, the
current `cwd` path delegating to it). The document carries a `projects`
list — name, folder, branches, commits, layers, storage figures, the
newest commit's `time`, and `active` (newest time inside
`now − 6 weeks`) — with only the active projects' full views attached;
an inactive project's summary rides without its view. The bridge caches
the document for thirty seconds (the home read's own pattern), and
`GET /api/workspace-history` answers the rail document; the section's
full view arrives through the same route with a project parameter.

*Checks:* H1.

### W1 — the rail and the sections

`index.html`'s workspace-history stage gains the rail nav (stage-split);
`paintWorkspaceHistory` becomes the rail painter plus one section painter
— the strip, the tree, and the work for the selected project; the hash
route `#/workspace-history/<name>` addresses it, the rail remembers it,
and the calls card follows the section (machine-wide, as it is).

*Checks:* H2, H3, H4.

### W2 — the two cards in the pane

`historyFiles` and `historyStorage` move from the board into the pane's
object card as its two sections; the tree's node click opens that card
at the ref (the existing `historyRef` state, one card instead of two
panes); the board renders only the strip, the tree, and the work.

*Checks:* H5, H6.

### W3 — every interaction proven, the measurement, the docs

A harness case: two registered projects with activity (rail sections,
one section at a time, the route, the pane card's two sections, the tree
click landing the ref, the files tabs, the store figures) and one
registered project whose newest commit sits outside the window (absent
from the rail). The probe's before/after measurement, the screenshots
read as the review, and the docs.

*Checks:* H7, H8, H9.

## Use cases

- [ ] A: The owner reaches any project with recent activity through the rail instead of one launch-folder view.
- [ ] B: A project's tree, work, and calls render for the project whose section is open — one section's components at a time.
- [ ] C: The files and the store live in the pane, one press from the project, with the tree's click landing there at the ref.
- [ ] D: A project without six weeks of activity is absent from the rail, not a dead row.

## Acceptance criteria

- [x] **H1 — the rail document filters honestly.** The read carries every
      registered folder's summary with its newest commit time and its
      active flag (newest inside `now − 6 weeks`), attaches full views
      only for active projects, and a fixture with a project outside the
      window pins its absence; the document caches for thirty seconds.
- [x] **H2 — the rail is the shared helper.** The panel renders
      `sectionRail` (store `fleet-workspace-rail`), collapses to icons,
      survives a reload, marks `aria-current`, takes the arrow keys, and
      only the open section's cards exist in the DOM.
- [x] **H3 — a section is an address.** `#/workspace-history/<name>`
      opens it; bare `#/workspace-history` resumes the remembered one;
      an unknown segment falls back to the newest-active project.
- [x] **H4 — one section's components at a time.** The board holds only
      the selected project's strip, tree, and work; the calls card stays
      machine-wide and names traces-off when traces are off.
- [x] **H5 — the pane carries the store card.** The store's nine
      figures, the Init record (or its absence named), and the
      per-commit costs render in the pane's object card for the
      selected project.
- [x] **H6 — the pane carries the files card, and the tree lands
      there.** The files card (the ref's tree, the path, and a file's
      instruments — text, blame, revisions, diff) is the pane card's
      second section; the tree's node click opens the card at that ref.
- [x] **H7 — every interaction works.** A harness check per interaction —
      the section pick, the route, the refresh, the files tabs, the
      diff, the tree click's landing, the store figures — each asserted
      against its named element and its observed effect, in the same
      commit as the element.
- [x] **H8 — the filter is the owner's window.** A project whose newest
      commit sits outside six weeks does not appear in the rail; one
      inside does; the window is fixed and named in the code's comment.
- [x] **H9 — the gate and the measurement.** fmt, clippy, build, test
      green by exit code; the probe quotes the before (2,384 px / 30
      controls / five cards) against the after (one project's components
      in the board, the two cards in the pane); the screenshots read as
      the review.
- [x] **H10 — the docs.** The feature doc, the technical doc's module
      entry, and the functional spec's passage carry the rail, the
      filter, and the pane composition.

## Landed 2026-10-30

H1: `the_rail_document_filters_to_the_six_week_window` — two active
projects carry their views newest-first, the 50-day project is absent,
the dead read names the substrate's own refusal, and the imported-store
case (no commits, the runtime record's `created` inside the window) is
active — the measure is two-sided because the fleet store restarted
empty when its volume was re-created, and the import IS its origin; the
store file's birthtime cannot serve through a Docker volume, so the
substrate's own runtime record is the source. H2–H8: the face harness,
**458 checks across 48 cases** — the C5-family instrument case now
drives the pane (the store and the files cards render there, the file
and blame instruments re-render the open card when they land), and the
WORKSPACE case pins the rail's sections newest-first, one section body
at a time, the encoded-path route, the press opening the pane card (the
store and the files), closing, and the tree click's landing. H9: the
gate whole (fmt 0, clippy 0, build 0, the suite green — quoted in the
commit) and the probe's measurement: before, one board **2,384 px / 30
controls / five cards**, one project only; after, the section at
**1,307 px / 5 controls** with the store and files in the pane, the
rail carrying fleet (the five dead test fixtures named on one compact
card — the Docker reset took their volumes). The screenshots are read
as the review (tmp/shots-workspace-after). Two defects the build caught:
the object card's overflow used to turn `.face` into the scroll trap the
drawer's scrollIntoView dragged — the top bar left the viewport when a
memory's card opened (owner-reported; fixed in `9fe5f60`, every surface
benefits) — and the dead fixtures' refusals rendered as five unbounded
script-output walls above the section; one card, lines clipped.

## Files to be touched

**Backend / API:**
- `src/layerfs_view.rs` — `workspace_history_for(folder, since)`; the
  enumeration, the six-week filter, the per-project summaries
- `src/gui/bridge.rs` — the rail document with its thirty-second cache
- `src/gui/server.rs` — the project parameter on the route

**Frontend:**
- `src/gui/assets/index.html` — the stage's rail nav
- `src/gui/assets/app.js` — the rail painter, the section painter, the
  two cards in the pane, the tree click's landing

**Tests:**
- `tests/face/harness.mjs` — the project-fixture case (two active, one
  outside the window), the pane cards, the tree click

**Documentation (see Linked artifacts below):**
- `docs/features/workspace-history.md` — the section rewritten
- `TECHNICAL-DOCUMENTATION.md` — the view/bridge entries
- `FUNCTIONAL-SPECIFICATIONS.md` — the workspace-history passage

## Out of scope

- **Writes** — the surface stays read-only; commits, merges, and
  publishes remain the delegated child's, behind the arm.
- **The delegation rows' click-through** — unchanged; it still points at
  the branch's tree.
- **The TUI's board pane** — unchanged; this plan is the face.
- **A store-mtime pre-gate** — the rail reads every registered project's
  history (six today, cached); a cheaper first gate is a named follow-up
  if the registration count grows.
- **A configurable window** — six weeks is the owner's rule, fixed and
  named.

## Verification

**H1** — the fixture pin: two registered folders, one whose newest
commit is 50 days old:
```bash
cargo test the_rail_document_filters_to_the_six_week_window
```
Expected: the old project's summary carries `active: false` and no view;
the active one carries its view; the cache returns the same document
within thirty seconds.

**H7–H9** — the face harness (the workspace fixture gains a second
project and the stale one):
```bash
node tests/face/harness.mjs
```
Expected: every new check green; the count line quoted in the commit.
The probe at 1440×900 quotes the before/after; the screenshots are read.

## Linked artifacts

- `docs/features/workspace-history.md` — "The section": the project
  rail, the six-week filter, the pane's two cards, the tree's landing
- `TECHNICAL-DOCUMENTATION.md` section "Module reference" — the view and
  bridge entries gain the enumeration and the per-project read
- `FUNCTIONAL-SPECIFICATIONS.md` section "The surfaces" — the
  workspace-history passage: projects, the window, the pane

## Risks

- **One read per registered project per thirty seconds.** Six today
  (four of them LayerFS test fixtures — the filter shows them while
  they are active, which is the rule working); the cache bounds the cost,
  and the named pre-gate follows if the count grows.
- **The files card in the pane is interactive state.** Its tabs and
  path live in `face.history*` today and continue to; the drawer's
  re-render keeps the same state — the harness pins the tabs so a
  regression cannot hide.
- **A project's read can die** (worker down, store volume gone). The
  existing named refusals ("the LayerFS worker is not running", "the
  project's LayerFS Store volume is missing") ride on the section's
  document — the rail's summary for that project carries the same named
  reason rather than an empty row.

## Dependencies

- The settings revamp (`2026-10-29-settings-rail.md`, completed) — the
  shared `sectionRail`, the object-card idiom.
- The history instrument (`2026-10-26-layerfs-history-instrument.md`,
  completed) — the one read, the vocabulary, and the tree the sections
  render.

## Notes

- The machine's live registrations, measured 2026-10-30: `fleet`
  (the launch folder), `zcode-router-kit`, and four LayerFS test
  fixtures (`fresh-src`, `src`, `src-2`, `src-3`) under `tmp/` — all six
  qualify today (their stores were written days ago), and the filter's
  real work begins as older projects accumulate or their stores die.
- The activity measure is the read's newest `CommitView.time` — the
  same data the tree already draws — not a store mtime: a SQLite read
  can touch a store's WAL and mark a dead project alive, while the
  commit time only moves when a commit happens.
- The before-state was measured on the face as it serves (the memory
  build kept this panel unchanged): **2,384 px, 30 controls, five
  cards**, one view, the launch folder's own project only.
