---
status: draft
created: 2026-11-03
updated: 2026-11-03
slug: composer-actions-apply-on-submit
---

# Plan: every composer action applies when it is submitted

## Context

The owner reported two defects and one principle: *a delayed action is the
equivalent of an error in the eyes of users.*

1. **Stop does nothing** during a run.
2. **The execution mode** does not respond when changed mid-conversation.

The audit that followed (`docs/recaps/`, this plan's linked artifacts) found
both, and found they are one mechanism with several symptoms.

**The mechanism.** The agent loop reads the run's control file at exactly
two points — before each model call (`src/agent.rs:2551`) and inside the
pause loop (`:2944`). Every in-flight control — stop, steer, pause, fold,
pin — is written to that file by the bridge (`src/gui/bridge.rs:4140`,
`:3930`, `:4174`). A run waiting on an approval is blocked in
`GuiApprover::approve` → `decision.recv()` (`src/gui/bridge.rs:10004`),
which is at neither point.

So while a seat is pending, **all six controls hang** — and the seat is the
one thing the owner must answer. The stop was proven live: the server
accepted it (200), the card said *"stopping · the run ends at its next
step"*, `running` stayed `true` for minutes, and the run ended only after
the pending seat was answered through the API.

**A second, compounding defect.** On a fresh page load the conversation
log renders empty and the seat stays hidden with zero buttons, even though
`/api/state` carries 50–1098 events for the session and a live `approval`
object. The live SSE path paints them; the replay path does not. A reload
therefore strands a run: the seat the owner needs is gone, and the stop
cannot reach it.

**A third.** The run-scoped choices — mode, provider pin, plan mode, turn
budget, goal — apply to the **next** run, and only the folder says so. The
mode is the one the owner noticed: a run that started in `ask` keeps
asking after the mode is set to `auto`, because the approver's gate is
built once per run (`src/gui/bridge.rs:10378`).

## Approach

**Fix the mechanism, then the wording.** One change makes every in-flight
control able to interrupt the approval wait; one makes a reload honest;
one makes the run-scoped choices say what they are.

The increments, each independently valuable:

1. **F1 — a control interrupts the approval wait.** The bridge already
   owns the seat; when it writes a control for the driving session, it
   also resolves a pending seat so the agent thread can reach its next
   boundary. A stop must never wait for an answer the owner has replaced
   with a decision.
2. **F2 — the replay path paints what the state carries.** The log and the
   seat must render on a fresh load exactly as they do live, so a reload
   never strands a run.
3. **F3 — every run-scoped control names its scope.** The mode, the
   provider pin, plan mode and the turn budget say "next run" the way the
   folder already does — or become live, if the owner rules they should.
4. **F4 — `/compact`'s description matches its behaviour.** It promises
   "now"; it means the next turn boundary.
5. **F5 — the queue rows survive a repaint.** `paintQueue` rebuilds every
   row from `textContent = ''` on each call, and `paint()` calls it on
   every event. During a run the rows are destroyed and recreated under
   the cursor, so a click that spans a rebuild never fires — the send,
   edit and drop buttons do nothing. The rows must rebuild only when the
   queue actually changed, the way `paintSeat` already keys its buttons.

The docs move with the increments: `docs/features/gui.md` and the
installation's run-policy wording land with F3, not in a finale.

## Use cases

- **The owner who changes their mind.** A run is asking; the owner presses
  stop. The run ends now — no seat to answer first.
- **The owner who steers.** The owner types a correction; it rides the
  run's next request, and the seat it superseded is named as superseded.
- **The owner who reloads.** A reload shows the conversation and any
  waiting seat, exactly as it was.
- **The owner who sets a mode.** The control says whether it applies now
  or to the next run, and does what it says.

## Acceptance criteria

- [ ] **C0 — a stop ends the run with a seat pending.** With a seat
      waiting, `POST /api/stop` ends the run without the seat being
      answered: the run's `finished`/`failed` record lands, `running`
      clears, and the seat is resolved and named. Pinned by a test that
      raises a seat, writes a stop, and asserts the run ends with no
      answer sent.
- [ ] **C1 — the other controls still apply, and are not lost.** A steer,
      pause, fold or pin written while a seat waits is applied at the next
      boundary after the seat is answered, and is not silently dropped.
      Pinned by a test per control.
- [ ] **C2 — a reload is honest.** On a fresh page load with a state
      document carrying events and a standing `approval`, the conversation
      paints its transcript and the seat renders its decisions. Pinned by
      a face-harness case over a state document with both.
- [ ] **C3 — a run-scoped control says its scope.** The mode, provider
      pin, plan mode and turn budget each name whether they apply now or
      to the next run, and the behaviour matches the words. Pinned by the
      face harness over each control's title and the run request it
      produces.
- [ ] **C4 — `/compact` tells the truth.** Its description names the turn
      boundary it folds at. Pinned by the command-help text.
- [ ] **C5 — the queue's buttons survive a repaint.** With rows queued
      and events arriving, a click on send, edit or drop lands and takes
      effect. Pinned by a face-harness case that queues rows, drives
      events through the reducer, and asserts the row count and the
      composer after each press.
- [ ] **C6 — the gate is green.** fmt, clippy `-D warnings`, release
      build, and the suite, each read by exit code (KI-014).

## Files to be touched

| File | Change |
|---|---|
| `src/gui/bridge.rs` | F1: resolve a pending seat when a control is written for the driving session; F3: the mode's scope, if it goes live |
| `src/agent.rs` | F1: the seat-resolution path names the control that ended the wait |
| `src/gui/assets/app.js` | F2: the replay path paints the seat; F3: each run-scoped control names its scope; F4: `/compact`'s description |
| `tests/face/harness.mjs` | C2/C3/C5's pins |
| `tests/gui_sse.rs` or `tests/integration.rs` | C0/C1's pins over the real bridge |
| `docs/features/gui.md` | F3's wording, with the increment |

## Out of scope

- **Making the run-scoped choices live.** F3's default is to *name* the
  scope, not change it. Going live is a one-line follow-up if the owner
  rules for it, and it changes what a "run" means.
- **The seat's design.** When to ask, and what the destructive class is,
  is the approvals plan's ground.
- **The native shell.** The Tauri window is a separate plan; the face's
  behaviour is identical in both.

## Verification

- **The gate, whole:** `cargo fmt --all --check`, `cargo clippy
  --all-targets -- -D warnings`, `cargo build --release`, `cargo test`,
  each read by exit code.
- **A live stop with a seat pending:** raise a seat, press stop, and
  assert the run ends without an answer.
- **A live reload:** with a seat waiting, reload the page and assert the
  transcript and the seat render.
- **Each run-scoped control:** change it and read what the next run
  request carries.

## Linked artifacts

- The audit that found this: the stop read sites (`src/agent.rs:2551`,
  `:2944`), the seat wait (`src/gui/bridge.rs:10004`), the control writers
  (`src/gui/bridge.rs:4140`, `:3930`, `:4174`), the mode capture
  (`src/gui/bridge.rs:10378`).
- The approvals model: `src/approvals.rs` (the mode enum, the gate, the
  destructive class).
- The seat's contract: `src/agent.rs:996` (`Approval`), `:1010`
  (`ApprovalRequest`).

## Risks

- **Resolving a seat is deciding a call.** F1 must name *why* the seat
  closed — a stop is not a denial — or the transcript lies. The
  `AnsweredBy` seam already carries who answered; the same shape carries
  the control that did.
- **A stop that ends a run mid-tool.** The run may be between a model call
  and a tool result. The control is read at the next boundary, so the
  tool's result is already in hand; the run ends without spending another
  token.
- **F2 touches the replay path**, which the density and fold work depend
  on. The pin must cover a state document with events, folds and a seat,
  not just a seat.

## Dependencies

- Rust 1.99.1, and the gate's four legs.
- The `AnsweredBy` seam (`src/gui/bridge.rs`), which already names who
  closed a seat.

## Notes

- **The principle this plan encodes:** a control that cannot act now must
  say when it will act. The stop was the worst case because it said
  "at its next step" and there was no next step to reach.
- **The cheapest fix would be to deny the seat on every control.** It is
  also wrong: pause, fold and pin mean the run should continue, and
  denying the call it was asking about is a decision the owner did not
  make. F1 resolves the seat only where the control ends the wait.
