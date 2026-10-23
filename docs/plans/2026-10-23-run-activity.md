---
status: completed
created: 2026-10-23
updated: 2026-10-23
slug: run-activity
---

# Run activity: the thought and the command, one line each

## Context

The owner's reference: ZCode's activity timeline — one quiet line per
thing that happened ("Ran a command", "Thought · 5 seconds"), each
opening on demand, so the transcript stays readable while everything
that ran is one click away. The ask extends it to commands and the
other actions: the choice to see what is being run and what the model
was thinking.

What the face has today, read from the code:

- A tool call is a card that collapses to one line when it finishes
  (`app.js:846`), with the *output preview* toggled by a click and the
  raw record in the inspector. The command itself is clipped to 72
  characters in the collapsed line (`tools.rs:1221`) and the full
  argument is only in the raw JSON.
- Routing, memory, skills, lane joins, compaction, checkpoints and
  controls fold into one run strip (`app.js:944`); each folded line is
  text with a rewind, and nothing opens it.
- **The model's reasoning never arrives.** The SSE parser reads
  `choices[].delta.content` and `tool_calls` only (`api.rs:790`); there
  is no `reasoning_content` field, no `StreamEvent::Reasoning`, no
  record and no event. So a "Thought · Ns" row cannot exist until the
  stream carries it.

## Approach

**Reasoning rides the stream, once per turn.** The chunk delta gains
`reasoning_content`; `parse_sse` emits `StreamEvent::Reasoning`; the
agent accumulates it with the clock it started on, and when the response
lands it records one `Record::Reasoning { text, seconds }` and emits one
`UiEvent::Reasoning`. The log keeps the transcript reconstructable (the
reasoning is displayed, so it is in the log); it is never sent back to
the provider — the conversation builder ignores it, which is what
DeepSeek's own reasoner requires.

**One activity row, two kinds.** A thought row reads `Thought · 5s`
with a quiet glyph; a command row reads `Ran a command` with the
command clipped in the row. Both are one line, both expand in place on a
click, and neither adds chrome while closed. The command's expanded
detail carries what actually ran: the full argument (the command or the
path), how long it took, the output preview, and a `raw` action that
opens the record in the inspector. The tool call event grows an
`arguments` field so the face has the full text without another read.

**The strip opens too.** Each folded line in the run strip (route,
memory, skills, lane, compaction, checkpoint, folder, control, repair,
rewind) opens its record in the inspector on a click — the same
click-to-see-more, so no action is a dead label.

**Quiet by default.** Closed rows are one line and the transcript's
height is unchanged by this work until the owner asks; the expanded
detail is bytes, not a second card.

The reference rows, in the face's own voice:

```
◇ Thought · 5s
> Ran a command   cargo test
  ┌ opened ─────────────────────────────────┐
  │ cargo test                              │
  │ 1498 passed · 0 failed · 1 ignored      │
  │ 12s · [raw]                             │
  └─────────────────────────────────────────┘
```

## Acceptance criteria

- [x] **C0 — the stream carries reasoning.** `ChunkDelta` parses `reasoning_content`; `parse_sse` emits `StreamEvent::Reasoning` for it and does not fold it into `content`. A parser test feeds a chunk with both fields and asserts the reasoning event and that the assembled content holds only the answer.
- [x] **C1 — one record and one event per turn.** The agent accumulates a turn's reasoning and, when the response lands, writes one `Record::Reasoning { text, seconds }` and emits one `UiEvent::Reasoning` with the elapsed seconds; an empty reasoning emits nothing. `Record::Reasoning` replays to no message (the conversation builder ignores it), so the provider never sees it again. Pinned by a unit test over a stand-in stream and by `conversation()` ignoring the record.
- [x] **C2 — the thought row.** A reasoning event renders one row reading `Thought · Ns`, collapsed by default with the text hidden; a click expands it (markdown-rendered) and a second collapses it. A replayed history renders the same rows from `Record::Reasoning` cards. Pinned by a face test over a fixture reply with two reasoning blocks.
- [x] **C3 — the command row shows what ran.** The tool call event carries its full `arguments`; the collapsed row is one line, and a click expands a detail with the full command (or path), the elapsed time, the output preview and a `raw` action that opens the record. A refused call stays expanded. Pinned by a face test: a `bash` call with a long command expands to the whole command and its preview, and the raw action opens the inspector.
- [x] **C4 — the strip's lines open.** Each folded strip line opens its event in the inspector on a click; the turn's rewind stays its own control. Pinned by a face test on a route line.
- [x] **C5 — quiet by default.** A closed thought or command row is one line: the reasoning body and the command detail are hidden until asked, and the row carries no extra card. Pinned by the same face tests (the detail starts hidden) and the stylesheet's one-line row rule.
- [x] **C6 — documented.** The functional spec's run behavior and `docs/features/gui.md` name the activity rows, the reasoning record, and what stays raw; the ledger records anything found.
- [x] **C7 — green.** The gate (`fmt`, `clippy`, `build --release`, `test`) and the face harness; a live pass against a stand-in provider that emits `reasoning_content` and one tool call, showing a `Thought · Ns` row that expands and a command row that expands to its command and output.

## Files to be touched

`src/api.rs` (the delta field, the event, the parser and its test),
`src/session.rs` (`Record::Reasoning` and `conversation()`'s arm),
`src/agent.rs` (accumulate, record, emit; the tool call's `arguments`),
`src/gui/bridge.rs` (`describe` and `cards_from_records`), `src/gui/assets/app.js`
(the thought row, the command detail, the strip's click), `app.css`, the
face harness, and the two documents.

## Out of scope

Re-sending reasoning to the provider (it must not be). A token-level
thought stream (one row per block, which is what a reader can act on).
Persisting an expanded/collapsed choice across reloads. A reasoning
budget or a provider that does not emit the field (the row simply does
not appear). The TUI's own Thought block: a resumed log's reasoning
records replay to no pane entry there, and the face is the surface this
plan builds.

## Verification

Unit tests for the parser, the record's round trip and the conversation
builder; bridge tests for the card and the describe shape; harness cases
for the thought row, the command detail and the strip's click; the gate;
and a live pass on a stand-in provider emitting reasoning and a tool
call — the thought row's text, the command row's full command, and the
preview from the run.
