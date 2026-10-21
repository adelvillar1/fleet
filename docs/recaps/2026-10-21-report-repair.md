# The harness asks once for a missing report

Status: completed
Date: 2026-10-21

The prompt contract (`5d772e8`) tells the model what a stop owes, but a
prompt is an instruction. The loop still accepted an empty final message
— or a bare "done" — as a completed run, which is the failure the owner
reported: tool cards, then silence.

## What changed

- **The trigger.** `is_silent_stop` is true for a blank message or an
  acknowledgment-only one (`done`, `ok`, `okay`, `sure`, `great`,
  `all done`, `finished`, `completed`, case- and
  punctuation-insensitive) — in either case only when the run put at
  least one tool call
  through the gate. A chat that answers "yes" to a yes/no question is
  an answer, not a missing report.
- **The ask.** The silent assistant turn is not pushed (it said
  nothing, and two assistant turns would poison a resume). The model is
  asked again with `REPORT_INSTRUCTION`, a compiled-in constant appended
  to the system prompt for that request only, so the conversation it
  answers is the one it had when it stopped. The report it returns is
  the ordinary assistant turn.
- **The bound.** One ask per run, and only while a turn remains under
  the cap. A second silent stop ends the run with the stop reason
  `completed without a report` instead of `completed`, so the log's end
  record and the finished card name what is missing rather than showing
  a clean completion.
- **The visibility.** A `ReportRepair` UI event: one line in the browser
  transcript and the TUI run card, and a progress line on the headless
  path, so the extra request is seen and paid for knowingly.

Tests (mock-scripted): a blank stop and a "Done." stop are re-asked once
with the conversation unchanged and no empty assistant record; an
informative answer and a conversational "Yes." (no tools) are never
repaired; the cap ends on `completed without a report` after exactly one
ask. Gate: fmt, clippy, release build, `cargo test` — all green.

## What is left

More than one ask is deliberately out of scope: a model that ignores
the ask once will ignore it again, and each ask is a paid request. A new
session-log record type was not added either — the silent turn said
nothing and the ask is a compiled-in constant, so the log's ordinary
messages plus `REPORT_INSTRUCTION` reconstruct the repaired request.
