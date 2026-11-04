---
name: the-approval-wait-blocks-every-run-control
description: "project 2026-11-03 — the face's run controls (stop, steer, pause, fold, pin, resume) all wrote one control file that the agent loop read only before a model call, so while an approval seat was pending NONE of them could act — a stop was accepted, the card said "stopping", and the run hung until the seat was answered (proven live); FIXED in F1 (b2a4f9f) — the approval wait polls the control file without consuming it, and the bridge resolves the seat, naming the stop as the reason"
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

Reproduced live on 2026-11-03 against `fleet gui` on the owner's home,
driven in a real browser (the in-app browser over the served page):

1. A run in `mode:"ask"` raised an approval seat (`tool_search`).
2. Clicking **stop** → `/api/stop` returned **200**, the card said
   *"stopping · the run ends at its next step"*, and `running` stayed
   `true` for minutes.
3. Answering the pending seat (`POST /api/approve {"decision":"once"}`)
   → the run **immediately** read the stop and ended:
   `stop_reason: "stopped by the owner"`, `running: false`.

So the stop was queued correctly all along; the loop never got far enough
to read it.

**The mechanism (the record of why).** Every run control wrote the same
one-shot file (`<home>/sessions/<id>.control`, via
`delegate::write_control` / `write_control_value`), and the agent loop
read it at exactly **two** points — `src/agent.rs:2551` (before each
model call) and `:2944` (inside the pause loop). A run waiting on an
approval was blocked in `GuiApprover::approve` → `decision.recv()` — at
**neither** point. Therefore, while a seat was pending, all six composer
run controls were deferred indefinitely: stop (`/api/stop`), steer
(`/api/steer`), pause, fold (`/compact`), pin, resume (all
`/api/control`).

## FIXED — F1 of the composer-actions plan (b2a4f9f)

The design that landed, per the plan's own ruling that the cheapest fix
(denying the seat on every control) is wrong — pause/fold/pin mean the
run should continue, and denying the call it was asking about is a
decision the owner did not make:

- **The approval wait polls the control file without consuming it.**
  `agent::control_file_stops(path)` (a non-consuming peek over
  `parse_control`) is checked by `GuiApprover::approve` **before raising
  a seat** and inside its wait loop, which replaced the single blocking
  `decision.recv()` with a `try_recv` + 120 ms sleep poll. Seeing a stop
  ends the wait with the call refused — and the file is deliberately
  left in place, because **the loop's own control read is what stops the
  run; consuming it in the poll would eat the stop and leave a run that
  refuses one call and carries on.**
- **The bridge resolves the seat and names the stop.** `stop()` takes the
  pending seat, drops its responder (the wait ends in a refusal either
  way), and broadcasts an `answered` event with
  `by: "the stop you pressed"`, `decision: "deny"` and a `reason` field —
  **a stop is not the owner declining the call**, so the transcript names
  the stop rather than a denial. `GuiApprover::denial_reason` returns
  *"denied: the run was stopped by the owner."* when the refusal came
  from the control file.
- **Multi-call turns are covered**: after the first call settles as
  refused, every later `approve` re-checks the file and refuses without
  raising a new seat, so the run cannot re-strand on call two.
- Pause, fold, pin and **steer keep the seat standing** (C1's pin): the
  owner still owes that answer, and the written control is applied at the
  next boundary after it.

Pinned by `a_stop_resolves_a_waiting_seat_and_names_itself_the_reason`
and `the_controls_that_continue_are_not_lost_while_a_seat_waits` over the
real bridge. Face harness CA2/CA2b/CA5 cover the surface halves.

## The two sibling findings from the same audit

- **The mode is captured once per run** (`bridge.rs` `run_with` → the
  approver at run start). F3 landed the *wording* — the mode and the
  provider pin say "applies to the next run" where they paint. **Going
  live** (re-reading the mode per call) remains a one-line follow-up the
  owner has not ruled for. Remembered rules are still consulted before
  the mode, so a rule-covered call never asks under any mode.
- **The fresh-load empty log was NOT the replay path.** The harness
  proves `replay()` paints events and a standing approval from a state
  document (CA2). The live symptom's root cause was `/api/resume`
  **refusing a session with a run in flight** and `openWork` treating
  that refusal as "don't load" — no state read, empty log, hidden seat,
  pill "idle". F2 fixed it: the refusal is now the cue to load the view
  anyway. See [[composer-actions-plan-drafted-2026-11-03]].

Related: [[a-deferred-action-is-an-error-to-the-owner]] (the ruling this
was found under), [[an-effect-with-no-visible-trace-reads-as-broken]],
[[the-queue-rows-rebuild-under-the-cursor]] (F5, the sibling fix),
[[gui-client-verified-by-a-vm-harness]].
