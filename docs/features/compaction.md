# Compaction (slice 10)

Context-window management: summarize-and-continue under a token budget,
with the summary and the turn range it replaces logged as a session
record so model-visible ⟺ logged survives the fold.

Spec: the fork's `packages/compaction` (`compaction-basic/src/{config,
index,region,summarizer,types}.ts`) — read as a reference, never as a
dependency (hard rule 6).

```
src/compaction.rs    new: the budget, the tail, the estimate, the frame, the instruction
src/session.rs       the `compaction` record, the turn-numbered replay, the summary's marker
src/agent.rs         the trigger, the compaction request, the record, the UiEvent, the turn bookkeeping
src/tui/app.rs       the Compacted event as a card, the seeded card
src/tui/ui.rs        the card in the house style, the picker's `compacted` marker
src/main.rs          `fleet sessions` names the marker
tests/integration.rs the acceptance tests, driven through the mock server
```

## The trigger

Checked **between turns**: after each response lands and before the next
request is built. The measure is, in order:

1. the API-reported `prompt_tokens` of the last response, when it carried
   one — the streamed usage does (`include_usage` is asked for on every
   request), so this is the ordinary case;
2. the chars/4 estimate of the assembled messages, when it did not.

The threshold is the named constant `COMPACTION_TOKEN_BUDGET = 100_000`.
DeepSeek's window is 128k, so the fold fires at roughly 80% of it — early
enough that the request crossing it still fits, late enough that a run
which never approaches the window pays one integer comparison per turn
and nothing else.

**One compaction per crossing.** After a fold the counter resets to the
post-compaction size, and a latch holds until a measurement comes in under
the budget. The second half matters: a conversation whose verbatim tail is
itself over the window cannot be helped by folding, and without the latch
the loop would fold every turn until the turn cap.

## The mechanism

Summarize-and-continue, the fork's pattern. One request through **the
same client** — no separate provider, no separate model, the same routed
path with the same failover every other request takes — carrying the
messages being replaced plus the compaction instruction. The model's
answer is the summary.

The request carries **no tools**: a compaction request has nothing to
execute, and a declaration on it would invite a call with no context to
run in. The instruction is a compiled-in constant
(`COMPACTION_INSTRUCTION`), reconstructed per compaction request exactly
as the system prompt and the tool declarations are (hard rule 5's own
carve-out) — it is never logged, and it does not need to be, because the
messages it was attached to are.

## The record

```json
{"type":"compaction","summary":"…","first_turn":1,"last_turn":2,
 "tokens_before":120000,"tokens_after":900}
```

The turns it names stay in the log — a log is append-only — and replay
stands the summary in their place. That is what keeps the requests that
followed reconstructable byte for byte.

## The tail

The most recent `COMPACTION_TAIL_TURNS = 4` turns stay verbatim after the
summary. The tail starts on an **assistant** message, never on a tool
result: a message list that opens with a `tool` role is not a valid
sequence for the API, which is KI-016's failure in a new shape. Everything
before the boundary is what the fold replaces.

The summary rides as a user-role message whose text is the frame:

```
Summary of turns 1 through 2, compacted to fit the window:

- the summary's own text
```

The system prompt is rebuilt per run unchanged, so a compacted run's
prompt is the prompt it has always had.

## Replay and resume

`session::conversation` (with `session::messages` as its message-only
face) materializes the compacted shape: the summary stands in for the
replaced turns and the recent tail stays verbatim. It also returns the
turn number of the request that first carried each message, and the turn
the next request would be — which is what a compaction record names and
what a resumed run continues from.

A session with no compaction record replays byte-identically to before
this slice: the record is additive and an ordinary log's shape does not
change.

## The child loop

A subagent's run is the same loop over the same `Agent`, so the same
trigger applies to it with no special-casing. A child's fold lands in the
child's own log, beside its own `kind: subagent` marker.

## The surfaces

- **Headless**: one progress line, `— compacted: N turns → 1 summary
  (~X → ~Y tokens)`.
- **TUI**: one card in the tool card's own two-line shape — the dot, the
  bold accent name, the dim detail, the settled state, and the token
  counts wrapped underneath, so nothing is cut at the pane's edge. The
  summary itself is not rendered: it belongs to the request it rides in.
- **Session picker and `fleet sessions`**: a compacted session is marked,
  because its replay is a compacted shape and a reader deserves to know
  before opening it.

## Deliberately not built

The fork's later refinements, named here as the documented not-built-yet
rather than as gaps:

- **`compaction-tool-result-pruner`** — dropping or trimming individual
  tool results under pressure instead of folding whole turns. The
  verbatim tail is the crate's answer to the same problem at a coarser
  grain, and it is the one that keeps the log's requests reconstructable
  without a second record kind.
- **`compaction-image-offload`** — moving image content out of the
  window. This crate's tool set is text-only, so there is nothing to
  offload.
- **Ratio-shaped policy** (`thresholdRatio`, `retainRatio`,
  `headroomTokens`, per-model policy tables, `compactionRetries`,
  `maxOverflowRetries`). Two named constants replace the whole table: one
  budget and one tail size. The fork's ratios exist to size a window the
  adapter declares at runtime; this crate's window is one provider's, and
  a second constant is cheaper to reason about than a policy merge.
- **Overflow recovery** — retrying a request the provider refused for
  length. A 413-class refusal is a typed API error the run reports, the
  same as any other.
