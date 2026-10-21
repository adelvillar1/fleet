---
status: completed
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

- [x] **C0 — the reserve is deepseek-flash and every roster model has a window.** `grep -rn deepseek-chat src tests README.md docs/features` prints nothing; `api::DEEPSEEK_MODEL` (renamed under C5) is `"deepseek-flash"`; `context_window` returns `Some` for every HTTP model `default_roster()` serves; the window test's named-absent list is exactly `["blend1/blend1"]`. *(`9c72bbb`, ratified by this plan's approval.)*
- [x] **C1 — the tail is in every order.** `select`, `fallback` and `workload_decision` return an `order` whose last element is `("deepseek", "deepseek-flash")` whenever the deepseek candidate is enabled and `gates.allow_api_models` is true; the pair appears exactly once — a `routing.json` workload that already places it keeps the owner's position; with `gates.allow_api_models: false` no order contains it, pin included. `route::with_tail`; pinned by `every_order_the_router_builds_ends_on_the_reserve_tail` and `a_workload_that_already_places_the_reserve_keeps_the_owners_position`. A found-and-fixed edge: an empty preference table must decide its head walk *before* the tail is appended, or the reserve becomes the head of a walk nobody declared (`an_empty_table_walks_fleets_own_chain_and_nothing_outside_it` caught it).
- [x] **C2 — the tail is named when it serves.** The reserve is the last entry of every order (C1), so `Router::chat_stream` reaches it only after every routed candidate failed with a failover class; when it serves, `Router::row` ends the reason with `; served by the failover tail deepseek-flash`, and a reserve chosen by a pin or a workload carries no clause. When the tail fails too the run fails with the last error, unchanged. Pinned by `the_tail_is_named_on_the_row_only_when_it_served_as_the_tail` over the walk state `chat_stream` leaves.
- [x] **C3 — the leader keeps `hard`.** `select` is `select_walk(.., Walk::Leader)`: a top-level run still walks `ORCHESTRATOR_WORKLOAD`'s chain; the orchestrator tests are green unchanged, `route_cache::Cache::key_for` is unchanged.
- [x] **C4 — a child is the router's.** `Delegation` carries the parent's pin (read off the parent's `Router`) and lane specs; spawning a child calls `route::prepare_child` over the child's composed brief with the parent's pin and the child runs on `Client::routed(<its own router>)`, not the parent's client. The child's walk is the **judged** workload's chain (`Walk::Judged`, no orchestrator override): with a scripted sys1 judging `quick`, the leader's order heads `hard`'s chain and the child's heads `quick`'s (`a_child_walks_the_judged_workload_while_the_leader_keeps_hard`, `the_leader_walks_hard_and_a_child_walks_what_was_judged`); the child's decision row lands in the calibration stream and its session `start` record names the child's route model. *Named exception, KI-278:* a run with no home — a scheduled run, whose `improve_home` is deliberately `None` — or a parent on a direct, unrouted client (every test double) has no router to rule the child, which keeps the parent's client.
- [x] **C5 — no nominal model is hardcoded.** `api::DEFAULT_MODEL` is renamed `api::DEEPSEEK_MODEL` and referenced outside tests only by the roster's deepseek entry and the window table; `route::AUTO_MODEL` is `"auto"` and is what `AgentOptions::default()`, the TUI and GUI launchers seed; `fleet run` opens its session with the route decision's model (kept), and a scheduled run's session takes the model of the route `run_job` prepared before it; a `fleet chat` log, which opens before its first message exists, starts as the pinned pair's model or `auto`, and its end records name what served. *(Amended mid-build per the plans README: a chat or face log opens before any task exists, so "the decision's model" is the run path's property and `auto` is the truthful start of a session.)*
- [x] **C6 — `--model` is a pin.** `fleet run --model <id>` and `fleet chat --model <id>` resolve `<id>` through `route::resolve_model_pin`: `provider/model` is the exact pair; a bare model id maps to the one roster provider that serves it, in the roster's own spelling; the resolution is refused by name when no provider serves the id or more than one does, listing every `provider/model` spelling. The resolved pin reaches `prepare_with_pin`. The help text describes `--model` as a pin whose absence means the router decides. `a_model_flag_resolves_to_a_pin_or_is_refused_by_name` pins the resolutions; the CLI parse tests pin `None` as the default.
- [x] **C7 — the docs say what the code does.** `CLAUDE.md`: the "Today's state" DeepSeek bullet names the reserve as every walk's failover tail, and hard rule 9 reads that the leading session walks `hard`'s chain while a child is routed by the router over its brief and inherits the parent's pin. `docs/features/routing.md` gains "Children", "The failover tail" and "Pins and the nominal model"; `docs/features/subagents.md` drops "inherits the parent's model". `src/route.rs`'s module doc, `Candidate::billing`, `Billing`, `Preference` and `default_preferences` comments no longer claim the reserve cannot serve unpinned. The full gate — `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build --release`, `cargo test` — exits 0 four times.
- [x] **C8 — deferred, named.** `composed` stays unreachable by judgment: sys1's `route` task emits five labels and `composed` is not one; the lane is vendored and the label is upstream's. Target: a follow-up plan over `services/sys1` and a re-vendor. Effort-per-workload is its own plan (research recorded 2026-10-21). Neither is built here.

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
