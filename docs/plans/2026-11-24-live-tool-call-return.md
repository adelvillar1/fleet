---
status: completed
created: 2026-11-24
plan: live-tool-call-return
---

# A tool call in flight is not interrupted because the page looked away

The owner's report (2026-11-24): *"when a bash command is running in a
conversation and you switch to a different page and go back the bash command
shows as interrupted."*

## The mechanism

Arriving at a conversation runs `openWork`, which reposts `/api/resume` and
then `load()` → `replay(state)`. The replay rebuilds the transcript from the
log's events, and the log's **last** record while a call is running is the
`tool_call` with no `tool_result` yet.

The replay path carried an unpaired-call sweep (KI-485) that resolved every
such row as `interrupted` — correct for its own case, which is a log cut off
by a stop, a crash or a face restart, and explicitly "a run that exists only
because the page loaded". It ran **unconditionally**, so it also fired on the
call whose run is still alive.

Two things made it read as a lasting defect rather than a flicker:

- **Visibility.** `summarizeTurn` hides every finished tool row of a folded
  turn (`el.hidden = true`), and a fresh visit clears `face.turnFolded`
  (`replay` re-seeds it per conversation). So on return the in-flight call is
  the *only* tool row still visible, and it is the one wearing the wrong mark.
- **The process card.** The same replay set `face.process.stop =
  'interrupted'` and `'the run was interrupted'` whenever
  `face.process.active` was locally true, regardless of the server's own
  `running` flag.

The fact needed was already on the wire: `state.running` is the bridge's flag,
set at dispatch (`bridge.rs:7037`) and cleared when the run ends, so it
answers "is this call alive" without the page guessing.

## Acceptance criteria

- [x] C1. The replay's unpaired-call sweep runs only when the run is not live
      (`state.running` false), so a live call keeps its row running and the
      live `tool_result` closes it.
- [x] C2. The process card's interrupted closure carries the same gate: a
      live run never has its card closed by a re-read.
- [x] C3. KI-485's own case still holds: a log cut off at a `tool_call` with
      `running: false` still settles `interrupted`, on both the row and the
      card.
- [x] C4. A face-harness case drives the real path twice — open the
      conversation with a call in flight, leave for another surface, return —
      and asserts the row reads running both times and never `interrupted`;
      it must fail against the pre-fix `app.js`.
- [x] C5. The full gate: fmt, clippy `-D warnings`, release build, `cargo
      test`, the face harness (Node 26), `doc_truth.py`, `ki_ledger.py`,
      `plan_check.py` — all clean.

## Rows

- **R1 — the gate.** `app.js`'s replay sweep and process card. C1, C2.
- **R2 — the twin.** The harness case beside KI-485's, proving both verdicts
  from the same shape. C3, C4.
- **R3 — the gate and the ledger.** C5, plus KI-596 for the byte-slice trap
  found while writing the doc checker.

## Note

The cheap fix — dropping the sweep — would bring back KI-485 in full: rows
ticking "running · 1700s" against runs that no longer exist. The correct fix
is the gate, because the two cases differ by exactly one fact and that fact
is already in the state document.
