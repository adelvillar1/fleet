# Session Recap — 2026-11-09: the desk and learning, organised by project

**Summary.** The owner asked for the desk and learning to be redesigned around
**projects** and **conversation**: a rail of projects and their artifacts on both
surfaces, and a composer so the owner can iterate on a deliverable and on a
lesson. It shipped as 0.6.14, with 0.6.15 and 0.6.16 carrying the two defects the live
read caught. Both panes are now one shell: the projects rail, the artifact's
work, and the thread with the composer.

## The pattern

- **One projects read.** `GET /api/projects` unions the desk's bound
  deliverables (their `project` folder) and the lessons' own working
  directories (a session's `workspace` — the fact the archive already groups
  by) into projects with counts, plus an unbound bucket. No schema change was
  needed: both surfaces already carried their project, and nothing read it as
  one.
- **The desk's thread is a real conversation.** A deliverable's thread is a
  session of kind `desk`, born on the first iteration and seeded with the brief
  (so the brief is its first message and the model reads the artifact from the
  log). `POST /api/desk/<slug>/chat` runs the ordinary run path with the
  deliverable's project as its folder; the loop's cards stay beside it.
- **Learning's thread is the lesson itself.** A lesson is a session, so its
  replay is its thread; the composer answers into that session
  (`/api/learning/start?session=`), its role chip is the role the run rides,
  and the practice chip starts a practice beside the lesson. The four sections
  moved from the pane's rail to chips in the work column, because the rail is
  the projects rail now.

## No dead controls

Every control was pressed in the harness and (where there is live data) on the
running face:

| Control | What it does |
|---|---|
| a project row | selects it, expands its artifacts |
| a deliverable row | opens its work and its thread |
| a lesson row | opens its plan and its thread |
| the composer | posts the surface's own run with the artifact's binding |
| the project chip | selects the project in the rail |
| the artifact chip | re-reads the artifact and its thread |
| `sources` (desk) | opens the brief's source list |
| `review` (desk) | shows the end gate's standing refusals |
| `role` (learning) | flips teacher / practitioner |
| `practice` (learning) | starts a practice in its own session |
| an empty rail | offers the act that fills it |

A refusal paints beside the composer, and the composer's button is Stop while a
run is in flight.

## The pins

`PWS the desk is projects, work and a thread` and `PWS learning is projects, the
lesson and its thread` — fifteen checks across the rail's listing and expansion,
the artifact opening, the thread's replay, the composer's post, and each chip.
The suite is 1076 checks across 134 cases; the lib is 2071 tests. The gate's
four legs were green at both cuts.

## The live proofs

- `GET /api/projects` on the running face: `[('fleet', 0, 1)]` — one project,
  zero deliverables (none exist on this machine yet), one lesson.
- The learning rail renders `fleet · 1 · 1 lesson`; pressing it expands to the
  lesson; the thread replays the lesson session's own log (the lanes, the
  owner's words, the teacher's answer).
- The role chip flipped `role · teacher` → `role · practitioner`.
- The desk renders the rail (`fleet · 0 deliverables`), the empty shelf with its
  start action, and the composer.
- The desk chat route refuses honestly: a slug that is not there is a **409**
  naming the read failure, empty words a **400**.

**What the live read caught:**

- **0.6.15** — the learning document returned each lesson's project in the
  history rows but not on the lesson the pane shows, so the thread's project
  chip read `unbound` beside a rail that had the lesson under `fleet`. The read
  carries it on the lesson now.
- **0.6.16** — both shells landed in one column of the board's own twelve-column
  grid, so the panes squeezed to a twelfth of the width: the section chips
  stacked and the thread overlapped the work. Only the screenshot showed it; the
  harness has no layout. `.pw-shell` and `.pw-we` span every column now, and the
  re-read shows three clean columns on both surfaces (the desk's rail, empty
  shelf and thread; learning's rail, lesson card and thread with the composer).

## Files changed

- `src/gui/bridge.rs` — `projects()`, `desk_chat()`, the seeded brief, the
  lesson's project (history and active), the learning read's `?session=`.
- `src/gui/server.rs` — `GET /api/projects`, `POST /api/desk/<slug>/chat`, the
  learning read's session parameter.
- `src/desk.rs` — the deliverable's `session` field and `attach_session`.
- `src/gui/assets/app.js` — the projects shell, the rail, the threads, the
  composers, the chips, the desk and learning rewirings.
- `src/gui/assets/app.css` — the shell's styles.
- `tests/face/harness.mjs` — the two `PWS` cases and the projects route.
- `docs/features/desk.md`, `docs/features/learning.md`, the plan, the ledger
  (KI-501), `CHANGELOG.md` (0.6.14, 0.6.15).

## Open questions / next steps

- **No desk deliverables exist on this machine**, so the desk's iteration has no
  live press behind it — but it is no longer unproven: the bridge test
  `a_desk_iteration_opens_the_deliverables_thread` drives `desk_chat` twice
  through the command door and asserts the whole path (the session born with
  kind `desk`, the brief as its first message, the deliverable's `session`
  field written, the run dispatched with the deliverable's project as its
  folder, and the second press reusing the same thread without a second
  brief). The owner's first brief will show the thread opening with the brief
  in it.
- Moving an artifact between projects (a drag) and cross-project views beyond
  unbound stay out of scope, as the plan says.
- The same shell could carry the memory, skills and sessions rails later; the
  pattern is shared code, not three copies.
