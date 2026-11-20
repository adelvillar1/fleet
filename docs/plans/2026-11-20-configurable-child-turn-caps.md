---
status: completed
created: 2026-11-20
updated: 2026-11-20
slug: configurable-child-turn-caps
---

# Configurable child turn caps, defaulting to 100

The owner's request, verbatim: *"let's make delegated child process caps
configurable, and let's up the cap to 100"*.

The existing knob does not do what its own comment says it does. This plan
fixes that before it adds anything.

## Context

Four numbers govern a delegated child's turns, and only one of them is
reachable by the owner.

| Constant | Value | Where | Reachable? |
|---|---|---|---|
| `SUBAGENT_TURN_CAP` | 20 | `src/subagents.rs:77` | **no** — hard-coded |
| `SUBAGENT_MAX_TURNS` | 50 | `src/subagents.rs:90` | **no** — hard-coded, and it is the schema's `maximum` too (`src/tools.rs:186`) |
| `SUBAGENT_TURN_CAP_FLOOR` | 5 | `src/subagents.rs:84` | no (correctly: it is a floor, not a knob) |
| `delegation.max_turns` | 100 | `src/settings.rs:196` | yes — but it bounds **nothing on the delegation path** |

The `delegation.max_turns` setting reads into `route::RunConfig::max_turns`
(`src/route.rs:943-966`) and is then consumed **only** as the main session's
`options.max_turns` (`src/gui/bridge.rs:15606-15668`), where the loop is
`capped: false` (`src/agent.rs:3403`: `while !self.options.capped || …`). So
on the one path it reaches, it has no effect; and on the delegation path —
where the cap *is* real — the setting is never read. The `agent.rs:157` comment
("this constant is the SUBAGENT path's fallback ceiling, sourced through
`delegation.max_turns`") describes a wiring that does not exist.

A child's effective cap today is: whatever `max-turns` the call asked for
(default `SUBAGENT_TURN_CAP` = 20, refused above 50), then
`inherit_turn_cap` clamps it to `turn_ceiling(depth)` (`src/subagents.rs:123`),
and at depth ≥ 2 to `turn_cap_for_depth` and the parent's remaining turns
(`src/subagents.rs:140-149`, `:184`).

So "up the cap to 100" means three concrete moves: make the **default** 100,
make the **ceiling** 100 (so 100 is actually askable — today 51+ is refused by
name), and make both **read from config** so the owner can change them without
a rebuild.

## Approach

One new settings row carries both numbers, and one resolver reads it. The
constants stay as the compiled-in fallback, so an unconfigured home behaves
exactly as this plan says and `settings.unset` returns to a named value.

1. **Two settings rows, not one.** `delegation.max_turns` already exists and is
   documented as "the turn bound a delegated child runs under" — the fix is to
   make it true for the *default*, and add `delegation.max_turns_ceiling` for
   the *maximum askable*. Two rows because they answer two questions, and
   because collapsing them would mean an owner who raises the ceiling silently
   raises every unasked delegation with it (and vice versa).

2. **`const` → `fn`, keeping the name.** `SUBAGENT_TURN_CAP` and
   `SUBAGENT_MAX_TURNS` become resolver functions that take the resolved
   config and fall back to the constants. The call sites are few
   (`src/subagents.rs:110`, `:1179`, `:1182`, `src/delegate.rs:347`) and each
   one already has a home in scope or can be given one.

3. **The schema follows the config.** `src/tools.rs:186` hard-codes
   `"maximum": 50`, and `src/tools.rs:161`'s description says "default 20 …
   at most 50". Both must render the resolved numbers, or the model is told a
   bound the harness does not enforce — the exact failure the schema comment at
   `src/tools.rs:154-157` was written to prevent. `subagent_spec()` gains the
   resolved pair as an argument; `base_declarations` threads it through.

4. **A floor and a sanity bound on the config.** A ceiling below the default
   is refused by name (it would make the default unaskable and the effective
   cap the ceiling anyway). A zero or absurd value is refused the way
   `run_config` already refuses `turns == 0` (`src/route.rs:956-963`).

Depth behavior is unchanged: depth 1 gets the default, depth ≥ 2 still halves
and floors at `SUBAGENT_TURN_CAP_FLOOR`, and the parent-remaining clamp still
applies. Raising the base to 100 therefore gives a depth-2 child 50 where it
had 10 — which is why the halving rule stays and the floor stays.

## Acceptance Criteria

- [x] `delegation.max_turns` set to `120` in `config.json` makes an unasked
      depth-1 delegation run with a 120-turn cap, proven by a test that
      resolves the config and asserts the cap.
- [x] `delegation.max_turns_ceiling` set to `150` lets a call ask for
      `max-turns: 140` and succeed; asking `151` is refused by name quoting
      `150`, not `50`.
- [x] With no config, the default child cap is **100** and the default
      ceiling is **100**, asserted against the constants rather than literals.
- [x] The `subagent` tool's declared schema renders the resolved values: with
      the default home, `maximum` reads 100 and the description says "default
      100 … at most 100"; the existing schema pin test
      (`src/tools.rs:5432-5434`) is updated to read the resolver, not `50`.
- [x] A ceiling below the default is refused at read time with a message
      naming both numbers.
- [x] Depth behavior is unchanged: `turn_cap_for_depth(1)` is the default,
      `turn_cap_for_depth(2)` is half, `turn_cap_for_depth(9)` is the floor —
      all three already pinned by `src/subagents.rs:3414-3437`, and those tests
      keep passing with updated expectations.
- [x] `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`,
      `cargo build --release`, `cargo test` all exit 0.

## Files to be touched

- `src/subagents.rs` — the constants become resolvers; `SubagentRequest::parse`
  gains the resolved ceiling; `run` keeps `inherit_turn_cap` unchanged.
- `src/route.rs` — `RunConfig` gains the two child fields and `run_config`
  reads them with the validation above.
- `src/settings.rs` — the `delegation.max_turns` summary is corrected and
  `delegation.max_turns_ceiling` is added, with its `compiled_default` entry.
- `src/tools.rs` — `subagent_spec` takes the resolved pair; the description and
  the `maximum` render it; the pin test follows.
- `src/agent.rs` — `base_declarations` threads the resolved pair into
  `subagent_spec`.
- `src/delegate.rs` — the one `turn_ceiling(1)` call site passes the config.
- `docs/features/subagents.md` — the cap paragraph states the new default and
  the two settings.
- `docs/architecture/structure-guards.md` — untouched unless a bound moves; the
  tool-surface bounds do not change.

## Out of scope

- Removing the depth halving or the floor. They exist for a reason (a depth-2
  brief is a paraphrase of a paraphrase) and the owner asked for the cap, not
  the depth model.
- Any change to what bounds the **main** session. It stays uncapped by turns
  (the 2026-11-05 ruling); `delegation.max_turns` stops pretending to bound it.
- The finding in `docs/architecture/deep-dive-2026-11-20-oh-my-hermes.md` about
  write-scope enforcement. Separate work.
- Making `SUBAGENT_TURN_CAP_FLOOR` configurable. A floor the owner can lower to
  zero is not a floor.

## Verification

1. `cargo test subagents::` and `cargo test tools::` first, scoped, while
   iterating.
2. A scratch home with `delegation.max_turns: 120` and
   `delegation.max_turns_ceiling: 150`, driven through `SubagentRequest::parse`
   and the resolver, asserting both numbers and the refusal above the ceiling.
3. The full gate, read by exit code.

## Linked artifacts

- `docs/architecture/deep-dive-2026-11-20-oh-my-hermes.md` — the dive that
  surfaced the unwired setting while checking Fleet's own claims.
- `docs/features/subagents.md:80` — the current document of the cap.
