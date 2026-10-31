---
status: completed
created: 2026-10-31
updated: 2026-10-31
slug: conversation-density
---

# Plan: the conversation's density — single lines while the run runs, one summary at the turn's end, the answer full

## Context

The conversation spends its real estate on the things that are not the
answer. A tool call renders as a `card-row` carrying a glyph, the
tool's name, the summary, and a live `running · Ns` status — the three
lines the owner counts — even though its `tool-detail` is already built
and already collapsed by default. Thinking renders **only at
completion**: `UiEvent::Reasoning { text, seconds }` is sent once, when
the answer begins (or the stream ends), so during a turn the model's
thinking is invisible and afterwards it is a collapsed card the owner
never opens. The turn mark (D8) folds and unfolds its rows on demand,
carrying the rule the owner set then — "nothing auto-collapses — quiet
by default" — and the owner has now revised that rule for a turn that
has finished: **summarize it**. And neither card wears an icon that says
what it is: the tool's glyph is the tool's first letter, the thought's
is a `◇`.

The owner's direction (2026-10-31): *"while the turn is going the tools
should show as a single line card that I can expand and collapse, the
thinking should be a single line showing the stream and once completed
should also show as a card like the tools, once a full turn is completed
(form request to final output) all should be summarized (tool calls and
thinking) and show as a collapsible card and don't occupy meaningful
real state. The results of the request which is what matter the most
should show as a fully expanded card"* — plus the steer: *"tools and
thought should have an icon on the card that show what they are."*

## Decisions (the owner's, dated and quoted)

1. **2026-10-31 — the live line.** Tools and thinking are one line
  while a turn runs, each expandable; thinking streams its own line.
2. **2026-10-31 — the turn summarizes.** At the final output, the
  turn's tool and thinking rows collapse to the turn's own summary line
  — revising the 2026-10-29 "quiet by default" rule for completed turns
  only: history loaded from a log is never re-folded.
3. **2026-10-31 — the answer is the surface.** The final output renders
  fully expanded; nothing else does.
4. **2026-10-31 — the icons.** The tool card and the thought card carry
  an icon that names what they are, on the card and on the turn's
  summary.
5. **2026-10-31 — the working line, and only on the conversation.**
  *"while working the action shows on the composer and I think that
  spinning icon and working should show on the conversation and show an
  elapsed time"* — and the follow-on steer: *"it should not show in the
  composer because it's a property of the conversation."* The run's
  live state — the spinner, "working", the turn, the elapsed time, and
  a seat's wait — belongs at the tail of the conversation and nowhere
  else. The composer's hint reverts to its idle words in every state;
  the composer is where you act, not where the run reports.

## Approach

**The streaming line needs one new event; the rest is the face.**
`UiEvent::Thinking(String)` rides the reasoning deltas as they arrive —
the `Token` precedent, one send per delta — and the face renders a
single live line whose text grows with the stream. When the
`Reasoning { text, seconds }` event lands (unchanged: one event, the
log's own record), the live line is **replaced** by the thought card,
already collapsed, its body holding the block. The turn's rows then
follow one rule:

- **While running**: the tool card is one line — type icon, tool, `run
  and *Ns*` — with the summary, command, output, raw and re-run one
  press away (the detail that already exists). The thought card is one
  line — type icon, `Thought · Ns` — with the block one press away.
- **At the final output** (`closeAssistant`, where the answer lands and
  is rendered): the current turn's tool and thought rows fold — the
  existing `toggleTurn` machinery, one state — and the turn mark carries
  the summary: `turn 3 · 4 tools · thought 2s`, its press expanding
  them back. The answer stays open.

**The icons** are two new entries in the `ICONS` table (the shared SVG
set the `icon()` helper draws from — the rail's own vocabulary): a
tool-call glyph and a thought glyph, rendered by `icon()` on the tool
card, the thought card, and the turn's summary line. The ASCII `GLYPHS`
letters retire from the card's head.

**The working line lives at the tail of the conversation, and the
composer loses it.** The composer's hint already carries the words
(`working · turn N · <elapsed>`) and the one-second `busyTick`; the
words move to a line at the bottom of the conversation itself — a
spinner glyph, "working", the turn or exchange, and the elapsed time —
ticked by that same ticker, standing down when the run ends. A seat
swaps the words to its own (`waiting for an answer · <elapsed>`), the
seat's answer card beside it where the reader already is. The composer's
hint then renders its idle words in **every** state — no working, no
seat countdown in the composer (the steer: the run's state is a property
of the conversation, not a property of where you type). The line is
display only: it never becomes a log record, and it is re-drawn after
any stream repaint rather than riding the event rows.

High-level steps:

1. **P0 — the icons.** The `ICONS` table gains `tools` and `thinking`;
  the two cards render them.
2. **P1 — the thinking stream.** `UiEvent::Thinking` + its wire shape +
  the face's live line, replaced by the collapsed card on completion.
3. **P2 — the one-line cards.** The tool and thought cards' collapsed
  state is a single line each; the detail stays one press away.
4. **P3 — the turn's summary.** The final output folds the turn's rows,
  the mark names the counts, the answer stays expanded.
5. **P4 — the gates.**

## Increments

Each lands whole — code, checks, docs — and each is green on its own.

### P0 — the two icons

The `ICONS` table gains a `tools` glyph (a tool-call/terminal mark) and
a `thinking` glyph (a thought mark), each one stroke-weight with the
table's own idiom. The tool card's head and the thought card's head
render them through `icon()`; the `GLYPHS` letters leave the card heads
(the map stays for anything else it feeds).

*Checks:* R1.

### P1 — the thinking streams

`agent.rs` sends `UiEvent::Thinking(delta)` beside each reasoning
delta it already accumulates; `bridge.rs` serializes it (`type:
thinking`); the face appends the delta to one live line that follows the
stream (the token line's stick behavior). On the `reasoning` event the
live line is removed and `thoughtRow` renders — already collapsed, the
block in its body. A provider that reports no thinking emits nothing:
the line simply never exists.

*Checks:* R2, R3.

### P2 — the single-line cards

The tool card's collapsed state is one line: icon, tool, the live
status (`running · Ns` / `12s · ran`) — the summary moves into the
detail beside the command, reachable on one press. The thought card's
line is icon + `Thought · Ns`. Both expand and collapse on their heads,
exactly as they do now; only the collapsed composition changes.

*Checks:* R4.

### P3 — the turn's summary at the final output

`closeAssistant` folds the current turn's tool and thought rows (the
`toggleTurn` state, one mechanism) and the turn mark renders the
summary — `turn 3 · 4 tools · thought 2s` — counted from the rows
tagged with that turn, its `aria-expanded` starting false. The answer
row is never a fold target. History loaded from a log keeps its rows as
they stand: the rule revises "quiet by default" for the turn that just
finished and nothing else. The turn fold's own mark keeps its press, so
the summary is one line by default and one press from every row.

*Checks:* R5, R6, R7.

### P4 — the working line on the conversation

A `working-live` element sits at the tail of the conversation column,
between the stream and the composer. While `face.running` it shows the
spinner glyph, "working", the turn/exchange, and the elapsed time —
drawn from the same `face.process.at` anchor and ticked by the same
`busyTick` (`paintComposer` updates both, one timer). A standing seat
swaps it to the seat's own elapsed words, with the answer card where it
already is. The composer's hint keeps only its idle words in every
state: the `working`/seat branches move out of `paintComposer`, and the
`busy` class goes with them. The line is re-drawn on every stream
repaint (it sits outside the event rows, so a replay cannot swallow it)
and is hidden the moment neither a run nor a seat is live.

*Checks:* R9.

### P5 — the gates

The harness case, the docs, the whole gate.

## Landed 2026-10-31

P0: the `ICONS` table gained `tools` (a call at the prompt) and
`thinking` (the drawn diamond); the tool card's glyph and the
thought card's head render them through `icon()`, the `GLYPHS`
letters gone from the card heads. P1: `UiEvent::Thinking` rides the
reasoning deltas as they arrive (the `Token` precedent, one send per
delta, transport-only — the `Reasoning` record and the log are
unchanged), `describe` serializes it as `type: thinking` pinned by
`the_thinking_delta_shape`, the TUI ignores it beside `Reasoning`,
and the face grows one live line the `reasoning` event replaces
with the collapsed card. P2: the tool card is one line running and
done — icon, tool, live status — the summary moved into the detail
beside the command; the thought card is one line with its icon.
P3: the run's end folds the exchange's tool and thinking rows under
their turn marks (`summarizeRun` at the `finished`/`failed` site,
the `toggleTurn` state, one mechanism), each mark carrying the
counts (`turn 1 · 2 tools · thought 4s`); the answer row is never a
fold target, a replayed history never folds for the first time
(`face.replaying` guards the live path, the folds restore after a
rebuild), and the per-visit seed clears the folds so turns cannot
leak between conversations. P4: the `working-live` line sits at the
tail of the conversation — spinner, "working", the turn/exchange,
the elapsed time from the same `face.process.at` anchor, ticked by
the same one-second timer (`busyTick` now repaints `paintWorking`)
— a seat swaps it to its own wait, and the composer's hint keeps
its idle words in every state (the `busy` class and the working/seat
branches left `paintComposer`; plan mode's words survive a repaint
now, which they did not before). The face harness, **509 checks
across 54 cases** — the D1 case pins the icons, the streaming line
and its replacement, the one-line cards with the summary in the
detail, the fold with the counts and the answer open, the press
back, the seat variant, the composer's silence, and history never
re-folded; the chat case's stale hint check moved to the working
line. R2's wire pin: `the_thinking_delta_shape`. The live probe on
the restarted face: five real tool cards measure **32px — one
line — at 1440 and at 1100**, glyph SVG present, summary in the
detail, and the screenshots read as the review — which caught
the first pass's defect: the card body is a grid, so the status
stacked under the tool's name and the card was two lines; the
tool card's body is now a baseline flex row, the title
truncating, the status at the right end. The gate whole: fmt 0,
clippy 0, build 0, **1,848 passed / 0 failed / 5 ignored**
(`--lib` 1,657), the face harness **509 checks across 54
cases**, the ledger green (381 rows, 34 open).

## Use cases

- [x] A: A running turn shows one line each for its tool calls and its thinking, expandable in place.
- [x] B: The thinking is visible as a line while it streams, and collapses to its card when the answer begins.
- [x] C: The completed turn's tool and thinking rows collapse to the turn's own summary line; the answer below stays open.
- [x] D: A tool card and a thought card each wear an icon that names what they are.
- [x] E: While a run works, the conversation itself shows a spinner, "working", and the elapsed time — and a seat shows how long it has waited.

## Acceptance criteria

- [x] **R1 — the cards carry the icons.** The tool card's head and the
      thought card's head render the `tools` / `thinking` SVG from the
      shared `ICONS` table (not a letter, not `◇`); pinned by a harness
      check on both heads.
- [x] **R2 — the thinking streams.** A reasoning delta reaches the face
      and grows the live line while the turn runs; the wire shape
      (`type: thinking`) is pinned by a unit test; a provider with no
      thinking emits nothing and no line renders.
- [x] **R3 — the completed thought is its collapsed card.** On the
      `reasoning` event the live line is gone and the thought card
      renders collapsed with the block in its body, expandable.
- [x] **R4 — the cards are one line collapsed.** The running tool card
      measures one line (icon, tool, status) at 1440 and ~1100, with
      the summary, command, output, raw, and re-run behind one press;
      the thought card likewise. Pinned by measurement and harness
      checks.
- [x] **R5 — the completed turn summarizes.** When the final output
      lands, the turn's tool and thought rows are folded and the turn
      mark carries the counts (`N tools`, `thought Ns`); the summary is
      one line and its press expands every row.
- [x] **R6 — the answer stays open.** The assistant's final output
      renders fully expanded and is never folded by the summary.
- [x] **R7 — history is never re-folded.** A log loaded into the
      conversation keeps its rows as the record stands; only the turn
      that finished during this session folds.
- [x] **R9 — the working line is on the conversation and absent from
      the composer.** While a run is live the tail of the conversation
      carries the spinner, "working", the turn, and an elapsed time
      that ticks; a standing seat shows its own wait there; the line
      survives a stream repaint and is gone when neither is live — and
      the composer's hint reads its idle words while the run works, so
      the run reports only where the reading is. Pinned by a harness
      case (the words, the tick, the seat variant, the removal, and the
      composer's silence).
- [x] **R8 — the gate.** The four commands run and each exit code read
      directly (`cargo fmt --all --check`, `cargo clippy --all-targets
      -- -D warnings`, `cargo build --release`, `cargo test`) plus
      `node tests/face/harness.mjs`, all green with the counts quoted.

## Files to be touched

**Backend / API:**
- `src/agent.rs` — `UiEvent::Thinking` and its send beside the reasoning deltas
- `src/gui/bridge.rs` — the event's wire shape + the unit pin

**Frontend:**
- `src/gui/assets/app.js` — the two icons, the live thinking line, the one-line cards, the turn summary on completion, the working line
- `src/gui/assets/app.css` — the card heads, the summary line, the working line's spinner

**Tests:**
- `tests/face/harness.mjs` — the TURN case
- `src/gui/bridge.rs` tests — the wire-shape pin

**Documentation (see Linked artifacts below):**
- `docs/features/gui.md` — the conversation's density and the turn summary
- `FUNCTIONAL-SPECIFICATIONS.md` — the run behavior's transcript passage

## Out of scope

- **Folding history on load** — the revisited rule applies to the turn
  that finished, never to a log's past turns.
- **Changing what the log records** — the records are unchanged; this
  is display only.
- **The turn's row order and the D8 navigation** — the mark keeps its
  fold/press, its jumps, and its rewind.
- **Grouping tool results separately** — a result rides its call's
  collapsed card, as today.

## Verification

**R1, R4–R7** — the face harness and the probe:
```bash
node tests/face/harness.mjs
```
**R2** — the wire-shape pin:
```bash
cargo test --lib thinking_shape
```
**R8** — the gate, exit codes read directly; the probe quotes the card
heights at two widths; the screenshots are read as the review.

## Linked artifacts

- `docs/features/gui.md` — the conversation's cards and the turn summary
- `FUNCTIONAL-SPECIFICATIONS.md` § "Run behavior" — the transcript's shapes

## Risks

- **A per-delta event costs one send per reasoning frame.** The token
  path already does exactly this and the transport is the same queue;
  the line is a single DOM append, the cheapest shape in the surface.
- **Auto-folding at the final output can surprise a reader mid-scroll.**
  The fold happens where the answer lands, the mark names what folded,
  and one press restores the rows; the stick anchor keeps the reading
  position where the reader was.

## Dependencies

- The D8 turn-fold machinery (`toggleTurn`/`paintTurnFold`) — this
  reuses it rather than adding a second folding mechanism.
- The `ICONS` table and the `icon()` helper — the rail's shared set.
