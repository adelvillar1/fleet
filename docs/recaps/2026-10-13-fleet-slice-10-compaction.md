# Slice 10 — compaction (2026-10-14)

**Plan:** `docs/plans/2026-10-13-fleet-everyday-harness.md` (slice 10).
**Feature:** `docs/features/compaction.md`. **Ledger:** KI-079..KI-086.

## What landed

`src/compaction.rs` (new) — the vocabulary and arithmetic — plus
`src/session.rs` (the record and a turn-numbered replay), `src/agent.rs`
(the trigger, the fold, the record, `UiEvent::Compacted`, per-message
turn bookkeeping), and the TUI/headless surfaces.

- **The trigger.** Between turns — after each response lands, before the
  next request is built — measuring the API-reported `prompt_tokens` of
  the last response when it carried one, and the chars/4 estimate of the
  assembled messages when it did not, against
  `COMPACTION_TOKEN_BUDGET = 100_000` (roughly 80% of DeepSeek's 128k
  window). A run that never approaches the window pays one integer
  comparison per turn and nothing else.
- **One compaction per crossing.** A latch holds from a compaction (or a
  failed or skipped one) until a measurement comes back under the
  budget. The second half matters: a conversation whose verbatim tail is
  itself over the window cannot be helped by folding, and without the
  latch the loop would fold every turn until the cap.
- **The mechanism.** Summarize-and-continue, the fork's pattern, through
  **the same client** — one request on the run's own routed path, no
  separate provider and no separate model — carrying the messages being
  replaced plus the compiled-in `COMPACTION_INSTRUCTION` and no tools.
  The model's answer is the summary; an empty answer skips the fold
  rather than replacing the conversation with nothing.
- **A failed fold does not kill the run.** The fold is an auxiliary
  call: if its request errors, the run keeps its context, prints one
  named line (`— compaction failed: <reason>; the run continues
  un-compacted`), holds the latch, and continues. It then either
  finishes inside the window's 28k headroom or dies at the API's own
  length refusal, which names the real limit. (This was the review's
  Important: the error used to propagate and kill a healthy run.)
- **The record and the replay.**
  `{"type":"compaction","summary":…,"first_turn":…,"last_turn":…,"tokens_before":…,"tokens_after":…}`.
  The turns it names stay in the log — a log is append-only — and
  `session::conversation` (with `messages` as its message-only face)
  stands the summary in their place, so every post-compaction request
  reconstructs byte for byte: the tests compare the log-derived message
  list against the mock's captured HTTP body, two independent
  derivations. The tail is the most recent `COMPACTION_TAIL_TURNS = 4`
  turns, verbatim, starting on an **assistant** message — a list
  opening with a `tool` role is not a valid API sequence, KI-016's
  failure in a new shape. The summary rides a user-role message framed
  `Summary of turns A through B, compacted to fit the window:`.
- **Resume.** A compacted session resumed from disk continues coherently:
  the replay builds the compacted list from the record, new turns append
  after the tail, and the next turn numbers from the log's own. A log
  with no compaction record replays byte-identically to before.
- **Uniformity.** The subagent child is the same loop, so the same
  trigger applies to it with no special-casing.
- **The surfaces.** The headless line `— compacted: N turns → 1 summary
  (~X → ~Y tokens)`, one TUI card in the tool card's own two-line shape
  with the token counts wrapped underneath, and the `compacted` marker
  on the session picker and `fleet sessions`. The fork's
  `compaction-tool-result-pruner`, `compaction-image-offload`, its
  ratio-shaped policy table, and its overflow recovery are named in the
  feature doc as the deliberate not-built-yet.

## Evidence

- Gate, parent-run after each commit: `cargo build --all-targets` exit 0,
  **zero warnings**; `cargo test` exit 0 — **347 unit + 83 integration
  passed, 1 ignored** in the final state (baseline 326 + 74; nothing
  deleted). rustfmt/clippy still absent (KI-007).
- The acceptance tests drive the mock server and assert the next
  request's message list, the replay equality byte for byte, the resume's
  first request, the compaction request's own reconstructability from
  the log's preceding records, both measure paths, one fold per
  crossing, the tool loop's integrity across the fold, and the ordinary
  log's unchanged replay.
- The parent's visual pass drove a real fold through the TUI (six tool
  turns, the sixth's usage crossing the budget, the fold, the answer)
  and captured the card: `✓ compacted  2 turns → 1 summary  done` /
  `└ ~120000 → ~142 tokens`, sitting chronologically between the last
  turn and the final answer. The first script was wrong, not the code:
  a one-turn conversation has `tail_start == 0` — the whole conversation
  is the tail, so the trigger correctly refuses to fold, and the mock's
  FIFO then served the fold's script to the turn-2 request. Six turns is
  the first foldable point, exactly as the test helper's comment says.
- Negative probes: the failed fold's test asserts the run completes, no
  compaction record is written, the failure is named, and the fold is
  not retried; the pluralization fix fails at the old "1 turns".

## Review cycle

- **Stage 1 (spec, step-5-preview): SPEC-COMPLIANT** — all ten checks;
  the replay equality is a real cross-derivation, not a tautology.
- **Stage 2 (quality, step-5-preview): REQUEST_CHANGES** — no criticals;
  three Importants, all fixed in `0bdf969`:
  1. the fold's error path was fatal to the run and untested — now the
     continuation above, pinned by a test;
  2. the compaction card owed a live terminal capture — done by the
     parent (above), KI-086 logs the one caveat;
  3. `window_size` was dead state written four times and read never,
     with a doc comment falsely claiming it was the one-fold-per-crossing
     mechanism — deleted; the latch is the mechanism, and the comments,
     the feature doc, and the plan's record now say so.
  Plus the minors: the empty-summary skip now latches (a
  deterministically-empty model is not asked once per turn), the card
  pluralizes like the headless line, the plan's record counts its seven
  ledger rows, and KI-080's evidence cites the integration test that
  pins it.
- **Both reviews dispatched in ONE parallel call** — the pattern's
  second use.

## Not done (slice 11+)

- Scheduled runs (11), self-maintenance (12), diagnosis (13), boards
  (14), install (15).
- The minors logged: KI-079 (the fold's cost is not separable in the
  usage total), KI-081 (the estimate excludes the system prompt and the
  declarations), KI-082 (a skipped fold's silence beyond its line),
  KI-083 (the turn rule implemented twice, cross-pinned), KI-084 (three
  pre-existing malformed ledger rows, deliberately not fixed mid-slice),
  KI-086 (the card's grammar).
