---
status: completed
created: 2026-10-23
updated: 2026-10-23
slug: conversation-control
---

# Conversation control: goal, steer, btw, and the run's own dials

## Context

The owner's ruling: controlling the conversation is primordial; not
being able to adjust a running session is a deal breaker. A survey of
what exists (read from the code) found the pieces and the gaps:

- **Exists.** The control file the agent reads before every model call
  (`Control::Steer(text)` | `Control::Stop`); the face posts steers
  (`POST /api/steer`); Stop is a composer control (`29d1fed`); approvals
  answer once/always/remember; permissions, the provider pin, the
  working folder, and plan mode ride the composer; rewind and `/review`
  ride the checkpoints; compaction is automatic; `runs.max_turns` is a
  knob (a receipt, not a dead stop).
- **Gaps.** No way to queue a message while a run executes; `/compact`
  is dead in the face (the forced fold is read once, at run start); the
  provider pin applies to the next run only, so a rate-limiting provider
  cannot be switched mid-session; no pause; sessions have no names; the
  ↩ rewind shows on turns of a live run where the bridge refuses it; and
  Stop is graceful only — a tool call in flight finishes first.

## Approach

**One control vocabulary, read per turn.** The control file grows from
two verbs to five, all read at the same seam before each model call:
`Steer(text)` (exists), `Stop` (exists), `Fold` (force the context fold
at the next boundary), `Pin(text)` (re-route the running session for
its next turn — a provider id, `provider/model`, or `auto`), and
`Pause` (freeze before the next model call until the owner resumes).
The face posts each through one route (`POST /api/control`), and the
process card says which is in effect.

**The three commands**, in the composer's existing `COMMANDS` table:

- `/goal <text>` — the session's standing objective. Persisted in
  `<home>/sessions-goals.json` (the `sessions-workdirs.json` pattern),
  pinned in the log as a `goal` record (the prompt carries it — hard
  rule 5), and carried by every run as a `## Goal` prompt section; a
  chip in the run bar and one transcript row per change.
- `/steer <text>` — posts the existing steer path; refused by name with
  no run in flight, never queued into the next.
- `/btw <question>` — one tool-less model call through the session's
  router, landing a `btw` transcript row (question, answer, the cost
  line). The run's log gains nothing from it.

**The queue.** While a run is in flight the composer accepts text and
Enter appends it to an **ordered queue** above the composer. The owner
decides the order: a row drags to a new position (pointer drag), and
**send first** promotes one row to the head so it is read first; a ×
drops it. The flush sends the head as the next run, and when that run
ends the next row follows — the queue drains one message per run, in
the owner's order. Client-side state; a reload drops it, and the chip
says so.

**The smaller two.** Sessions get a title: `/rename <title>` writes
`<home>/sessions-titles.json`, the archive's rows show the title, and a
session with none shows its first owner message's first line (read
locally from the log — no model call). The ↩ rewind is rendered only on
turns whose run has ended, so the affordance never appears where the
bridge will refuse it; the stop's graceful boundary is stated where the
owner meets it (the stop button's title and the process card's line).

The mockups — the voice to approve before a line is built:

```
the transcript, after /goal ship the picker before Thursday
◇ goal  ship the picker before Thursday
        carried by every run in this session · /goal clear to drop it

the transcript, after /btw what does the picker's row action do?
? btw  what does the picker's row action do?
        "The row's open action opens the session in the conversation…"
        a side ask · one model call · not part of the run

the composer, with a run in flight and two queued messages
 ┌ queued · 2 · sends when the run ends ──────────────────────┐
 │ ⣿ then check the gate            [send first] [×]         │
 │ ⣿ and update the plan            [send first] [×]         │
 └────────────────────────────────────────────────────────────┘
 Ask fleet for something…                                  [Stop]

the process card, while a control is in effect
 · pinned to stepfun/step-5-preview for the next turn
 · paused by you · [resume]
```

## Acceptance criteria

- [ ] **C0 — the goal is standing and logged.** `POST /api/goal {text}` sets it (empty clears); `<home>/sessions-goals.json` survives a face restart; each run declares the session's goal from the sidecar and pins a `goal` record in its log; the system prompt carries `## Goal` with the text while set. A unit test pins the prompt with and without a goal, and the first-message early return holds only when no goal is set.
- [ ] **C1 — steer is the composer's too.** `/steer <text>` posts the control route with the run's session; with no run in flight it is refused by name and writes no control file (bridge test); with a run in flight the text reaches the file the agent reads (the existing `apply_steer` path).
- [ ] **C2 — btw answers beside the run.** `POST /api/btw {text}` returns the question and the answer from a one-shot, tool-less call through the session's prepared router (preparing one when none); the transcript gains a `btw` row with the cost line; the session's log gains no message or turn from the exchange (a bridge test asserts the record count and the text's absence). It works whether or not a run is in flight.
- [ ] **C3 — the queue is a list the owner orders.** With a run in flight, Enter appends the text to an ordered queue above the composer; each row renders its text with a drag handle, **send first**, and a cancel. A drag reorders the rows (pointer drag), and **send first** promotes a row to the head. The run's `finished`/`failed` event flushes the head as the next run in the same session, and the next row follows when that run ends. A face test pins the order (append, promote, drag-reorder) and the flush; the chip says a reload drops the queue.
- [ ] **C4 — fold is real.** `/compact` with a run in flight writes `Control::Fold`; the turn loop reads it and forces the fold at the next boundary (a unit test on the control's parse and the loop's fold call). With no run in flight the command says honestly that there is nothing to fold yet.
- [ ] **C5 — the pin reaches the running session.** `Control::Pin(text)` re-prepares the route for the running session's next turn — a provider id, a `provider/model` pair, or `auto` — and the swapped route is what the next turn's request crosses (no new run needed). The row's reason names the re-route; an unusable pin is refused by name and the run continues on its route. Pinned by a unit test on the control and an integration test whose second turn serves on the pinned provider.
- [ ] **C6 — pause freezes and resume continues.** `Control::Pause` blocks the turn loop before the next model call; the face shows `paused by you` with a resume control; resume writes `Control::Resume` and the run continues from the same turn, its log gaining no message from the pause. A unit test pins the parse and the loop's wait.
- [ ] **C7 — sessions have names.** `/rename <title>` writes `<home>/sessions-titles.json`; the archive rows render the title; a session with none renders its first owner message's first line, clipped (read from the log, no model call). Pinned by a test over a log with and without a title.
- [ ] **C8 — the two fidelity fixes.** The ↩ rewind renders only on turns of ended runs (a face test on the live-turn row); the stop button's title and the process card's line state the graceful boundary ("ends at the next step").
- [ ] **C9 — discoverable and documented.** All commands are in `COMMANDS` with descriptions the palette ranks and `help` lists; the functional spec's conversation surface names the commands and the run controls; the graceful-stop boundary is named there too; the gate is green.

## Files to be touched

`src/agent.rs` (the control vocabulary, the fold/pin/pause handling, the
`## Goal` section, `AgentOptions.goal`), `src/gui/server.rs` (the goal,
btw, and control routes), `src/gui/bridge.rs` (goal/pin/fold/pause
writes, the btw call, the queue's none — it is client-side),
`src/settings.rs` (the goal and title writers), `src/boards.rs` (the
archive's title and first-line), `src/gui/assets/app.js` (the commands,
the queue, the chip, the pause control, the rename), `index.html` +
`app.css` (the queue chip, the pause control, the goal chip),
`src/route.rs` (a re-prepare for a running session), the functional
spec, and `docs/features/gui.md`.

## Out of scope

Killing a bash child mid-call (the boundary is documented: the stop
lands at the next model call). Auto-titling by model call. The queue
surviving a reload. The CLI's and the TUI's own forms of these commands
(the face first; the TUI's Ctrl-R steer stays as it is). `/btw` with a
history — one question, one answer, one row.

## Verification

The unit, bridge, and face tests named per criterion; the gate; and a
live pass on the running face walking C0, C2, C3, C5, and C6 in order —
the goal in a real run's prompt, a btw absent from the run's log, a
queued message starting the next run, a mid-run pin serving the next
turn, and a pause that resumes into the same turn.

## Linked artifacts

- `src/agent.rs`'s control-file vocabulary — the seam every run control rides.
- `29d1fed` — the composer's Stop, the sibling this plan completes.
- The run-control notes' open item: "a side conversation with the orchestrator" — `/btw`.
