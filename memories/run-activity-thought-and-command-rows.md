---
name: run-activity-thought-and-command-rows
description: "owner request 2026-10-23 (ZCode's activity timeline as the reference) — the transcript is now one closed row per thing that happened: a `Thought · Ns` row per response (the model's reasoning never reached fleet before; it now rides the stream, the log and the face, and is never sent back) and a command row whose press opens what actually ran, its time, its output and a raw-record button; the run strip's lines open their records too"
type: project
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Owner, 2026-10-23, with a screenshot of ZCode's own timeline: *"I really like the way
zcode presents what's happening without being wasteful but also giving the option to
show more details for example the thought items where you can click on them and see
more details, but also needs to extend to commands an other actions, giving me the
choice to see what's being run and what's the thought process of the model."*

**The blocker was upstream of the face.** Fleet never received the model's reasoning
at all: the SSE delta parsed only `content` and `tool_calls`. So the work was
end-to-end, planned as `docs/plans/2026-10-23-run-activity.md` (status completed,
eight criteria ticked) and shipped the same day (`202aac6` feature, then `32ce8a1`,
`790d069`).

**What shipped:**

- `ChunkDelta` reads `reasoning_content`; `parse_sse` emits `StreamEvent::Reasoning`
  beside the content, and `chat_stream_plain` accumulates reasoning into nothing (it
  is not part of the answer). The agent owns a `ReasoningBlock` — first delta starts
  the clock, `close()` returns the text and seconds **once**, and it closes when the
  answer begins *or* when the response lands (a tool-only turn has no answer to
  close it). One `Record::Reasoning { text, seconds }` per response and one
  `UiEvent::Reasoning`. **`conversation()` ignores the record**, so a reasoning
  model's thinking is never sent back — compile-enforced by a test asserting the
  built messages are exactly the question and the answer.
- `UiEvent::ToolCall` gained `arguments` (the raw call), because the collapsed line
  is a 72-char summary and the expanded row must name what actually ran.
- The face: `thoughtRow` renders `◇ Thought · Ns`, closed, opening to
  markdown-rendered words; a tool card's press opens a `.tool-detail` (full command
  or path, elapsed + `ran`, the output preview, a `raw record` button) instead of the
  generic JSON inspector; a refused call stays expanded; each run-strip line opens
  its own record.
- **Replays name their commands:** `cards_from_records` built only result cards, so a
  resumed transcript had a tool and an output but no call. The assistant record's
  `tool_calls` now become `tool_call` cards (summary + arguments) after the message
  and before the results — the same pair the live stream emits.

**How it was verified.** Unit: the parser's reasoning events with the assembled
content untouched; the record's round trip plus `conversation()` ignoring it;
`ReasoningBlock`'s one-close rule; a replay card carrying its command. Face harness:
three cases (thought row, command detail, strip line) plus a stub fix — **the DOM
stub must honour `stopPropagation`**, because without it a nested button's click
also reads as its card's click, which is a fidelity gap rather than an app bug (the
same shape caught the tool-card-vs-raw-button collision). Live: a stand-in `node` SSE
provider that streams `reasoning_content`, calls `bash` with `echo hi`, then answers
in markdown; the log held `reasoning`, the transcript one closed `Thought · 0s` and
one command card whose detail named `echo hi`, its time and its output. On the real
archive a resumed conversation showed `echo eleven` and `sleep 6`.

**Kept honest:** the TUI ignores `UiEvent::Reasoning` and `Record::Reasoning` (a
later increment, stated in the plan); reasoning duration is whole seconds and a fast
stand-in reports `0s`; the model's thinking appears only when the provider streams
the field.

**The closing audit added the pins the earlier pass lacked** (`2d88d70`, gate 1512,
harness 101 checks across 14 cases). A completion verifier had refused the objective
because the evidence was live-only: the parser test and the card test existed, but
nothing proved the *turn loop's* rule (one event per response, before its answer) and
nothing proved the record reached the endpoint. Added: an end-to-end integration test
over the scripted mock (`a_runs_thinking_is_logged_once_and_heard_before_the_answer`
— exactly one `Reasoning` event before the first token, one log record, and
`conversation()` not carrying it back), a replay-card test, and two face checks for
C5 (the closed row is a flex line; every thought body and command detail starts
hidden). **The generalisable lesson:** a feature whose pieces are each unit-tested
still needs one test spanning producer → log → surface, because that seam is where
"the event fires and the record is written" can be true while "once" and "in order"
are not. KI-304 (the replay naming its command) came from the same audit.

Related: [[fleet-work-and-the-conversation-revamp]] (the surface this lands in),
[[gui-client-verified-by-a-vm-harness]] (the harness and its stub lessons),
[[an-undefined-call-hides-in-a-branch-nothing-exercises]] (same family: a path
nothing exercises).
