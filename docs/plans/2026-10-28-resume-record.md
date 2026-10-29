---
status: completed
created: 2026-10-28
updated: 2026-10-29
slug: resume-record
---

# Plan: an interrupted session continued by the owner is a resumed session, not a terminated one

## Context

The owner's ruling (2026-10-28, this session): a conversation that was
handed back and then continued must not read as terminated — and the
same session on the archive must not read as interrupted while it is
being actively continued. The owner pointed at this very session as
the live example: two `end` records sit mid-log (each handback writes
one) with thirteen newer records after the last, yet every surface
still reads the conversation as over.

Ground truth verified by code read this session:

- `session::summarize` reads **only the log's last line** to derive
  `ended` — so between turns the last line is the latest handback's
  `end`, and every archive row, replay payload, and CLI listing reads
  "ended" from a shape that cannot say "continued".
- `gui::bridge::work_state` then maps `ended: true` to
  `handed back`/`completed` — the terminated word the owner saw.
- `Session::open` opens a log for append and records nothing, so the
  continuation itself is invisible: no record ever marks the resume.
- The session format is JSONL with a `#[serde(tag = "type")]` record
  enum; `end` records carry `stop_reason`/`turns`/`served_model`; the
  established term for this concept is **resume** (11 sites).
- The bot face renders the server's state words verbatim ("it invents
  no second state machine"), so a new server word needs no bot change;
  the app face's `WORK_STATE_WORD` map is the one JS surface to extend.

## Approach

1. **Core (src/session.rs)**: a `resume` record (`at` stamp),
   written by `Session::open` when the log's last record is `end` —
   the one seam every continuation flows through. `summarize` folds
   the whole file it already reads: `ended` is true only when the
   **last** record is `end`; any later record (including `resume`)
   clears it and sets the new `resumed: true` field. `stop_reason` and
   `turns` read from the last `end` but are cleared when `!ended`,
   matching the field's own contract.
2. **Surfaces (src/gui/bridge.rs, src/gui/assets/app.js)**:
   `work_state` speaks `resumed` for a resumed session (with the live
   word `working` winning while a run is actually in flight in it);
   the replay payload carries `resumed` and its `ended` becomes
   truthful; the app face's state-word map and the replay drawer's
   summary line say "resumed" instead of "ended"/"interrupted".
3. **Proof through the real paths**: a session log that ends and is
   reopened must summarize not-ended and resumed; `work_state` tests
   pin the new word and the driving-override; a bridge replay test
   reads a resumed session through `Bridge::replay`.
4. **Log honestly**: the defect files as KI-354 with this session's
   evidence and closes fixed in the same write. (KI-354 was taken by
   the sem1 llama-server instance between this plan's draft and its
   close-out; the row landed as **KI-365** — same intent, the next
   free number.)

## Acceptance criteria

- [x] C1: A log whose last record is `end`, once opened again, carries
      a `resume` record and summarizes `ended: false, resumed: true`;
      a start-only log is not stamped; the summary's `stop_reason`/
      `turns` clear when `ended` is false. Pinned by
      `a_continued_session_is_resumed_not_ended`.
- [x] C2: `work_state` returns `resumed` for a resumed, idle session
      and `working` for a resumed one with a run in flight; the replay
      payload reports `resumed` truthfully; the app face maps the word.
      Pinned by `a_resumed_session_is_not_terminated`.
- [x] C3: the gate exits 0 with exit codes read directly (fmt 0,
      clippy 0, build 0, test 0 — 2026-10-29); KI-365 filed and closed
      with the evidence (KI-354 taken); the plan's status flipped in
      the same turn.

## Files to be touched

- `src/session.rs` — the record, the stamp, the fold, the tests
- `src/gui/bridge.rs` — `work_state`, the replay fold, the tests
- `src/gui/assets/app.js` — the state-word map, the drawer line
- `docs/known-issues.md` — KI-354

## Out of scope

- Reworking `serving_model`'s KI-073 rule for resumed sessions.
- The CLI `fleet sessions` listing's column set (it renders whatever
  the summary carries; no new column this slice).
- Purging or rewriting any existing log.

## Verification

C1/C2 by the new tests in the gate; C3 by the gate's exit codes read
directly and the ledger row's evidence block.

## The declaration

```graph
{
  "nodes": [
    {
      "approach": "Add the resume record to the session format, stamp it in Session::open when the log's last record is end, fold ended/resumed truthfully in summarize, and pin it with tests through Session::create/open/summarize",
      "cost": 2,
      "id": "resume-core",
      "paths": [
        "src/session.rs"
      ],
      "serves": "C1"
    },
    {
      "approach": "work_state speaks resumed (working wins while a run drives the session), the replay payload carries resumed with a truthful ended, the app face maps the word and its drawer line says resumed",
      "cost": 2,
      "id": "resume-surfaces",
      "paths": [
        "src/gui/bridge.rs",
        "src/gui/assets/app.js"
      ],
      "prereqs": [
        "resume-core"
      ],
      "serves": "C2"
    },
    {
      "approach": "Run the full local gate with the toolchain PATH prefix, exit codes read on cargo itself; file and close KI-354 with this session's evidence; set the plan's status in the same turn",
      "cost": 2,
      "id": "resume-verify",
      "paths": [
        "docs/known-issues.md"
      ],
      "prereqs": [
        "resume-core",
        "resume-surfaces"
      ],
      "serves": "C3"
    }
  ]
}
```
