---
status: complete
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

- [x] **C0 — the state's budget.** Both state branches carry *Met:*
      `next_run_budget` derives `budget_for` from the pin, else the
      log's last `End.served_model` (via `declared_window_for_model`),
      else the configured model; `budget_declared` names the floor.
      `the_state_budget_follows_the_next_runs_model` reads 157,286 for
      a 262k-declared model, and the old floor assert became the
      derived one (600,000 for the fixture's 1M model). Live: the
      console's empty session reads `budget: 100000, declared: false,
      model: auto` — the floor, named.
- [x] **C1 — the run's budget on the wire.** `UiEvent::Usage` carries *Met:*
      `budget_tokens`, `folds`, `verbatim_share`, `cache_read_tokens`;
      `describe()` maps all four. Live proof: the served model changed
      step-5-preview → deepseek-flash mid-proof and the event's budget
      followed (614,400 then 100,000), the mid-run override included.
      The wire, caught on `/events`: `{"budget":100000,"cache_read":3968,
      "completion":2,"folds":0,"prompt":49724,"share":97,"type":"usage"}`.
- [x] **C2 — the health line.** The working line reads the measure, not *Met:*
      the turn — `healthLine()` names occupancy, the trend's
      turns-to-the-fold (only on three climbing readings), and the
      folds. `theWorkingLineReadsTheHealthNotTheTurn` pins fresh
      (no slope), trending (~N turns to the fold), and the folds; the
      exchange-count case now asserts the line names neither turn nor
      exchange. The trend is an approximation by its own doc comment.
- [x] **C3 — the meter.** The head meter divides by the same budget *Met:*
      the line does; `paint()` sets calm / high / fold-imminent (the
      `full` band retired, the `imminent` rule beside it), and
      `contextDetail()` names tokens, the declaring model or the named
      floor, folds, the share, and the cache-hit ratio. The harness
      case reads both surfaces from the same event.
- [x] **C4 — the resume seed.** The state read carries the last turn's *Met:*
      prompt tokens and the fold count off the log (`session_context_health`
      over the usage and compaction records), the reset block seeds
      them, and `aReopenedConversationSeedsItsHealth` reads 64% on
      reopen with the hover naming the model and the fold. Live: the
      post-fold state read carried `context=49710, folds=1` — the
      reopened conversation's health, not a zero.
- [x] **C5 — the cache parse.** The usage parser reads whatever the *Met:*
      **served** provider reports: the serde alias reads DeepSeek's
      `prompt_cache_hit_tokens`, the `prompt_tokens_details` capture +
      `absorbed()` fold reads OpenAI's nested count, the Anthropic name
      as before — all into the generic `Usage.cache_read_tokens`, and
      `the_cache_breakdown_reads_whatever_provider_served` pins each
      plus the absence and the never-serialized capture. Live:
      `cache_read: 3968` rode the wire event from the served
      deepseek-flash response.
- [x] **C6 — the survival check.** At each fold the agent checks task, *Met:*
      goal, deliverable paths (`write_file`/`send_file` paths,
      `deliverable_paths`), and ledger items against the fold's working
      set (`compaction::survival_check`, the limitation stated at the
      definition); the record carries the answer. Unit pins: the dropped
      path names itself, containment survives case/whitespace, an empty
      item is kept. Live: the fold's record read `survival: {kept: 1,
      total: 1}` — the task survived in the verbatim tail, honestly
      counted rather than assumed.
- [x] **C7 — the first-hand share.** `verbatim_share` classifies by *Met:*
      the fold's own markers (`summary_frame`'s and the elision stub's
      opening words), rides the usage event, and shows in the hover.
      Unit pins: 100 until a fold marks the conversation, the stub
      counts as second-hand. Live: `share: 97` on the wire after the
      proof's fold — second-hand, and only because of it.
- [x] **C8 — the gate.** fmt 0, clippy 0, release build 0, *Met:*
      `cargo test` 0 (1,838 lib + 164 integration + the eval corpus and
      the rest, all suites), harness 844 checks / 95 cases exit 0 —
      counts read from their own lines. The eval corpus's log-shape
      counts were re-measured (10/10/11) and the position-based asserts
      moved one slot for the usage record. KI-426 filed the layerfs
      stubs' load-sensitivity the full runs exposed.
- [x] **C9 — the live proof.** On a scratch home (`/tmp/fleet-health-proof`, *Met:*
      the machine's credentials in its environment, nothing on disk),
      seventeen runs through the real API: the router served
      step-5-preview and the budget followed it (614,400); the owner's
      per-model override moved the budget to 100,000 mid-conversation
      without a restart; the fold fired at a two-turn boundary through
      the owner's own `{"fold": true}` control — 198,036 → 42,186
      tokens, one compaction record, survival kept 1 of 1; the wire
      carried the usage event with the budget, the share (97), and the
      served provider's cache breakdown (3,968); the state read seeded
      the reopened conversation (context 49,710, folds 1). The proof
      also caught the seam the plan's contract names: the fold's
      per-pair window lookup and the gauge's by-model scan disagree
      when an override sits under a different provider than the one
      that served — the proof's own first override did, and the honest
      reading is the plan's one-arithmetic rule, which the fold's
      per-pair lookup follows.
- [x] **C10 — the delivery.** `~/.fleet/bin/fleet` replaced through a *Met:*
      new inode and `launchctl bootout` + `bootstrap` on `com.fleet.gui`
      (pid 98960, 127.0.0.1:3080); the served `/app.js` carries
      `healthLine`/`turnsToFold`/`contextDetail`, and the console's
      state document reads the derived budget with the floor named on
      an empty session. The Tauri shell rebuilt and bundled
      (`fleet_0.3.0-beta_aarch64.dmg`, 10.39 MiB; the packaged binary
      carries `budget_declared`, `prompt_cache_hit_tokens`,
      `prompt_tokens_details`, and the embedded face's
      `face.budgetDeclared` line). Branch pushed
      (`b28d04b..d23ea4c`).

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
