---
status: completed
created: 2026-10-21
updated: 2026-10-21
slug: report-repair
---

# Plan: the harness asks once for a missing report

## Context

The prompt now carries a stop contract (`REPORTING_CONTRACT`), but a
prompt is an instruction, not a guarantee. The loop still accepts a
final assistant message that is empty — or a bare "done" — as a
completed run, so a model that ignores the contract leaves the owner
exactly where they complained: tool cards and silence.

## Approach

One bounded repair, in the loop, at the seam where a run stops:

- **The trigger** (`is_silent_stop`): blank content, or an
  acknowledgment-only message (`done`, `ok`, `sure`, `all done`, …,
  case- and punctuation-insensitive) — in either case only when the run
  executed at least one tool. A chat that answers "yes" to a yes/no
  question is not repaired, because it did not fail to report.
- **The repair**: the silent assistant turn is **not** pushed to the
  conversation (it says nothing, and two assistant turns would poison a
  resume). The model is asked again with `REPORT_INSTRUCTION`, a
  compiled-in constant appended to the system prompt for that request
  only. The report that comes back is logged as the ordinary assistant
  turn.
- **The bound**: one repair per run, and only when a turn remains under
  the cap. If the repair is also silent, the run ends — and its stop
  reason is `completed without a report` instead of `completed`, so the
  log and the finished card say what happened rather than showing a
  clean end.
- **The visibility**: a `ReportRepair` UI event, so the extra model
  call is seen and paid for knowingly; the headless path writes the same
  line to its progress sink.

## Acceptance criteria

- [x] C1: A run that executed tools and stops with no content is re-asked once; the run's final message is the report, and the log has no empty assistant record.
- [x] C2: A bare acknowledgment (`done`/`ok`, case- and punctuation-insensitive) triggers the same repair; an informative final message does not.
- [x] C3: A run with no tool calls is never repaired for an acknowledgment — a conversational answer stays one.
- [x] C4: The repair is bounded to one per run; a second silent stop ends the run with stop reason `completed without a report` in the log and on the finished event.
- [x] C5: The repair is visible: a `repair` event on the live surfaces and a headless progress line; the repaired request's system prompt carries `REPORT_INSTRUCTION`.

## Files to be touched

- `src/agent.rs` — `REPORT_INSTRUCTION`, `is_silent_stop`, the loop's stop seam, the `ReportRepair` event, the tool-call counter, and unit tests.
- `src/gui/bridge.rs` — `describe` maps the event.
- `src/gui/assets/app.js` — the repair line in the transcript.
- `src/tui/app.rs` — the repair line on the run card.
- `tests/integration.rs` — the five behaviour tests against the mock.
- `TECHNICAL-DOCUMENTATION.md`, `FUNCTIONAL-SPECIFICATIONS.md` — the loop's repair and its bound.

## Out of scope

- More than one repair (an unbounded loop is a spend risk, and a model that ignores two asks will ignore ten).
- A new session-log record type: the instruction is a compiled-in constant and the silent turn said nothing, so the log's ordinary messages plus the constant reconstruct the request.
- A repair for a run whose final message is short but informative — length is not the test; emptiness and acknowledgments are.

## Verification

```bash
cargo test --lib silent_stop
cargo test --test integration asked_once
cargo test --test integration a_second_silent
cargo test --test integration an_informative_or_conversational
```

Expected: the blank stop and the ack stop are re-asked once, the
informative and no-tools stops are not, the cap ends with
`completed without a report`, and the repaired request's system prompt
carries the instruction.
