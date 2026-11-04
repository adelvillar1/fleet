---
status: draft
created: 2026-11-04
updated: 2026-11-04
slug: context-health
---

# Plan: context health — the measure that replaces the turn count

## Context

The 2026-11-03 ruling retired the composer's turns-left: *"turn count is
meaningless to end users — compaction count or context degradation is the
only thing a user would care for."* The working line
(`#working-live`, `app.js:3602`) is now the last place a turn count faces
the owner: it reads "working · turn N · elapsed since the last event".

What the owner asked for instead is **context health**, shown on the
conversation beside that spinner. The audit of what exists:

- **The fold's arithmetic is already window-derived.** `maybe_compact`
  measures the API's `prompt_tokens` (else the chars/4 estimate with the
  prompt counted, KI-081) against `budget_for(window)` — three fifths of
  the serving model's declared window, floored at 100k, held below the
  window's edge (`src/compaction.rs:55`), the window read per-model from
  the roster with the owner's override (`src/route.rs:158`).
- **The face's arithmetic is hardcoded — the divergence this plan
  exists to close.** Both bridge state branches serve
  `"budget": COMPACTION_TOKEN_BUDGET` (`src/gui/bridge.rs:830, 869`),
  the 100k floor constant, so the head's context meter divides by 100k
  whatever model serves: ~4% low on DeepSeek's 128k, ~36% low on 262k,
  ~84% low on 1M. The gauge and the fold disagree, and the gauge is the
  one the owner sees. **The owner's steer: the budget is a function of
  the model's context window, never a constant.**
- **The data already reaches the browser.** Each turn
  `UiEvent::Usage` maps to `{prompt, completion}` (`bridge.rs:10514`);
  the face keeps `face.context = prompt` and computes a percentage in
  three places — but only the head meter shows one, and against the
  wrong denominator.
- **The survival records already exist outside context.** The goal is a
  goals-store row, the task and every ask ride the log, the todo ledger
  is re-injected at every fold (pinned by `a_fold_retains_every_ledger_item`,
  `agent.rs:9438`), deliverables are files on disk read off the log by
  the shelf, and the system prompt is re-sent whole every request. A
  fold shrinks only the working set — so the honest measure of "what's
  left" is about the working set, and it is computable without a model
  judgment: the fold's own markers (`summary_frame` texts, elision
  stubs) are string-identifiable in `self.messages`.
- **DeepSeek reports cache stability for free and Fleet never reads
  it.** `prompt_cache_hit_tokens` / `prompt_cache_miss_tokens` ride
  every response (hit + miss = prompt); the `Usage` struct has
  `cache_read_tokens` but the chat-completions parser never fills it.

**Outcome.** One arithmetic — the fold's — shown on the conversation:
occupancy against the budget the fold actually uses, its trend, folds so
far, what survived the last fold, and the cache canary. The turn count
leaves the surface entirely.

## Precedents this plan answers

- **The turn budget's deferred discussion** (2026-11-03, decision 3 of
  `docs/plans/2026-10-31-quota-failover-and-compaction-count.md`): the
  crude turn-cap proxy. This plan answers the *surface* half — the
  display becomes context health. The turn-budget *machinery* stays
  until that question is decided on its own.
- **The turns-left retirement** (`4caed48`): this plan removes the last
  turn count facing the user and makes the head gauge — the ruling's
  "one degradation signal" — actually agree with the fold.
- **KI-420** (the shelf drawn only from live events vanished on resume):
  C4 is the same defect class caught before it ships — a reopened
  conversation must read its health off the log, not off a stream that
  no longer exists.
- **KI-081** (the optimistic estimate): the measure the gauge shows is
  the same measured `prompt_tokens` the fold compares, so the two can
  never read differently.
- **The usage-in-calibration plan's Increment C** (`ShapeRow`): the
  per-turn curve is already recorded per run; this plan surfaces it live
  while the run is in flight.

## The contract

- **One denominator.** The state derives
  `budget_for(declared_window(provider, model))` for the model the next
  run would serve (the pin if set, else the session's last serving
  model); with nothing declared it serves the floor and says so. The
  per-turn usage event carries the *run's* budget from
  `serving_window()` — a mid-run pin can change the model, and
  therefore the budget, at the next turn boundary.
- **One measure, two surfaces.** The working line on the conversation
  and the head meter draw the same occupancy; the meter's bands widen
  (calm / high / fold-imminent); the hover detail names tokens, budget,
  the model that declared the window, folds, first-hand share, and the
  cache-hit ratio when the provider reports one.
- **The trend is shown only when it exists.** Turns-to-the-fold is
  derived from the live prompt history and appears once a slope is
  stable — never on turn one, never as a guess.
- **Survival is a logged fact, not a vibe.** At each fold the agent
  checks the named set — task, goal, deliverable paths, ledger items —
  for presence in the assembled working set and records the M-of-N
  line in the compaction record, naming what dropped. The known
  limitation (prose paraphrase reads as absent) is stated where the
  check is defined, not hidden.
- **The share is computed from the fold's own markers**, so it cannot
  drift from what the fold actually wrote.

## Acceptance criteria

- [ ] **C0 — the state's budget.** Both state branches carry
      `budget_for(window)` for the model the next run would serve; the
      floor appears only when no window is declared, and the state names
      that. Harness: a session whose serving model declares 262k reads
      157,286, not 100,000.
- [ ] **C1 — the run's budget on the wire.** `UiEvent::Usage` carries
      the run's budget and the cache hit count;
      `describe()` passes both; the face uses the event's budget while
      a run is live and the state's when idle. Harness: a fixture run
      that re-pins mid-run shows the budget change at the next turn.
- [ ] **C2 — the health line.** The working line reads the measure, not
      the turn: "working · 34k of 96k · ~12 turns to the fold · 2 folds
      · 8s since the last event", with turns-to-fold only when a stable
      slope exists and never for a fresh run. The turn count is gone
      from the line. Harness pins the three states: fresh, trending,
      post-fold.
- [ ] **C3 — the meter.** The head meter divides by the same budget the
      line does; the bands are calm / high / fold-imminent; the hover
      detail names tokens, budget, the declaring model, folds, share,
      cache-hit ratio. One arithmetic, two surfaces — pinned by a
      harness case that reads both.
- [ ] **C4 — the resume seed.** The state read carries the last turn's
      prompt tokens and the fold count off the log, so a reopened
      conversation shows its health immediately; `face.context` /
      `face.budget` never initialize to 0% against a conversation that
      has run. Proven on a resumed session in the harness.
- [ ] **C5 — the cache parse.** The chat-completions parser reads
      `prompt_cache_hit_tokens` / `prompt_cache_miss_tokens` into
      `Usage.cache_read_tokens` per DeepSeek's contract (hit + miss =
      prompt); a provider that reports neither leaves the field absent,
      never zero. Fixture pins the parse and the ratio arithmetic.
- [ ] **C6 — the survival check.** At each fold the agent checks task,
      goal, deliverable paths, and ledger items against the assembled
      working set; the compaction record gains the M-of-N line and
      names anything that dropped; the limitation (prose containment)
      is stated at the definition. Harness: a fold fixture whose
      summary omits a deliverable path reads "3 of 4 deliverables still
      in context — the fold dropped landing-copy.md".
- [ ] **C7 — the first-hand share.** The working set's verbatim /
      summarized / stubbed composition is computed from the fold's own
      markers, rides the usage event, and shows in the hover detail;
      it only drops at a fold. Harness: no folds reads 100% verbatim;
      post-fold reads lower.
- [ ] **C8 — the gate.** fmt, clippy `-D warnings`, release build,
      `cargo test` exit 0; the harness count read from its own line;
      every existing case stays green.
- [ ] **C9 — the live proof.** On a scratch home, a model whose declared
      window is overridden small drives a real fold live: the line's
      occupancy climbs, the fold logs its survival line, the share
      drops, the cache ratio shows, and reopening the session seeds the
      health read from the log.
- [ ] **C10 — the delivery.** The deployed `~/.fleet/bin/fleet` serves
      the new face, the Tauri app is rebuilt, the branch is pushed, and
      the plan's status flips in the landing turn.

## Out of scope

- The turn-budget machinery itself — it stays; the 2026-11-03 question
  is answered only on the surface this plan opens.
- The compaction request's manifest seam and the sem1 coverage score —
  the exact "what did the summarizer keep" measures; deferred
  increments, named in C6's limitation note.
- The TUI's display — the browser face is the surface the owner named.

## Verification

```sh
node tests/face/harness.mjs

# the gate (exit codes read directly)
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo test
```

The harness cases pin the state's denominator, the wire's budget and
cache fields, the line's three states, the meter parity, the resume
seed, the survival line, and the share's monotonic drop. C9 is driven
by hand on a scratch home and recorded in the recap.

## Linked artifacts

- `docs/features/gui.md` — the runbar section: the gauge's contract
  gains the model-derived denominator, the bands, and the health line.
- `docs/plans/2026-10-31-quota-failover-and-compaction-count.md` — the
  deferred discussion this plan answers.
- `docs/known-issues.md` — the divergence this plan closes would be a
  defect row if it shipped; the plan closes it instead.

## Risks

- **Two budget sources can drift** (state vs. event). The contract is
  one function read from two places; the harness case that reads both
  is the pin.
- **Turns-to-fold can lie** when the slope is noisy (one huge tool
  result). It appears only on a stable slope and reads as an
  approximation, never a promise.
- **The survival check's containment is crude for prose.** Stated at
  the definition; the manifest seam is the named cure if the check's
  false negatives ever hide a real loss.
- **A mid-run pin changes the budget mid-conversation.** The line
  reflects the model that will serve the *next* request — the same
  truth the fold acts on.
