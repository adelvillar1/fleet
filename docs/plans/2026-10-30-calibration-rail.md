---
status: completed
created: 2026-10-30
updated: 2026-10-30
slug: calibration-rail
---

# Plan: the calibration revamp — the gate first, the stream's own words as sections, the day at hand

## Context

The calibration surface is the knowledge group's last long page and the
only one still outside the work rail's idiom: one scroll of eight cards —
the head, the strip, a three-chip filter, the fit cells, then the day's
stream as one card per op (the last 20 rows each) — **2,860 px tall with
59 controls and 55 stream rows** on the face as it serves. The deep dive
(2026-10-30) measured what the revamp stands on:

- **The vocabulary is the substrate's own.** The stream carries thirteen
  ops (`route`, `route_judged`, `turn`, `delegation`, `propose`,
  `eval_corpus`, `recall_hit`, `consolidate`, `tool_screen`, `explain`,
  `verify`, `embed_timing`, `lifecycle` — `src/calib.rs`'s constants, the
  fork's dev-decisions inheritance); the face groups nine of them and
  drops the rest. The categories already exist; the rail just needs to
  carry them all, named, with only the day's live ops shown.
- **The gate is the decision the surface exists to serve.** The
  failover-learning gate (`calibration.informs_failover`), the fit rule
  (`FIT_MIN_ROWS` graded rows, `FIT_MIN_SPAN` confidence span), and the
  fit cells that inform it — today they sit mid-page behind a filter.
  They are the needs-you of this surface: what the owner arrives to
  judge.
- **The eval corpus has no home on the face.** An `eval_corpus` row
  carries the twenty scenarios' verdicts (`scenario`, `family`,
  `passed`, `failure` — the bridge already ships them), and the surface
  renders only the aggregate line. The corpus is the harness's model of
  its own performance; it deserves its section.
- **The day axis stops at today.** The read is today-only; the CLI has
  `--day` and the substrate has `read_day` for any day. The surface's
  biggest usability gap: yesterday's trail is unreachable from the face.

The owner's direction, 2026-10-30: the same rail pattern — *"let's do a
deep dive of calibration and then use the rail to improve organization
and usability"*.

## Decisions (the owner's, dated and quoted)

1. **2026-10-30 — the pattern.** *"one more surface to go using the same
   pattern … use the rail to improve organization and usability"* — the
   shared `sectionRail`, one section at a time, the pane for the record.
2. **2026-10-30 — the deep dive first.** *"let's do a deep dive of
   calibration and then"* — the sections come from the substrate's own
   op vocabulary and the gate's own decision, measured before written.

## Approach

**The gate leads.** The first section is **the gate**: the failover
gate's state, the fit rule in its own numbers, and the fit cells — each
cell as a one-line row (provider · shape, its fit badge, what it still
needs) whose press opens the pane card (the cell's counts, rates, spend,
and the ask-sys1 action, tabs stood down). This is what the surface
exists to serve; it stops being a mid-page card behind a filter chip.

**The stream's own words are the sections.** One section per op the day
actually carries, badge = its row count, in the substrate's own order:
routes, judgments, turns, delegations, proposals, eval runs, recall
hits, consolidations, tool screens, explains, verifies, embed timings,
lifecycles. The old three-chip filter retires — the rail is the filter.
A row's press keeps the pane record it has (the raw JSON card), now with
the tabs stood down like every other surface.

**The eval corpus gets its section.** When the day's last `eval_corpus`
row carries scenarios, the **eval runs** section renders the aggregate
strip and the twenty scenarios as one-line rows — scenario · family, its
pass badge, its failure named — the harness's model of its own
performance, readable without the CLI.

**The day comes to hand.** The route takes `?day=YYYY-MM-DD` (the CLI's
own flag, the substrate's own `read_day`); the instrument carries a small
day stepper — back one day, forward one day, today named — and a day with
no rows renders its honest empty state, not yesterday's rows. The strip
and every section follow the chosen day.

High-level steps:

1. **C0 — the day and the sections in the read.** The bridge's
   calibration read takes the day; the document carries the per-op rows
   it already does plus the ops it drops today (`delegation`, `verify`,
   `embed_timing`, `lifecycle`).
2. **C1 — the rail and the sections.** The stage-split markup, the
   shared `sectionRail`, the hash route `#/calibration/<section>` (with
   the day riding the query), the section painter.
3. **C2 — the gate, the rows, and the corpus in the pane.** The gate
   section's one-line cells and the pane card; the op sections' rows and
   the record card; the eval section's scenario rows.
4. **C3 — every interaction proven, the measurement, the docs.**

## Increments

Each lands whole — code, checks, docs — and each is green on its own.

### C0 — the day and the sections

`calibration_read(home)` becomes `calibration_read_at(home, day)` — the
same document, the day explicit, `today` delegating. The stream map
gains the four dropped ops (`delegation`, `verify`, `embed_timing`,
`lifecycle`) in `row_json`. The route takes `?day=`, refusing a
malformed day with the CLI's own words.

*Checks:* T1.

### C1 — the rail and the sections

`index.html`'s calibration stage gains the rail nav (stage-split);
`paintCalibration` becomes the rail painter plus one section painter —
the gate section, then the ops the day carries, the eval-corpus section
flagged from its row's payload; `#/calibration/<section>` addresses one,
the rail remembers; the filter chips retire.

*Checks:* T2, T3, T4.

### C2 — the gate, the rows, and the corpus

The gate section renders the gate state, the fit rule, and one-line cell
rows under the pane-card idiom; the op sections render their rows as
one-line items (the `streamLine` text, kept) whose press opens the pane
record; the eval section renders the aggregate and the twenty scenarios.
The day stepper rides the instrument.

*Checks:* T5, T6, T7.

### C3 — every interaction proven, the measurement, the docs

A harness case over a fixture day (cells in all three states, six ops,
an eval row with scenarios, a day stepper); the probe's before/after;
the screenshots; the docs.

*Checks:* T8, T9, T10.

## Use cases

- [ ] A: The owner judges the failover gate and its fit cells first, not behind a filter chip mid-page.
- [ ] B: Any op's day-row trail is reachable as a section, including the four ops the surface dropped.
- [ ] C: The eval corpus's twenty scenarios are readable from the face.
- [ ] D: Yesterday's calibration is one step back, not unreachable.

## Acceptance criteria

- [x] **T1 — the read takes the day.** `calibration_read_at(home, day)`
      answers that day's rows and cells; the route's `?day=` carries it
      and a malformed day is refused with the CLI's own words; absent is
      today.
- [x] **T2 — the rail is the shared helper.** The panel renders
      `sectionRail` (store `fleet-calibration-rail`), collapses to
      icons, survives a reload, marks `aria-current`, takes the arrow
      keys, and only the open section's cards exist in the DOM.
- [x] **T3 — a section is an address.** `#/calibration/<section>` opens
      it; bare `#/calibration` resumes the remembered one; an unknown
      segment falls back to the gate.
- [x] **T4 — the sections are the substrate's own words.** The gate
      leads; the ops-with-rows follow in the substrate's order with
      honest badges; an op with no rows that day renders no section; the
      three-chip filter retires.
- [x] **T5 — the gate section serves the decision.** The gate state,
      the fit rule in its own numbers, and one-line cell rows (fit
      badge, what it still needs) render first; a cell's press opens the
      pane card with the counts, the rates, the spend, and the ask-sys1
      action, tabs stood down.
- [x] **T6 — the rows keep their records.** An op row's press opens the
      pane record card (the raw row the reader parsed), tabs stood down;
      the four previously-dropped ops render with the same discipline.
- [x] **T7 — the corpus is readable.** The eval section renders the
      aggregate and the last run's scenarios as one-line rows —
      scenario · family, the pass badge, the failure named; a day whose
      eval row carries no scenarios names it.
- [x] **T8 — the day is at hand.** The stepper steps back and forward
      through days, the strip and the sections follow, and an empty day
      renders its honest empty state; the route's `?day=` opens that day
      directly.
- [x] **T9 — every interaction works.** A harness check per interaction —
      the section pick, the route, the cell press, the row press, the
      corpus rows, the day stepper — each asserted against its named
      element and its observed effect, in the same commit as the element.
- [x] **T10 — the gate and the measurement.** fmt, clippy, build, test
      green by exit code; the probe quotes the before (2,860 px / 59
      controls / 8 cards) against the after; the screenshots read as the
      review.

## Landed 2026-10-30

T1: `the_calibration_read_answers_the_day_it_is_given` — two written day
files read apart (yesterday's delegation row, today's embed timing), a
malformed day refused with the CLI's words. The deep dive's one wrong
assumption, corrected in the build: the parse itself dropped three of
the four ops — `OPS` carried ten — so the read widened with them
(`verify`, `embed_timing`, `lifecycle`; `ReadRow` gains their fields,
all defaulted, and `fleet calib` gains the groups beside). T2–T9: the
face harness, **468 checks across 49 cases** — the CAL case pins the
rail (the gate first, the day's ops, no empty sections), one section
body, the gate's cells and their pane card, the section route, the
delegation row's child-and-branch, the record in the pane, the corpus's
aggregate and scenarios, and the stepper reading yesterday. T10: the
gate whole (fmt 0, clippy 0, build 0, the suite green — quoted in the
commit) and the probe's measurement: before, one board **2,860 px / 59
controls / 8 cards / 55 rows**; after, the gate section at **350 px / 3
controls**, the busiest op section (embed timings, 18 rows) at **902 px
/ 21**, one section body at every address — and the live rail carrying
nine sections including the four the surface used to drop. The
screenshots are read as the review (tmp/shots-calibration-after).

## Files to be touched

**Backend / API:**
- `src/gui/bridge.rs` — `calibration_read_at(home, day)`; the four ops
  in `row_json`; the route's day parameter
- `src/gui/server.rs` — `?day=` on the calibration route

**Frontend:**
- `src/gui/assets/index.html` — the calibration stage's rail nav
- `src/gui/assets/app.js` — the rail painter, the section painter, the
  gate section, the corpus rows, the day stepper

**Tests:**
- `tests/face/harness.mjs` — the calibration-fixture case

**Documentation (see Linked artifacts below):**
- `docs/features/calibration.md` — the surface section rewritten
- `TECHNICAL-DOCUMENTATION.md` — the calib module entry
- `FUNCTIONAL-SPECIFICATIONS.md` — the calibration passage

## Out of scope

- **Writing to the stream** — the surface stays read-only; rows come
  from runs, the gate flips through settings.
- **The delegation calibration surface** (`fleet calib --delegation`) —
  the per-(provider × brief-shape) beat stays CLI-side; the delegation
  rows render in their stream section.
- **A day-range aggregate** — one day at a time, the stepper's unit;
  cross-day trends are a named follow-up.
- **The judge/delegate/explain beats' CLI surfaces** — unchanged.

## Verification

**T1** — the day pin over a written fixture day:
```bash
cargo test the_calibration_read_answers_the_day_it_is_given
```
Expected: green — yesterday's rows answer for yesterday, today's for
today, a malformed day refuses with the CLI's words.

**T9–T10** — the face harness and the gate:
```bash
node tests/face/harness.mjs
cargo fmt --all --check && cargo clippy --all-targets -- -D warnings \
  && cargo build --release && cargo test
```
Expected: every new check green; the count line quoted in the commit;
the gate's four exit codes read directly; the probe quotes the
before/after; the screenshots are read as the review.

## Linked artifacts

- `docs/features/calibration.md` — "The surfaces": the rail, the gate
  section, the ops, the corpus, the day stepper
- `TECHNICAL-DOCUMENTATION.md` section "Self-calibration and
  self-maintenance" — `calib.rs`'s entry gains the face's rail and the
  day read
- `FUNCTIONAL-SPECIFICATIONS.md` section "The surfaces" — the
  calibration passage

## Risks

- **The 512-row cap hides a busy day's tail.** The strip already names
  "the day's last 512"; the sections inherit the note, and the cap is
  the substrate's own bound — a busy day's earlier rows stay CLI-side,
  named.
- **The day stepper walks past the stream's first day.** An empty day
  renders its honest empty state (no rows is a fact, not an error); the
  stepper does not guess the stream's bounds.
- **The eval scenarios ride one row.** A day's LAST eval_corpus row
  carries the corpus render; earlier runs the same day stay in the
  eval section's row trail.

## Dependencies

- The settings revamp (`2026-10-29-settings-rail.md`, completed) — the
  shared `sectionRail`, the object-card idiom.
- The three landed revamps (skills `8c36c6f`, memory `737af35`,
  workspace `2cbc626`) — the idiom this surface completes.

## Notes

- The before-state was measured on the face as it serves: **2,860 px,
  59 controls, 8 cards, 55 stream rows** — one scroll, the gate's
  decision behind a filter chip, four ops dropped, the corpus
  aggregate-only, today unreachable from yesterday.
- The ops the surface drops today and the plan adds: `delegation` (the
  orchestrator's own rows, the join to the child sessions),
  `verify` (the fact-gate verdicts), `embed_timing` (KI-249's
  measurement rows), `lifecycle` (the memory facts' births and
  retirements).
- The fit cells' pane card is the existing drawer's body — the plan
  adds the tabs-stood-down idiom and the one-line row, not a second
  card builder.
