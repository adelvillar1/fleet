---
status: completed
created: 2026-11-09
updated: 2026-11-09
slug: project-work-surfaces
---

# Plan: learning and the desk, organised by project and iterated in a thread

## Context

The owner's report (2026-11-09): *"redesign both learning and desk — they need the
composer and to be more chat centric so users can iterate on learning and
knowledge work. Their project organisation: a rail with projects and
deliverables. On learning the same pattern, project organised and showing the
different learnings associated to a project. Special care on dead buttons and
dead ends."*

Today the two surfaces are boards, not places to work. The **desk** has a shelf
of deliverables, a brief form and a project view, but no conversation: an
iteration is a fresh brief, and the thread that produced a deliverable is
invisible. **Learning** has a section rail (lesson, graph, practice, history)
with no composer at all — the lesson is read, never spoken to — and no project
dimension: a lesson is a session of kind `teaching`, and nothing binds it to the
work it teaches.

The mockup (measured and read, paper and graphite):
`docs/plans/mockups/2026-11-09-desk-learning/index.html`.

## Approach

One pattern, applied twice: **a projects rail on the left, the artifact's work in
the middle, and the thread with the composer on the right.**

**One projects read.** `GET /api/projects` unions what already exists — the
desk briefs' `project` binding, the lessons' project (new: the lesson's `start`
record carries it), and the config's registered project paths — into projects
with their artifact counts and an **unbound** bucket. The rails on both
surfaces read it, so a project is one name in one place.

**The desk becomes a conversation about a deliverable.** Selecting a project
lists its deliverables; selecting one opens its loop (brief, claims, coverage,
outline, review — the cards the surface already draws) beside the **thread**:
the sessions that deliverable's runs produced, rendered as the conversation they
are. The composer beneath sends through the existing desk route with the
deliverable's project and slug in the run's context, so an iteration reads the
same sources and the whole thread. The end gate stays where it is: the review's
refusal blocks closing, and the composer's `review before close` chip is that
gate's visible state, not a second opinion.

**Learning becomes a conversation about a lesson.** Selecting a project lists its
lessons (each with its state: in progress, held, done); selecting one opens the
lesson's plan view beside its **thread** — the lesson session's own log, which
already exists — and the composer beneath sends into that session with the
**role** that lesson runs under (the teacher for a teaching session, the
practitioner for a practice). "Next step" is a composer verb, not a button
elsewhere; a practice starts from its chip and lands in the same thread.

**No dead buttons, no dead ends.** Every visible control maps to an action:
- a project row selects and filters (and its count says how many artifacts);
- an artifact row opens that artifact's thread and work;
- the composer's chips are state made visible and pressable — `project` and the
  artifact are the thread's binding (pressing one opens its picker), `sources`
  opens the desk's source list, `role` toggles teacher/practitioner, `practice`
  starts one, `review before close` shows the gate;
- an empty project shows the one action that fills it ("start a brief here",
  "start a lesson here"), never a blank pane;
- a run in flight turns the composer's button into Stop, exactly as the chat's
  does; a refusal paints beside the composer;
- a project row with nothing in it is still selectable (it opens the empty state
  with its action).

High-level steps, each closing the criteria it names:

1. **C0** — the owner reads the mockup; the current surfaces' data and the
   project sources are read and recorded (what exists, what must be added).
2. **C1** — the projects read and its route; the unbound bucket.
3. **C2** — the lesson's project on its record and in the read; the learning
   rail and its list.
4. **C3, C4** — the desk rail and the desk thread; the learning rail and the
   lesson thread.
5. **C5** — the composer on both surfaces: the same idiom as the chat's, the
   chips, the role, the run's binding, Stop.
6. **C6, C7** — the empty states, the refusals in place, the whole-control audit.
7. **C8** — the harness cases, one per control group and per surface.
8. **C9** — the feature docs, the ledger, the recap, both checkers, the gate, the
   cut.

## Acceptance criteria

These are the contract; each is a verifiable assertion. The work is done when
every box is checked.

- [x] C0 — The mockup has been read by the owner, and the two surfaces' current data (the desk's project binding, the lesson's session kind, the config's project paths) is recorded with what the redesign must add.
- [x] C1 — One `GET /api/projects` returns every project with its deliverable count, its lesson count and an unbound bucket, and both rails render from that one document.
- [x] C2 — A lesson carries its project: the lesson's `start` record names it, the learning read returns it, and a lesson created without a project lands in the unbound bucket.
- [x] C3 — The desk rail lists projects and, under the selected one, its deliverables; pressing a deliverable opens its work and thread; pressing a project filters the rail and the work column to it.
- [x] C4 — The learning rail lists projects and, under the selected one, its lessons with their state; pressing a lesson opens its plan and its thread (the lesson session's own log, in order).
- [x] C5 — Both surfaces carry a composer in the chat's own idiom: Enter sends, the button turns Stop while a run is in flight, the send posts the surface's own route with the artifact's binding (the desk's project and slug; the lesson's session and role), and a refusal paints beside the composer.
- [x] C6 — Every composer chip does something visible: `sources` opens the desk's source list, `role` toggles teacher/practitioner in the learning composer, `practice` starts a practice in a new session, `review before close` shows the end gate's state; each is pinned by a harness case.
- [x] C7 — No dead ends: every project row and every artifact row is pressable and its press changes the pane; every empty state names the action that fills it; an empty project, an empty desk and a lesson with no plan each render their own action rather than a blank card.
- [x] C8 — The harness carries the cases: the projects read feeds both rails; a project press filters; a deliverable press opens its thread; a lesson press opens its log; the composer posts the right run per surface; the chips act; the empty states name their action. The suite is green.
- [x] C9 — `docs/features/desk.md` and `docs/features/learning.md` describe the rails, the threads and the composers; the ledger row is closed; `ki_ledger.py` and `plan_check.py` exit 0 at status: completed; the four-leg gate is green; the cut is published.

## Progress (2026-11-09, v0.6.14)

C0–C9 are met. The one `GET /api/projects` unions the desk's bound
deliverables and the lessons' working directories (a lesson's project is
its session's own workspace) with the unbound bucket; both panes are the
same shell — the projects rail, the work column, the thread with the
composer. The desk's thread is a `desk`-kind session born on the first
iteration and seeded with the brief; learning's is the lesson's session,
answered through `?session=`. The chips and the empty states were proven
by the harness's two `PWS` cases (15 checks); the whole suite is 1076
checks across 134 cases. C9 closed with the cut (v0.6.14, then 0.6.15 and 0.6.16 for the two defects the
live read caught: the lesson's project missing from its own row in the read, and
the shells landing in one column of the board's twelve-column grid). The visual
read of both surfaces shows three clean columns, and the live presses are in the
recap.

## Files to be touched

**Backend / API:**
- `src/gui/bridge.rs` — the projects read, the lesson's project, the lesson thread's read (the session log), the desk thread's read (the deliverable's sessions).
- `src/gui/server.rs` — `GET /api/projects`; whatever the threads need.
- `src/lesson.rs` — the lesson's `start` record gains its project.

**Frontend:**
- `src/gui/assets/app.js` — `paintDesk`, `paintLearning` and their rails, threads and composers.
- `src/gui/assets/app.css` — the rails, threads and composer states.

**Tests:**
- `tests/face/harness.mjs` — the cases C3–C8 name.

**Documentation:**
- `docs/features/desk.md`, `docs/features/learning.md` — the new surfaces.
- `docs/known-issues.md` — the owner's report as a row, closed with the proofs.
- `CHANGELOG.md` — the release sections.

## Out of scope

- Moving deliverables or lessons between projects (a drag is a later increment).
- Cross-project views ("everything in progress") beyond the unbound bucket.
- The desk's or the lesson's internals: the loop's stages and the teaching method stay as they are; this redesign is the surface around them.
- Reworking the memory/skills/sessions rails to the same pattern (the same idea, another plan).

## Verification

**C0** — the mockup read in paper and graphite; a note in the plan naming what the surfaces carry today (the desk's `project` field, the lesson's `start` kind, the config's registrations).
**C1** — `curl /api/projects` names every project with its counts; the harness asserts both rails render from it.
**C2** — a lesson started with a project carries it in its log and in `/api/learning`; one without lands under unbound.
**C3** — harness: press a project, the rail narrows; press a deliverable, its work and thread paint.
**C4** — harness: press a lesson, its plan paints and its thread lists the session's records in order.
**C5** — harness: Enter posts `/api/desk` (or the lesson's run) with the binding; the button's mode is `stop` while running; a refused post paints beside the composer.
**C6** — harness: one case per chip.
**C7** — harness: the three empty states name their action; a press on every row type changes the pane.
**C8** — `node tests/face/harness.mjs` green with the new cases named.
**C9** — `python3 scripts/ki_ledger.py`, `python3 scripts/plan_check.py`, the four gate legs, and the cut's own feed read-back.

## Linked artifacts

- `docs/features/desk.md` — the rail, the thread, the iteration composer.
- `docs/features/learning.md` — the project dimension, the lesson thread, the role's composer.
- `FUNCTIONAL-SPECIFICATIONS.md` — the two surfaces' controls.
- `CHANGELOG.md` — the release section, at cut time.
- `docs/plans/mockups/2026-11-09-desk-learning/index.html` — the measured, read mockup this plan builds.

## Risks

- **The desk's thread needs its runs bound to the deliverable.** The brief already keys the project; the plan must confirm the deliverable's own sessions are discoverable (and if not, bind the run to the slug the same way).
- **The learning thread is free** — a lesson is a session — but its composer must not fork a second conversation beside the log; the send goes into the same session the lesson is.
- **Two surfaces in one plan is a lot of diff.** The criteria are grouped per surface so a partial landing is visible as partial.
- **The rails duplicate the memory/skills idiom.** They reuse `sectionRail`; a divergence would be a house collision (prefix the classes).

## Dependencies

- `docs/plans/mockups/2026-11-09-desk-learning/index.html` — measured (no overflow, no clipped text, ink 5.29–16.05:1) and read in paper and graphite.
- The owner's report of 2026-11-09 (this plan's context).
- `docs/features/desk.md`, `docs/features/learning.md` — the machinery the surface sits on.
