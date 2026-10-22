---
status: active
created: 2026-10-22
updated: 2026-10-22
slug: chat-ux-polish
---

# Plan: the conversation earns its scroll — chat UX polish

## Context

The owner's ruling (2026-10-22): the whole conversation/chat UX needs a
lot of polish — too much real estate is being misused with cards showing
commands and checkpoints, the 25-turn limit is very annoying, and the
models complete work without providing results or actionable requests.
The ask: deep research plus UI expertise against the common patterns of
today's LLM chat applications, then a contract for the fix.

Evidence in this session's code read:

- Every event type renders its own full-width transcript card. The
  dispatcher (app.js, the event switch) gives routing, memory recall,
  skills loading, lanes, cron, compaction, checkpoints, folder changes,
  repair notices, dropped events, and plans one card each — the same
  visual weight as a tool call, which is the actual work. A routine run
  opens with three plumbing cards (route, memory, skills) before the
  first real tool fires.
- Checkpoint cards carry a rewind button and two lines, per checkpoint,
  for safety plumbing the owner touches rarely.
- The turn cap is a compiled constant: default max turns 25 at
  src/agent.rs line 113. A run that hits it dies with an end card; the
  work stops mid-task and the owner must manually re-prompt with no
  carried state. Compaction exists for context, not for turns.
- The end-of-run card renders how the run stopped, not what it
  produced. The reporting contract in the system prompt demands a
  result and a next step in prose, but nothing structural carries it:
  no artifacts, no links, no suggested ask. The repair beat (the
  harness asking for an owed report) already exists as an event — the
  principle is in the code; the surface does not cash it.

Research grounding (this session): the beUI agent-interface catalog
(tool results expand while running and collapse to a compact completed
state; loading is informative not decorative; approvals are distinct
affordances); a production pattern library of agent control surfaces
(approvals, receipts, activity logs, rollback hooks — hatchworks,
March 2026); product mechanics (sub-agent steps nest under one agent
row in xpander; Devin's runs continue until the deliverable exists;
Claude/ChatGPT collapse long tool activity into one summary line with
a timer that expands on click). The common pattern across all of them:
the transcript carries conversation and work; everything else is
chrome, collapsed by default, opened on demand.

## Approach

One principle drives every decision: the transcript is for the work
and the words; the harness tells its story elsewhere. Six decisions:

**D1 — an event hierarchy ends card-per-event.** Transcript-level
cards survive for exactly: user messages, assistant prose, tool
activity (D2), approval seats, and the end receipt (D5). Everything
else — routing, memory recall, skills loading, lanes, cron,
compaction, folder changes, repair, dropped — collapses into a single
per-run status line beneath the run's header: one compact strip
(served-by · facts recalled · skills loaded · lanes joined), one
click to expand the full sequence as a plain list. The events remain
logged and replayable; only their transcript weight changes. A run's
plumbing goes from N cards to one line.

**D2 — tool rows follow the expand-while-running, collapse-when-done
pattern.** A running tool shows verb, target, and live seconds; on
completion it collapses to one line (tool · target · outcome dot),
with its output preview one click away. Refused calls and seats stay
expanded — the owner's decision should never hide. Checkpoints stop
being cards entirely (D3).

**D3 — rewind becomes a row affordance, not a card.** Checkpoint
state moves into the transcript rows themselves: any turn's hover or
context affordance offers rewind-to-here, and the run's status strip
names the last checkpoint. Zero checkpoint cards; the safety affordance
gains coverage (every turn, not every Nth) while losing the footprint.

**D4 — the turn cap becomes a budget the owner sees and owns.** The
compiled 25 becomes a settings knob (default raised; per-run override
on the composer), the run bar shows remaining turns while a run is
live, and hitting the budget produces a continuation receipt: what
was done, what remains, and a continue control that resumes the same
session with its state — the model's own hand-off, not a silent wall.
Stopping stays user-owned (the stop button); the wall stops being
surprising.

**D5 — every completion ends in a receipt with an ask.** The finished
card becomes structured: outcome line, artifacts (files touched,
deliverables — linking through the file addresses once the face's
file-address gap closes, KI-289), verification evidence (tests run,
gates green), and the model's own actionable next request rendered as
a one-click composer fill. The harness side: the existing repair beat
extends to any finish that carries no result text — the report is
requested, not assumed — so the receipt always has something true to
render.

**D6 — the reporting contract gains teeth at both ends.** The system
prompt's contract stays; the face enforces it visually (the receipt)
and the run enforces it procedurally (the repair beat). A run that
cannot state a result cannot end silently.

## Increments

Each increment closes green — the four-command local gate in full,
each exit code read on the cargo invocation itself — and commits with
explicit paths before the next begins.

1. **The status strip.** Re-route the plumbing events out of the
   transcript into the per-run status line with expand; the card
   functions remain for the log replay view. Ship behind nothing —
   the face is loopback-local, verify live with screenshots. Gate;
   commit.
2. **Tool rows and rewind.** The tool-row lifecycle (running expands,
   done collapses, refused stays) and checkpoint-to-row-affordance
   with the rewind picker on turn rows. Gate; commit.
3. **The turn budget.** The settings knob, the run-bar budget display,
   the continuation receipt, and resume-from-receipt. Gate; commit.
4. **The end receipt.** The structured finish card (outcome,
   artifacts, verification, ask-with-fill), the repair-beat extension
   for result-less finishes, and artifact links wired to file
   addresses when that lands. Gate; commit.

## Acceptance criteria

- [ ] C1 — a fresh run's transcript shows at most one plumbing line
  per run (the status strip); routing, memory, skills, lanes, cron,
  compaction, folder, repair, and dropped events never render their
  own cards in the live conversation, and the strip expands to the
  full event sequence on click.
- [ ] C2 — a tool row shows verb, target, and live elapsed seconds
  while running; on approval it collapses to a single line with an
  outcome dot and the output preview one click away; a refused call
  stays expanded with its reason.
- [ ] C3 — no checkpoint cards render in the transcript; rewind is
  reachable from any turn's affordance and from the status strip's
  last-checkpoint line, and completing a rewind lands the same state
  the old card flow produced.
- [ ] C4 — the turn budget is a settings knob with a raised default,
  the run bar shows remaining turns during a run, and reaching the
  budget renders a continuation receipt whose continue control resumes
  the same session with its state carried.
- [ ] C5 — every finished run ends in a structured receipt: outcome,
  artifacts, verification evidence, and a next-request affordance
  that fills the composer on click; a finish with no result text
  triggers the repair beat and the receipt renders the recovered
  report.
- [ ] C6 — the replayed log (sessions surface) still shows every
  event: the hierarchy changes transcript weight, not the record, and
  a replayed session reconstructs what the model saw.
- [ ] C7 — the eleven existing surfaces are unchanged beyond the
  conversation panel; the gate is green in full, exit codes read on
  the cargo invocations.

## Files to be touched

- src/gui/assets/app.js — the event switch, the status strip, the
  tool-row lifecycle, the checkpoint affordance, the receipt, the
  budget display
- src/gui/assets/app.css — strip, row, and receipt styles; the
  collapse states
- src/gui/bridge.rs — the continuation receipt's session data, the
  repair-beat extension for result-less finishes, the budget knob in
  the settings document
- src/agent.rs — the turn cap read from settings, the continuation
  hand-off (resume with carried state), the finish event's structured
  fields (outcome, artifacts, verification, ask)
- src/settings.rs — the turn-budget knob
- docs/features/gui.md — the conversation surface's new contract
- FUNCTIONAL-SPECIFICATIONS.md, TECHNICAL-DOCUMENTATION.md — the
  surface and the run behavior

## Out of scope

- A second conversation layout (multi-pane inbox threads): the
  bot-first shell is the bot-PWA plan's contract, separate.
- Message editing/branching of past user turns: not named in the
  ruling; a later plan if wanted.
- Changing what is logged: the record keeps every event; only the
  transcript's rendering hierarchy changes.
- Model selection, routing, or the approval gate's mechanics: they
  render differently, nothing about their behavior moves.

## Verification

- C1/C3: live run against the face with screenshots before/after;
  count cards in the transcript for a scripted run; expand the strip.
- C2: a scripted multi-tool run; each row's three states
  (running/collapsed/refused) screenshotted.
- C4: set the budget to a small number in settings, run a task that
  exceeds it, read the receipt, press continue, and verify the session
  resumed with state.
- C5: a normal finish and a forced result-less finish (prompt crafted
  to end without a report); the receipt and the repair beat each
  verified live.
- C6: open a pre-change session from the archive; every event type
  still present in the replay view.
- C7: the gate in full; a pass over the other ten surfaces.

## Linked artifacts

- docs/features/gui.md — the conversation's contract, rewritten
- FUNCTIONAL-SPECIFICATIONS.md — the conversation surface section
- TECHNICAL-DOCUMENTATION.md — the bridge's event vocabulary and the
  run behavior section
- docs/known-issues.md — close the KI-289 dependency note if the file
  address lands first

## Risks

- **Replay fidelity**: the transcript hierarchy must not eat events —
  C6 is the guard, and the log format does not change.
- **The continuation receipt's resume path**: resuming with carried
  state touches the run/session seam; smallest version is a new user
  turn seeded with the hand-off text, which is honest and cheap.
- **Scope pull**: this plan is polish, not the bot shell; the
  bot-PWA plan owns the second surface.

## Dependencies

The receipt's artifact links depend on the face's file address
(KI-289, proposal face-file-address-deep-link) landing first or
together; everything else is independent. The bot-PWA plan shares the
conversation's event vocabulary but no rendering surface.
