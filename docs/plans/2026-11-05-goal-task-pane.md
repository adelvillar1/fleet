---
status: active
created: 2026-11-05
updated: 2026-11-05
slug: goal-task-pane
---

# Plan: the goal's task pane

## Context

The owner's ask: whenever a goal is running, the right-side collapsible
pane should show a running list of the tasks and each one's completion
status, live throughout the goal's execution.

The substrate already exists and is not surfaced. The `todo` tool
(`src/tools.rs:1697`, one of the eleven built-ins) is the run's task
list: the model sends the complete list every call
(`items[] {step, status}`, schema `tools.rs:1952-1973`; `todo_write` and
`update_todo_list` are aliases, `tools.rs:2741`), the agent stores it
(`self.todos`, `agent.rs:1389`; `store_todo`, `agent.rs:6931` — statuses
normalized to `pending` / `in_progress` / `completed`), it takes the
single `todo` write slot so two calls in a batch cannot race
(`agent.rs:6884-6886`), and it survives a fold as
`## Task list (carried across the fold)` (`render_todo_block`,
`agent.rs:6955`, consumed by the compaction summary at `agent.rs:3642`
and `3685`). What it does not have: the face never hears of it (zero
`todo` hits in `app.js` and `bridge.rs`), and nothing typed is written
to the log — only the ordinary tool records. A restarted session
rebuilds nothing.

The surface also already exists: the right pane
(`aside.pane#pane`, `index.html:1003`, opened by `#pane-open`, keyed on
`body[data-pane="open"]`) with its run tab holding the process card and
the live delegation branches. Mid-run events flow over the existing SSE
`/events` (`tool_call` / `tool_result` ride `describe()`,
`bridge.rs:10676`, through `record()`, `bridge.rs:10611`, which orders,
caps, and broadcasts). Reload rebuilds through `cards_from_records`
(`bridge.rs:6246-6470`) — `Record::Goal` → the `goal` card at
`bridge.rs:6426` is the precedent for a typed state the face keeps.

One declared thing is missing on the goal side: the comment at
`agent.rs:6387-6393` says "the `goal-complete` UI event rides the ui
channel" — no such variant exists. The face learns of a completed goal
only through a tool-result card, and `sessions-goals-done.json`
(`agent.rs:6444`) is read by the agent alone.

Decision-model check: nothing here is a repeated per-item judgment —
the statuses are the model's own declarations relayed verbatim through
an existing exact-match alias map. No sys1 job; said so rather than
left unnamed.

## Direction lock

- **The `todo` list is the task list.** No second task store, no new
  file, no new endpoint. The log is the store (hard rule 5); the pane
  is a view.
- **The pane is `aside.pane`**, run tab, beside the process card. The
  owner's "right-side collapsible pane" is this exact surface; no new
  collapsible is invented.
- **The face shows what the model declared, nothing else.** An empty
  list renders an empty (or absent) card — never a list synthesized
  from tool calls.
- **A goal-started run opens the pane; a manual run does not.** The
  auto-open rides the owner's own set-goal reply (immediate, not on the
  next paint-cycle event).

## Approach

Four seams, one JSON shape (`{"type":"tasks","items":[{"step","status"}]}`,
statuses already normalized by `store_todo` — the producer speaks the
shape, the face never parses tool arguments):

1. **The record.** `Record::Tasks { items: Vec<TaskItem> }`
   (`TaskItem { step, status }`, the tool's own status vocabulary),
   written in the dispatch path beside the tool result every time an
   accepted `todo` call lands (`agent.rs:4176`'s neighborhood, the same
   seam that already calls `store_todo`). Replay handled in
   `session.rs` like every record; non-replaying to the transcript as a
   message (the `Goal` precedent, `session.rs:325`, `1380`).
2. **The event.** `UiEvent::Tasks { items }` emitted at the same site;
   one `describe()` arm (`bridge.rs:10676`) → `record()` carries it to
   the in-memory log and the broadcast, so live subscribers and the
   refresh replay both get it.
3. **The rebuild.** A `cards_from_records` arm turns `Record::Tasks`
   into the same `tasks` card; the face's `apply()` case sets
   `face.tasks` from either path. The last full-replacement call wins —
   the tool's own contract is the state machine.
4. **The goal's own completion.** `UiEvent::GoalDone { text, note }`
   emitted from `run_goal`'s `complete` arm, `Record::GoalDone`
   written beside it, both described and rebuilt like the above — the
   event the code comment already declares. This is the card half of
   the unattended-goals plan's C7; that plan keeps the chip's styling.
5. **Pins, proof, docs.** The Rust pins cover the record round-trip
   (C1), the event on the wire (C2), the rebuild from a replayed log
   (C3), the goal-done record (C6), and the prompt sentence (C7); the
   fold's existing test stands updated only where a count shifted (C8).
   `tests/face/harness.mjs` gains the surface's cases — live paint,
   empty-hide, full-replacement repaint, auto-open, goal-done, replay
   (C4/C5/C9) — and the live pass drives the release binary on a
   scratch home end to end (C10). The gate runs whole before any push
   carrying code — `cargo fmt --all --check`, `cargo clippy --all-targets
   -- -D warnings`, `cargo build --release`, `cargo test`, each exit
   code read on the invocation, never through a status-losing pipe
   (C11) — and the doc updates land with the work:
   `gui.md`'s process-card section, `tools.md`'s `todo` row, and the
   two contract docs (C12).

The card: in `#pane-run` after `#process-card`, a goal header (the
session's standing goal; `active` → `done` on `goal_done`) above one row
per task — mark and step text, the in-progress row carrying the
active treatment, completed rows settling. Empty list → the card is
hidden. Long list → the card scrolls internally. The exact marks, type,
and ink come from the mockup (C0), drawn in the app's own tokens.

The prompt nudge: the goal section of the run prompt (`RunOptions.goal`,
`agent.rs:431-437`, composed at `agent.rs:1895`) gains one fixed
sentence naming `todo` as the way to track progress on the work. The
sentence is a prompt-builder constant (reconstructable per hard rule 5's
own terms); the goal text itself stays pinned by its `goal` record.

## Precedents this plan answers

- **`Record::Goal` → the `goal` card** (`bridge.rs:6426`): a typed
  record the face keeps as state — this plan repeats the pattern for
  `Tasks` instead of having JavaScript parse raw tool arguments (the
  fixture-vs-bridge lesson: fix the shape at the producer).
- **Unattended goals** (`docs/plans/2026-11-05-unattended-goals.md`):
  its C5 landed the `goal` tool; its C7 wants the goal's done state on
  the face and its C9 wants harness cases. This plan lands the
  `GoalDone` signal both need and leaves the chip's styling to that
  plan. Its uncapped main loop is why the pane matters: long unattended
  runs need their progress visible without polling the transcript.
- **House mockups first**: the mockup is C0, measured AND the PNGs
  read, and the owner reads it before any code lands — the huddles and
  desk scars.
- **A deferred action is an error**: the auto-open happens on the
  set-goal reply's own `started: true` (`bridge.rs:4294-4299`), the
  moment the owner acts — never "when the next event arrives".
- **The eval count scar** (`records.len()` assertions): new mid-run
  records shift record counts in replay tests. The plan treats that as
  a deliberate update per affected test, not a blind renumber.

## Use cases

- **UC1 — set a goal, watch it work.** The owner sets "draft the
  release notes, then verify every linked issue exists" with no run in
  flight. The pane opens on the run tab showing the goal header
  (active) and, as the model declares steps, one row per task whose
  mark moves pending → in_progress → completed, live, without a
  refresh.
- **UC2 — leave and come back.** A long goal run folded twice
  overnight. The owner reloads the face: the pane rebuilds the list —
  three done, one in progress — from the log, and the fold carried the
  list into the model's context so the run itself never lost it.
- **UC3 — the goal finishes.** The model calls `goal` with `complete`;
  the card's header flips to done with the tool's note, the row list
  stays as the record of what happened, and the run ends on the model's
  report as it already does.

## UX routes

No new HTTP routes. The existing `GET /events` carries two new event
types (`tasks`, `goal_done`); the existing `GET /api/state` replay and
`POST /api/goal` (whose reply already carries `started`) are unchanged
in shape. The card lives inside the existing pane — no route, no
surface change beyond the pane body.

## Acceptance criteria

- [ ] **C0 — the mockup is drawn, measured, read, and owner-read.**
      `docs/plans/mockups/2026-11-05-goal-task-pane/` holds the card in
      the pane, static HTML in the app's own tokens with the theme
      switcher riding; measured rendered in every theme (no overflow,
      no clipped text, ink ≥ 4.5:1) and the PNGs read in a light and a
      dark theme; the owner has read it before any code of C1+ lands.
- [ ] **C1 — the list is a log record.** Every accepted `todo` call
      writes `Record::Tasks` beside its tool result; `session::replay`
      round-trips it; the record does not replay as a transcript
      message. Pinned by tests.
- [ ] **C2 — the list is a live event.** `UiEvent::Tasks` is emitted
      where the list is stored; `describe()` maps it to
      `{"type":"tasks","items":[…]}`; a subscriber on `/events`
      receives it mid-run. Pinned by tests.
- [ ] **C3 — a restart rebuilds the list.** After a full restart, a
      session whose last run declared tasks shows that list in the pane
      through the `cards_from_records` arm — no state is kept anywhere
      but the log. Pinned by a replay test.
- [ ] **C4 — the card is true.** One row per task in order, the mark
      matching the normalized status (`pending` / `in_progress` /
      `completed`), the in-progress row visually distinct, completed
      rows settled; hidden while the list is empty; scrolls internally
      when long. Pinned by harness cases.
- [ ] **C5 — the pane opens when a goal starts working.** A goal set
      with no run in flight opens the pane on the run tab immediately
      (the reply's `started: true`); a manually started run never opens
      the pane by itself; the pane's open/closed state still obeys the
      owner's close. Pinned by harness cases.
- [ ] **C6 — the card names the goal and its end.** The header shows
      the session's standing goal while it runs and flips to done when
      `goal` `complete` lands — `UiEvent::GoalDone` + `Record::GoalDone`
      exist, both reach the face live and on rebuild, and the tool's
      note is shown. Pinned by tests and a harness case.
- [ ] **C7 — goal runs are told to track.** A run with a standing goal
      carries the one-sentence `todo` instruction in its prompt's goal
      section; a run without a goal does not. Pinned by a prompt test.
- [ ] **C8 — the fold still carries the list.** The existing fold
      block survives this plan's changes unchanged (a run that folds
      mid-goal keeps its list in context). Pinned by the existing test,
      updated only if its count shifted.
- [ ] **C9 — the harness counts the surface.** `tests/face/harness.mjs`
      gains the live-paint, empty-hide, full-replacement, auto-open,
      goal-done, and replay cases; the harness is green with the new
      cases counted.
- [ ] **C10 — the live proof.** On a scratch home: a multi-step goal
      set through the face, the pane opening on start, tasks appearing
      and progressing live in the SSE stream, a fold mid-run, the goal
      completing through the tool, the header flipping to done, a
      restart rebuilding the list — every observation quoted in the
      recap.
- [ ] **C11 — the gate runs whole locally before any push carrying
      code.** The four commands of the house gate — `cargo fmt --all
      --check`, `cargo clippy --all-targets -- -D warnings`,
      `cargo build --release`, `cargo test` — each exit 0 with its exit
      code read on the invocation, never through a status-losing pipe.
- [ ] **C12 — the docs.** `docs/features/gui.md`'s process-card section
      gains the tasks card; `docs/features/tools.md`'s `todo` row names
      where the list surfaces; the two contract docs' sections below
      are updated; the recap quotes C10.

## Files to be touched

- `src/session.rs` — `Record::Tasks` and `Record::GoalDone`, replay
  handling, the non-replaying list.
- `src/agent.rs` — the `UiEvent::Tasks` emission at the store site, the
  record writes, `UiEvent::GoalDone` + `Record::GoalDone` in
  `run_goal`'s complete arm, the goal-section instruction sentence.
- `src/gui/bridge.rs` — `describe()` arms for both events,
  `cards_from_records` arms for both records.
- `src/gui/assets/app.js` — `face.tasks`, the `apply()` cases, the
  card painter, the auto-open on the goal reply, the goal-done marking.
- `src/gui/assets/app.css` — the card's styles, from the mockup's CSS.
- `src/gui/assets/index.html` — the card's skeleton in `#pane-run`.
- `tests/` — the record/replay/emit/prompt pins.
- `tests/face/harness.mjs` — C4/C5/C6's cases.
- `docs/plans/mockups/2026-11-05-goal-task-pane/` — the mockup.

## Out of scope

- **The plan-graph nodes in the pane.** The supervision vocabulary
  (`fail`/`demote`/`settle`, `9d6bc4a`) already renders in the
  `#/delegation` panel and the live branch card; the delegated-child
  view is not duplicated here.
- **A tasks store file.** The log is the store; no sidecar like the
  goals' JSON files is added.
- **Editing the list from the face.** The model owns the list; the face
  reads it. Steering happens through the existing steer box.
- **The goal chip's styling.** The `GoalDone` signal lands here; the
  chip's active/done presentation is the unattended-goals plan's C7.
- **Cron- or resume-started goal runs auto-opening the pane.** Only the
  owner's own set-goal action opens it; a run that starts while the
  owner is away does not grab the surface.

## Verification

```sh
# the mockup: render at real size in all four themes, measure, read the PNGs
# (the desk-ux measurement harness; both checks, not one)

# the pins and the suite
cargo test

# the face
node tests/face/harness.mjs

# the gate — exit codes read on the cargo invocation, never through a pipe
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo build --release
```

C10's live pass drives the release binary on a scratch home
(`FLEET_HOME=tmp/goal-pane-home`), sets a multi-step goal through the
face, and quotes: the pane opening, each `tasks` frame on `/events`, the
fold's carry, the `goal_done` frame, the restart's rebuild.

## Linked artifacts

- `docs/features/gui.md` — "The process card": the tasks card beside
  it, the auto-open, the goal header.
- `docs/features/tools.md` — the `todo` row: where the list surfaces.
- `FUNCTIONAL-SPECIFICATIONS.md` — the conversation surface's goal
  behavior: the running task list and its completion marks.
- `TECHNICAL-DOCUMENTATION.md` — the event/record vocabulary: `tasks`
  and `goal_done`.
- `docs/recaps/` — the proof.

## Risks

- **The model never calls `todo`.** The pane is a view over the model's
  own declarations; an empty card is the honest failure, the prompt
  nudge (C7) is the mitigation, and the face never fabricates rows from
  tool calls.
- **The list goes stale.** Statuses are the model's claims, not
  verified facts; the card presents them as declared (the fold block's
  own honesty, carried to the face).
- **Record-count assertions shift.** New mid-run records change
  `records.len()` in replay tests (the eval-count scar); each affected
  count is updated deliberately with the reason named.
- **House CSS collisions.** An imported stylesheet owns `.field`,
  `.pane`, `.rail` — the mockup and the card prefix their classes
  rather than reusing those names.
- **The pane's width on small screens.** The pane is owner-resizable
  (300–720px); the card's rows must ellipsize, not overflow — the
  mockup measures at both ends of the grip.
