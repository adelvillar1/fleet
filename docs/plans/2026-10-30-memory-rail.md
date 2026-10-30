---
status: completed
created: 2026-10-30
updated: 2026-10-30
slug: memory-rail
---

# Plan: the memories revamp — the rail, the type categories, and the management verbs in the pane

## Context

The memories surface is the knowledge group's second long page, built in
the same sitting as skills': the corpus card (search, four filter chips,
**every row at once — no budget**, 114 files today), then two cards beside
it (today's recall, the corpus mix). The deep dive (2026-10-30) measured
what the surface manages:

- **The corpus is 114 live facts** — 97 `project`, 7 `feedback`,
  5 `state-sync`, 2 `user`, 2 `episodic`, 1 `reference` — plus **34
  retired** still in the tree and **13 archived** by the cycle. The
  substrate's own declared vocabularies are the categories: `type` (the
  four canonical words plus the documented synonyms), `tier`
  (working/durable/episodic), `basis` (confirmed-by/inferred), and the
  shape axes (7 structure + 6 failure + 2 outcome). Unlike skills, **no
  new table is needed** — the taxonomy is already declared in the
  substrate; the 5 `state-sync`-typed facts (a shape tag written into the
  type field) are exactly what an **everything else** section makes
  visible.
- **The face can edit but not manage.** The one write route is `POST
  /api/fact` (the substrate's own field rewriter — sound, pinned by
  tests). What the face cannot do today, though the CLI and the model's
  tools can: **create** a fact (`Memory::remember`), **supersede** one
  (the modify/forget path, `Memory::supersede` — whose `approved` flag is
  the permission model: a `confirmed-by` fact refuses without it), **run
  the consolidation cycle** (`consolidate()`), or **see the archive**
  (`render_archive`, CLI-only). Managing memories means exactly these
  verbs, and the rail pattern is what gives each a place to live.
- **The cycle's queue is already computed** — `Consolidation`'s own
  filters name the compress set (working facts past `valid-until` or
  `WORKING_MAX_AGE_DAYS`), the supersede set (durable, inferred,
  never-recalled, past `STALE_AFTER_DAYS`), the **untouched** list
  (confirmed-by in the same shape — named, left alone), and the **kept**
  list (incomplete facts — never folded, their missing companions are the
  point). A preview over the same filters is the **needs you** section,
  with no new rules.

The owner's direction, 2026-10-30: *"use the exact same pattern to
achieve consistency on how we manage memories"* — the skills/settings/
bots shape: the shared `sectionRail`, one section at a time, one-line
rows, the right pane for details and now for the management verbs.

## Decisions (the owner's, dated and quoted)

1. **2026-10-30 — the pattern.** *"use the exact same pattern to achieve
   consistency on how we manage memories"* — rail → one section →
   one-line rows → the item in the pane; the section set comes from the
   substrate's own declared words, not a new table.
2. **2026-10-30 — management, not just reading.** The verbs the pane
   gains are the ones the substrate already validates: supersede (the
   click is the `approved` flag the permission model requires), run the
   cycle, and the archive read. Nothing new is invented; every route is a
   thin wrapper over an existing function.

## Approach

**Sections from the substrate's own vocabulary.** The type axis is the
category set — `user`, `feedback`, `project`, `reference`, `episodic`,
and **everything else** for the free-form types the substrate
deliberately permits (the protected synonyms map onto user/feedback
before the section is chosen, so a `preference` fact sits with feedback
where the protection already put it). Beside the categories: **needs
you** first (the cycle's preview + the exceptions), then **retired**
(in-tree, superseded or expired), **archive** (what the cycle forgot,
read-only, collision serials named), and **today's recall** (the hits
card, promoted). Shapes ride rows and the pane as marks — the recall
axis, not the management axis; the CLI keeps `--shape`.

**The verbs arrive as thin routes over the validated writers.** `POST
/api/fact/supersede` wraps `Supersede::parse` + `Memory::supersede` with
`approved: true` — the owner's click **is** the approval, the same
semantics the skills approve rides, and the justification field is
required by the parser either way. `POST /api/memories/consolidate` wraps
`consolidate()` (rows land on the calibration stream exactly as the
scheduled pass writes them). The **needs you** preview is a new
`preview` function beside `consolidate` reusing its four filters verbatim
— compute, write nothing — so the section's counts can never disagree
with what the cycle would actually do.

The surface consumes the same shared declarations as skills and settings:
`sectionRail()` (store `fleet-memory-rail`, hash `#/memory/<section>`),
the `cfgItem` one-line row, `budgetRows`, and the pane card
(`openDrawer`, tabs stood down). The existing fact editor moves into the
pane unchanged — it was the pattern's prototype. Search stays at the
surface head across sections; the live/protected/retired filter chips
retire into the sections themselves (protected is a mark on the row; the
retired section replaces the chip).

High-level steps:

1. **M0 — the management routes and the preview.** `memory::preview`
   (the cycle's filters, no writes), the supersede route, the
   run-the-cycle route, the archive list in the read.
2. **M1 — the rail and the sections.** The memory stage gains the rail
   nav; `paintMemory` becomes one section painter; the hash route.
3. **M2 — the rows and the pane.** One-line rows with budget and marks;
   the pane carries the body, the existing editor, related, and the new
   verbs (supersede with its justification, run the cycle from needs
   you).
4. **M3 — every button proven, the measurement, the docs.**

## Increments

Each lands whole — code, checks, docs — and each is green on its own.

### M0 — the management routes

`memory::preview(root, today)` computes the four sets (compress,
supersede, untouched, kept) and nothing else — the same filters
`consolidate` runs, extracted beside it so one change moves both. The
read (`boards::read_memory` → the face document) gains the preview, the
archive list (`load_facts_archived` over `.archive/`), and each fact's
section word. The two POST routes land in `server.rs` + thin
`bridge::fact_supersede` / `bridge::memories_consolidate` handlers over
the existing functions, refusals typed (already-superseded, already-
expired, needs-justification) exactly as the writers name them.

*Checks:* B1, B2.

### M1 — the rail and the sections

`index.html`'s memory stage gains `<nav class="section-rail"
id="memory-rail">`; `paintMemory` becomes the rail painter plus one
section painter; `#/memory/<section>` addresses a section, bare
`#/memory` resumes the remembered one; needs you leads; the type
categories follow in the substrate's own prominence order (user,
feedback, project, reference, episodic), everything else after them;
retired, archive, and today's recall close the rail.

*Checks:* B3, B4.

### M2 — the rows and the pane

Category sections render one-line `cfgItem` rows (name, description
value, type/basis marks, recalls, protected/retired state) under a
30-row budget with the count named; search stays at the head. The pane
card keeps the fact editor as it is (the rewriter's fields, the name/
tier/basis rule stated) and gains **supersede** — justification
required, replacement fields optional and inheriting per the field
rules — refused in place with the writer's own words. The needs-you
section's rows offer **run the cycle now** (one press, the pass lands,
the section refreshes); its untouched and kept rows say why they were
left.

*Checks:* B5, B6, B7.

### M3 — every button proven, the measurement, the docs

A harness case per action button (edit/save, supersede, run the cycle,
ask, copy, file, search, budget, section pick) asserted against its
named element; a live browser pass (edit one fact, supersede one
scratch fact, run one cycle in a scratch home); the Chromium probe
quoting before/after heights and control counts; the docs.

*Checks:* B8, B9, B10.

## Use cases

- [ ] A: The owner reaches any fact through its type's section instead of one unbudgeted list of 114 rows.
- [ ] B: A fact is superseded from the pane, with the justification the permission model requires — today that path is CLI-only.
- [ ] C: The consolidation cycle's queue is visible before it runs, and runnable from the section that names it.
- [ ] D: A retired fact, an archived fact, and a free-form-typed fact each have a place that says what they are.

## Acceptance criteria

- [x] **B1 — the preview is the cycle's own filters.** `memory::preview`
      returns the same four sets `consolidate` would act on, writes
      nothing, and a unit test pins preview ∘ consolidate agreement on a
      written fixture (the sets before the pass equal the actions after
      it).
- [x] **B2 — the routes are the writers.** `POST /api/fact/supersede`
      lands a supersession through `Memory::supersede` (the click carries
      `approved: true`; a confirmed-by fact supersedes, an unnamed
      justification is refused by the parser) and `POST
      /api/memories/consolidate` lands one pass whose calibration rows
      match the scheduled pass's; every refusal is the writer's own typed
      reason.
- [x] **B3 — the rail is the shared helper.** The memory panel renders
      `sectionRail` (store `fleet-memory-rail`), collapses to icons,
      survives a reload, marks `aria-current`, takes the arrow keys, and
      only the open section's cards and rows exist in the DOM.
- [x] **B4 — a section is an address.** `#/memory/<section>` opens it;
      bare `#/memory` resumes the remembered one; an unknown segment
      falls back to `needs you`.
- [x] **B5 — the rows are one line with a budget.** A section's rows are
      `cfgItem` one-liners carrying type, basis, recalls, and the
      protected/retired marks; the first 30 show and the press names the
      rest; the search box reads every field across every section.
- [x] **B6 — the pane is the fact.** The row's press opens the card with
      the body, the existing edit form (unchanged fields, the rewriter
      untouched), related, and the actions; a save refreshes the row.
- [x] **B7 — supersede lives in the pane.** The supersede control asks
      for the justification, lands through `Memory::supersede`, refuses
      in place with the writer's own reason (already superseded, already
      expired, missing justification), and refreshes the row to its
      retired state; needs you offers **run the cycle now**, and the
      section's untouched/kept rows name why they were left.
- [x] **B8 — every button works.** A harness check per action button —
      edit/save, supersede, run the cycle, ask, copy, file, search,
      budget, section pick — each asserted against its named element and
      its observed effect, in the same commit as the element.
- [x] **B9 — the categories are the substrate's own words.** Sections
      come from the fact's `type` (with the protected synonyms mapped),
      nothing else; the live corpus measured through them is recorded in
      this plan (the 114/34/13 counts re-verified at landing); free-form
      types land in everything else, visible.
- [x] **B10 — the gate and the measurement.** fmt, clippy, build, test
      green by exit code; the Chromium probe quotes the before/after
      heights and control counts; the screenshots read as the review.

## Files to be touched

**Backend / API:**
- `src/memory.rs` — `preview` extracted beside `consolidate`; the unit pin
- `src/boards.rs` — `read_memory` carries the preview, the archive list, the section word
- `src/gui/server.rs` — the two POST routes
- `src/gui/bridge.rs` — `fact_supersede`, `memories_consolidate`, the read's new fields

**Frontend:**
- `src/gui/assets/index.html` — the memory stage's rail nav
- `src/gui/assets/app.js` — `paintMemory` as rail + section painter; the pane's supersede control; the needs-you section

**Tests:**
- `tests/face/harness.mjs` — the per-button case and the section/route/pane cases
- `src/memory.rs` tests — the preview agreement pin; the supersede-route pin over a scratch corpus

**Documentation (see Linked artifacts below):**
- `docs/features/memory.md` — the surfaces section rewritten
- `TECHNICAL-DOCUMENTATION.md` — the memory module reference
- `FUNCTIONAL-SPECIFICATIONS.md` — the memory surface passage

## Out of scope

- **Creating a fact from the face** — `remember` is the model's tool and the CLI's verb; the pane edits, supersedes, and runs the cycle, and a create form is a named follow-up if the owner wants it.
- **Restoring an archived fact** — the archive stays read-only in the face; the cycle's forget is reversible by hand and by git, and an un-archive verb is its own decision.
- **Editing name, tier, or basis** — unchanged rule: they are the lifecycle's inputs, and renaming is a supersession (which this plan adds to the pane).
- **The ranker, the shapes, the recall path** — untouched; `--shape` filtering stays CLI-side.
- **The scheduled consolidator's timing** — unchanged; the button runs one pass on demand, it does not re-arm the timer.

## Verification

**B1** — the agreement pin over a written fixture:
```bash
cargo test the_preview_names_what_the_cycle_would_do
```
Expected: green — the preview's sets before the pass equal the pass's
actions after it, on the same fixture and day.

**B2** — the route pins over a scratch corpus:
```bash
cargo test the_face_supersede_lands_the_permission_model
```
Expected: green — inferred supersedes on justification, confirmed-by
supersedes with the click's approval, a missing justification is the
parser's typed refusal.

**B3–B8** — the face harness (scratch home, the memory cases bumped):
```bash
node tests/face/harness.mjs
```
Expected: every new check green; the count line quoted in the commit.

**B10** — the gate, whole, exit codes read directly; the Chromium probe
at 1440×900 quoting before/after; the screenshots read as the review.

## Landed 2026-10-30

B1: `the_preview_names_what_the_cycle_would_do` — the four sets before
the pass are the actions after it, on a fixture carrying all four
shapes. B2: `the_management_verbs_land_through_the_substrates_own_writers`
— a missing justification is the parser's 409, an inferred fact
supersedes on the recorded justification through `POST
/api/fact/supersede`, and one pass lands through `POST
/api/memories/consolidate`. B3–B8: the face harness, **451 checks across
47 cases** (the MEM case: the rail's sections and badges — empty type
words never render — one section at a time, the route, the queue's four
sets with the run-the-cycle POST, the one-line rows with the basis and
recalls marks, the budget's named press, the pane card with the edit
form and the supersede verb, the supersede POST carrying the
justification, the cross-section search results, the archive, and the
today section). B9: the live read — user 2, feedback 7, project 97,
reference 1, episodic 2, everything else 5 (the `state-sync`-typed
facts), retired 34, archive 13; the live queue honestly empty (every
inferred fact has been recalled or confirmed). B10: the gate whole —
fmt 0, clippy 0, build 0, the full suite green — and the probe's
measurement: before, one board **7,207 px tall with 339 controls**;
after, the largest section is project at **1,493 px with 34 controls**
(needs-you 439 px / 4, archive 1,141 px / 3, today 1,210 px / 15), 10
rail rows, one section-body at every address. The before/after
screenshots are read as the review (tmp/shots-memory-*). One defect the
build caught on itself: the dead-code sweep that retired the old board
took `memHits` with it and the today section threw — restored, and the
harness now opens every section so a missing builder cannot come back.

## Linked artifacts

- `docs/features/memory.md` — section "The surfaces": the rail, the type sections, the pane's verbs, the preview
- `TECHNICAL-DOCUMENTATION.md` section "Memory and skills" — the module table's memory entries gain `preview` and the two routes
- `FUNCTIONAL-SPECIFICATIONS.md` section "The surfaces" — the memory passage: sections, the pane, supersede and the cycle from the face

## Risks

- **The preview and the pass can drift if one filter changes without the other.** Mitigation: one filter code path — `preview` is extracted from `consolidate`, not a copy — and the agreement pin fails the gate if they diverge.
- **Supersede from a browser is a destructive-ish write.** Mitigation: the justification is required and recorded in the calibration row and the replacement's body; a confirmed-by fact already demanded exactly this approval — the click merely delivers it; the refusal paths are the writer's own typed words.
- **Run-the-cycle from the face runs a pass the scheduler would also run.** Mitigation: a pass is idempotent by construction (stamped facts are skipped, expired-but-folded facts skip the archive), the rows land on the same stream, and the button sits on the section that names what it will do.

## Dependencies

- The settings revamp (`2026-10-29-settings-rail.md`, completed) — `sectionRail`, `cfgItem`, `budgetRows`, the pane card idiom.
- The skills revamp (`2026-10-30-skills-rail.md`, draft) — the same pattern's first knowledge-group landing; the memories plan reuses its section painter shape and its needs-you ordering, and should land after it so the two surfaces read as one idiom.

## Notes

- The type field is free-form by design (KI-059's synonym mapping exists
  because the fork's corpus wrote `preference` and `identity`); the
  section set therefore maps synonyms before choosing, and **everything
  else** is a permanent resident, not a defect — today it holds the five
  `state-sync`-typed facts, which are legal files a pane edit can
  re-type.
- The counts this plan was written against (2026-10-30, the repo corpus
  at the face's launch root): 114 live (97 project, 7 feedback,
  5 state-sync, 2 user, 2 episodic, 1 reference), 20 confirmed-by / 94
  inferred, 103 durable / 2 episodic / 1 working, 34 retired in-tree,
  13 archived. B9 re-verifies them at landing.
- The memory corpus the face reads is the launch tree's own
  (`resolve_root` from the bridge's cwd — the repo's `memories/`), and
  that resolution is unchanged; the plan re-arranges the surface over
  the same read.
- The skills plan's S0 (the approve fix) has no analogue here: every
  existing memory button reads sound — the pane's save rides the tested
  rewriter route, and the file/ask/copy actions are client-side. The
  memories gap is missing verbs, not broken ones.
