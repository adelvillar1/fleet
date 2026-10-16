---
status: completed
created: 2026-10-16
updated: 2026-10-16
slug: orchestrator-routes-hard
---

# Plan: the orchestrator routes through `hard`

## Context

The owner's ruling (2026-10-16): the orchestrator — the first model the
router serves on a run — must be a strong model, whatever the roster
assigns to the `hard` category, because that model is the planner: it
assigns the work and supervises. Today `route::select` walks the chain
of the workload sys1 judges the task to be, so a task judged `quick`
plans with `step-3.7-flash` and one judged `composed` plans with
`blend1`. Every caller of `route::prepare`/`route::select` — `fleet
run`, the chat loop, compaction, cron jobs, the TUI — is an orchestrator
selection; the lanes never route through `select` (their launch specs
name their own models). Delegation children inherit the parent's model
(hard rule 9), so strengthening the orchestrator's first selection
strengthens the whole run.

## Approach

The walk leaves the judged label at one point: `select` resolves the
walked chain from the compiled-in `ORCHESTRATOR_WORKLOAD` ("hard")
instead of the judged label, falling back to the judged label's chain
only when the table names no `hard` at all. Everything else keeps its
meaning:

- The sys1 judgment still runs. The decision row's `workload` field
  stays the judged label — `fleet judge` grades the judgment against
  outcomes, and that grading must not read a relabeled row.
- The reason names both halves: the judged workload and, when the walk
  diverges from it, that the orchestrator walked `hard`'s chain. The
  pin path composes the same way.
- A `routing.json` `workloads` override of `hard` governs the walk —
  an override replaces `hard`'s list, and the orchestrator follows,
  which is what "whatever is assigned to the hard category" means.
- The route-decision cache key folds the walked workload in, so
  pre-change entries cannot serve a weak head: the key format changes,
  the cache invalidates whole, and the first run re-judges.
- Demotion and failover apply to the walked chain exactly as before —
  demotion reorders this session's walk, never the selection.

## Use cases

- [x] A: A task sys1 judges `quick` is planned by `hard`'s head
      (`stepfun/step-5-preview` today), with the reason naming both.
- [x] B: The owner overrides `hard`'s candidates in `routing.json`;
      the next orchestrator walk follows the override.
- [x] C: A cache written before this change serves nothing; the first
      run after it re-judges and re-walks.
- [x] D: A pinned run still judges, records, and walks — filtered to
      the pin, with the divergence named.

## UX routes

No new surfaces. The route row's reason line and the router board's
rows carry the divergence; the TUI status bar's routing-mode line is
unchanged.

## Acceptance criteria

- [x] B1: `route::select` walks `ORCHESTRATOR_WORKLOAD`'s ("hard")
      chain, falling back to the judged workload's chain only when the
      table names no `hard`; the sys1 judgment still runs and the row's
      `workload` field stays the judged label.
- [x] B2: the reason names the judged workload and, when the walk
      diverges, that the orchestrator walked `hard`'s chain; the pin
      path composes.
- [x] B3: a `routing.json` `workloads` override of `hard` governs the
      orchestrator's walk.
- [x] B4: the cache key folds the walked workload in; a pre-change
      cache serves nothing and self-heals on the first run.
- [x] B5: demotion reorders the walked chain and never the selection,
      unchanged; failover walks the strong chain.
- [x] B6: the gate runs whole — `cargo fmt --all --check`,
      `cargo clippy --all-targets -- -D warnings`,
      `cargo build --release`, `cargo test` — exit codes read directly.
- [x] B7: the records say the ruling — `docs/features/routing.md`,
      `docs/harness-orientation.md`'s environment section,
      `CLAUDE.local.md`'s roster paragraph, and a corpus fact.

## Files to be touched

- `src/route.rs` — `ORCHESTRATOR_WORKLOAD`, the walk, the reason
  composition, tests
- `src/route_cache.rs` — the key format, tests
- `docs/features/routing.md` — the orchestrator paragraph
- `docs/harness-orientation.md` — the environment bullet
- `CLAUDE.local.md` — the roster paragraph
- `memories/orchestrator-walks-the-hard-chain.md` — new fact

## Out of scope

- Making the orchestrator's workload itself configurable
  (`routing.json` key) — the const is one line; the field is
  speculative until the owner asks.
- Lane-side model choices (launch specs already name their own).
- Any change to the judgment gates or the confidence floor.

## Verification

**B1/B2:** unit tests — a `quick`-judged task's decision walks
`hard`'s candidates with the reason naming both; a `hard`-judged task's
reason is the existing shape; a table without `hard` falls back.

**B3:** the existing override test pattern — override `hard`'s list in
a routing file, assert the walk follows.

**B4:** a cache file written under the old key does not match; one
written under the new key does.

**B5:** the demotion tests' order assertions survive unchanged.

**B6:**
```bash
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
cargo fmt --all --check && cargo clippy --all-targets -- -D warnings \
  && cargo build --release && cargo test
```

## Linked artifacts

- `docs/features/routing.md` — the orchestrator paragraph
- `docs/harness-orientation.md` — the environment section
- `memories/orchestrator-walks-the-hard-chain.md` — the ruling as fact

## Risks

- Quota burn on the strong model rises: every run plans on `hard`'s
  head regardless of task shape. That is the ruling's stated intent
  (the planner supervises), recorded here rather than argued with.
- Eval scenarios asserting per-workload heads shift to `hard`'s head;
  the eval corpus's expectations are updated with the same increment,
  not papered over.

## Dependencies

- `docs/plans/2026-10-16-fleet-routes-through-its-own-router.md` — the
  in-process router this ruling tunes.
- CLAUDE.md hard rule 9 (delegation inherits the parent's base model) —
  unchanged; this plan is what makes the inherited model the strong
  one.

## Convergence (graded 2026-10-16)

Commit `69c7d0d`. The gate ran whole at the increment: all four legs
exit 0, 1022 passed / 1 ignored, exit codes read directly.

- **B1 met** — the walk leaves the judged workload at one point:
  `walked` resolves to `ORCHESTRATOR_WORKLOAD` when the table names it,
  else the judged label; the judgment still runs and
  `decision_from(..., Some(workload.clone()), ...)` keeps the judged
  label in the row. Pinned at the unit level by the updated fallback
  and pin tests (both walk `hard`'s chain now) and at the integration
  level by `the_orchestrator_walks_hard_whatever_the_task_was_judged`
  (a `quick`-judged task whose row records `quick` and whose order is
  `hard`'s own).
- **B2 met** — the reason composes `sys1 judged \`<label>\`; the
  orchestrator walks \`hard\`'s chain; ` when they diverge, and the pin
  path composes the same way; asserted in the integration test.
- **B3 met** — `the_orchestrators_walk_follows_a_hard_override` drives
  an overridden `hard` list through select; the two rewired lane tests
  exercise the real `routing.json` override path through `prepare`
  (the owner's `hard` override aims the walk at blend1).
- **B4 met** — `key_for` appends `walk=<ORCHESTRATOR_WORKLOAD>`; the
  pre-change key format (the bare producer key) no longer matches, and
  `the_cache_key_names_the_walked_workload` pins both the new key's
  shape and the old format's refusal.
- **B5 met** — the demotion reorder and failover tests pass unchanged;
  the rewired refusal test proves the walk still advances by name
  past a failing candidate, on the orchestrator's chain.
- **B6 met** — the gate, exit codes direct.
- **B7 met** — `docs/features/routing.md` carries the orchestrator
  paragraph, `docs/harness-orientation.md`'s environment bullet names
  the ruling, `CLAUDE.local.md`'s roster paragraph says it, and
  `memories/orchestrator-walks-the-hard-chain.md` records it.

The named risk held: two walk-mechanics tests (`a_composed_judgment_answers_through_the_blend1_lane`,
`a_lane_that_refuses_advances_the_walk_by_name`) asserted
per-workload chains and were rewired onto the ruling — their fixtures
override `hard` through `routing.json`, which is also the B3 path, and
their walk/failover mechanics pass unchanged. The eval corpus needed no
change: its scenarios assert against the mock roster where the walked
head was already the assertion's subject.
