---
status: active
created: 2026-10-21
updated: 2026-10-21
slug: router-rules-everything-else
---

# The router rules everything but the leader

## Context

Four owner rulings on 2026-10-21, given while reading the router deep dive:

1. Nothing is hardcoded to a model — "the default for everything should be
   the router unless I override it."
2. "The router invokes the lanes and the models based on difficulty."
3. A provider pin narrows the router to that provider's own models.
4. "If all models from the router fail then deepseek-flash is the failover."

Asked how (2) squares with the 2026-10-16 orchestrator ruling — every run
walks `hard`'s chain because the first model loaded is the planner — the
owner narrowed it: *"the orchestrator, the session that leads should be set
to hard, everything else the router rules."*

What the code does today, measured against those rulings:

- `api::DEFAULT_MODEL` ("deepseek-flash" since `9c72bbb`) is seeded as the
  nominal model by `fleet run`, `fleet chat`, the GUI launcher, cron and
  `AgentOptions::default()`; the router replaces it per attempt, so on a
  routed run `--model` changes nothing but the session's `start` record.
- A delegated child shares the parent's client (`src/subagents.rs`
  "the child shares it"), so it runs on whatever candidate the parent
  committed to — the 2026-10-16 hard rule 9. Nothing judges the child's
  brief.
- No workload targets deepseek; an exhausted walk fails by name. The
  reserve serves only under a pin.
- `select` walks `ORCHESTRATOR_WORKLOAD` for every caller, children
  included.

## Approach

Keep the leader on `hard`'s chain. Give every other route — children,
side-routes — the judged workload's own chain. Append deepseek-flash as
the last entry of every walk order the router builds, filtered by the same
admissibility gate as everything else and named on the row when it
serves. Retire the nominal-model seed: the `start` record carries the
route decision's model, and `--model` becomes a pin resolved by name.

**Disclosure.** An increment implementing C0 was committed at `9c72bbb`
before this plan existed (not pushed). It is subject to this plan's
approval like every other criterion: if C0 is rejected or reshaped, that
commit is reverted on the owner's word.

## Acceptance criteria

- [ ] **C0 — the reserve is deepseek-flash and every roster model has a window.** `grep -rn deepseek-chat src tests README.md docs/features` prints nothing; `api::DEFAULT_MODEL` is `"deepseek-flash"`; `context_window` returns `Some` for every HTTP model `default_roster()` serves; the window test's named-absent list is exactly `["blend1/blend1"]`.
- [ ] **C1 — the tail is in every order.** `select`, `fallback` and `workload_decision` return an `order` whose last element is `("deepseek", "deepseek-flash")` whenever the deepseek candidate is enabled and `gates.allow_api_models` is true; the pair appears exactly once even when a `routing.json` workload already lists it; with `gates.allow_api_models: false` no order contains it. Unit tests in `src/route.rs` pin the judged, fallback, provider-pinned and vision orders.
- [ ] **C2 — the tail is named when it serves.** A `Router::chat_stream` walk whose routed candidates all fail with failover-eligible classes attempts deepseek last and serves; the decision row's `reason` then ends with `; served by the failover tail deepseek-flash`. When the tail fails too, the run fails with the last error as today. A mock-scripted test pins both.
- [ ] **C3 — the leader keeps `hard`.** A top-level run's `select` still walks `ORCHESTRATOR_WORKLOAD`'s chain; the existing orchestrator tests stay green unchanged, and `route_cache::Cache::key_for` is unchanged.
- [ ] **C4 — a child is the router's.** `Delegation` carries the home, the dotenv map and the lane specs; spawning a child calls `route::prepare_with_pin` over the child's composed brief with the parent's pin and the child runs on `Client::routed(<its own router>)`, not the parent's client. The child's walk is the **judged** workload's chain (no orchestrator override): with a mock sys1 judging the parent `hard` and the child `quick`, the parent's order heads `stepfun/step-5-preview` and the child's heads `stepfun/step-3.7-flash`; under a parent provider pin the child's order is filtered to that provider. The child's session `start` record names the child's route model.
- [ ] **C5 — no nominal model is hardcoded.** `api::DEFAULT_MODEL` is renamed `api::DEEPSEEK_MODEL` and referenced outside tests only by the roster's deepseek entry and the window table (`grep -rn DEEPSEEK_MODEL src | grep -v test` prints exactly those two lines plus the definition); `route::AUTO_MODEL` is `"auto"` and is what `AgentOptions::default()`, `parse_run`, `parse_chat`, the GUI launcher and cron seed; a routed run's session `start` record names the route decision's model, pinned by a test asserting `Record::Start { model } == decision.model`.
- [ ] **C6 — `--model` is a pin.** `fleet run --model <id>` and `fleet chat --model <id>` resolve `<id>` to a route pin: `provider/model` is the exact pair; a bare model id maps to the one roster provider that serves it; the resolution is refused by name when no provider serves the id, or when more than one does, listing every `provider/model` spelling (e.g. `zai-coding-plan/glm-5.3-flash` and `ollama-cloud/glm-5.3-flash`). The resolved pin reaches `prepare_with_pin`. The help text describes `--model` as a pin whose absence means the router decides. Unit tests pin the three resolutions.
- [ ] **C7 — the docs say what the code does.** `CLAUDE.md`: the "Today's state" DeepSeek bullet names the reserve as every walk's failover tail, and hard rule 9 reads that the leading session walks `hard`'s chain while a child is routed by the router over its brief and inherits the parent's pin. `docs/features/routing.md` gains "The failover tail" and "Children" sections; `docs/features/subagents.md` drops "inherits the parent's model". `src/route.rs`'s `Billing`, `Gates` and `default_preferences` comments no longer claim the reserve cannot serve unpinned. The full gate — `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build --release`, `cargo test` — exits 0 four times.
- [ ] **C8 — deferred, named.** `composed` stays unreachable by judgment: sys1's `route` task emits five labels and `composed` is not one; the lane is vendored and the label is upstream's. Target: a follow-up plan over `services/sys1` and a re-vendor. Effort-per-workload is its own plan (research recorded 2026-10-21). Neither is built here.

## Files to be touched

`src/route.rs` (tail, `AUTO_MODEL`, child walk mode, comments, tests),
`src/api.rs` (`DEEPSEEK_MODEL`), `src/agent.rs` (`AgentOptions` default,
start-record model), `src/subagents.rs` (`Delegation` fields, the child's
own route), `src/cli.rs` (`--model` as a pin, help), `src/main.rs`,
`src/gui/mod.rs`, `src/gui/bridge.rs`, `src/cron.rs` (seed `auto`, pass the
decision's model), `CLAUDE.md`, `docs/features/routing.md`,
`docs/features/subagents.md`, `tests/integration.rs` (prompt and record
pins).

## Out of scope

Effort levels per workload. Adding `composed` to sys1's label set. Polling
context windows at runtime. Renaming the `deepseek` provider id. Rewriting
the known-issues ledger or dated plan/recap records that name
`deepseek-chat` as history. Changing the confidence gate or the preference
table's contents.

## Verification

The gate above, read by exit code. The C1/C2/C4/C5/C6 tests are
mock-scripted (no network). The grep assertions in C0 and C5 are run
literally. A live smoke — `fleet run --model ollama-cloud/glm-5.3-flash
"say hi"` serving on that pair, and a `fleet run` with every plan key
unset serving on deepseek-flash as the named tail — is the owner's call,
since it spends plan tokens.

## Linked artifacts

`9c72bbb` (C0). `docs/plans/2026-10-16-orchestrator-routes-hard.md` (the
ruling this plan narrows to the leader).
`docs/plans/2026-10-16-fleet-routes-through-its-own-router.md` (the router
ruling). `docs/plans/2026-10-15-router-enforcement.md` (the `composed` leg
C8 defers to). `docs/features/routing.md`.
