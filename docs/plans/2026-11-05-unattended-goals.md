---
status: active
created: 2026-11-05
updated: 2026-11-05
slug: unattended-goals
---

# Plan: the cap leaves the main session — goals drive unattended work

## Context

The owner's report (2026-11-05), and then the owner's ruling that
reshapes it: the turn budget stops goals from being completed
unattended; setting a goal to implement a plan does nothing; and — the
ruling — **the cap should not exist on a main session at all**. From a
user's perspective it adds no value, and hitting it is
counterproductive in spirit. If it was built on the assumption that a
turn count preserves context, that assumption is dead: **context
health is what matters, not turn count** — the fold, the
budget-following-model, and the survival check landed 2026-11-04. If
the cap earns its keep on delegated children, it stays there.

The verification:

- **The cap's origin**: slice 1 (`84ba44a`) — a cost guard from the
  first commit, whose law is "an unbounded loop is a quota fire, not
  an agent" (`src/agent.rs:142-146`). It was never a context
  mechanism; the context job it was a crude proxy for is now done by
  machinery that measures the real thing.
- **The main-session facts**: `DEFAULT_MAX_TURNS = 100`
  (`src/agent.rs:147`); config `runs.max_turns` can only lower
  (`src/route.rs:866-880`); the face's per-run body override tightens
  only; reaching the cap ends the run with the `max-turns` event and
  a receipt that waits for a human press (`app.js:2465-2495`).
- **The goal facts**: `/goal` and the goal pop store the standing
  objective; it rides every run's prompt (`agent.rs:435`, `1881`,
  `2011`) — but **nothing starts a run from it**. A goal set with no
  run in flight sits until the owner happens to send a task.
- **Delegation's cap is load-bearing — it stays.**
  `inherit_turn_cap` (`src/subagents.rs:140-150`) bounds a child by
  the depth-stepped ceiling (`turn_ceiling`: depth 1 keeps
  `SUBAGENT_MAX_TURNS`, depth 2+ halves) and by the parent's
  remaining budget — "a child that outlived its parent's budget
  would spend turns on a run whose own answer is already written."
  The mechanism already has the uncapped-parent path built:
  `parent_remaining = None` leaves the depth-stepped ceiling holding
  on its own.

So the shape of the work: **the main session's loop runs to its
natural end** — the model's final report, the owner's stop, a
blocker, an approval wait — with context health carrying the long-run
burden; **delegation keeps its cap** as the technical bound it
provably is; and **goals drive the work**: a goal starts a run, a
tool completes it, and the face tells the truth about all of it.

## The documented contracts this plan amends

The arch-gate flags these; they are deliberate, at the owner's
ruling.

1. **The cap on the main session** (`agent.rs:142-147`, the
   continuation receipt's contract in the face): the law was "an
   unbounded loop is a quota fire, not an agent." The amendment: on
   the main session the loop is unbounded by turns — the quota-fire
   bound moves to where it is technical (delegation, kept) and to the
   owner's stop, which is always in hand. The quota-fire law is not
   deleted; it is scoped to the delegation branch, and the residual
   risk on main is named in Risks.
2. **"Config can only lower"** (`route.rs:866-880`): with the main
   cap gone, `runs.max_turns` becomes the delegation child's bound
   and moves to the delegation scope (`delegation.max_turns`); the
   owner's config value is authoritative there, and the face's
   per-run budget override — a main-session construct — is removed
   with the cap it served.

Everything else — the desk evidence spine, the graph gates, the
approval machinery, the fold — is untouched. The uncapped loop rides
the same run path and asks the same seats.

## Approach

1. **The main loop loses the cap.** The loop condition runs to the
   natural end: the model's report with no tool calls, the owner's
   stop, a refusal, or an approval wait. `DEFAULT_MAX_TURNS` leaves
   the main path; the turn *counter* stays (checkpoints and the log
   are keyed on it — it is bookkeeping, not a bound). The
   `max-turns` event, the cap receipt, and the "continue the work"
   button die with it; the receipt remains for the ends that are real
   (failed, stopped by the owner).
2. **Delegation keeps the cap, explicitly and configured.** The child
   bound moves to `delegation.max_turns` in config (the owner's value
   authoritative; the depth-stepped ceiling stays as the structure —
   `turn_ceiling` unchanged), and `parent_remaining` reads `None` on
   an uncapped parent — the existing path, now the only one, pinned
   by the inheritance tests unchanged.
3. **The goal starts work.** Setting a goal on a session with no run
   in flight starts a run whose task is the goal itself — the
   owner's words as the first instruction, the ordinary run path
   (approvals, checkpoints, the log). With a run in flight, the goal
   rides as today and the set reply says so.
4. **The goal completes, explicitly.** The `goal` agent tool (show /
   complete / clear), declared on every run like `remember`:
   `complete` writes the done state to the goals store **and the
   log** (hard rule 5), and the face names it — the chip goes green.
   The run's own end stays the model's report; the goal's completion
   is a fact, not an inference.
5. **The pending-goal defect.** A goal set on the welcome rides the
   pending-new path: it attaches to the session the first send
   creates. The launch-session mapping dies.
6. **The face tells the truth.** The budget card and the settings'
   `runs.max_turns` row leave the main surfaces; the delegation
   setting appears where the cap now lives; the goal chip shows
   active / done; the working line's context-health readout —
   occupancy, folds, turns-to-fold — is the long-run signal, as it
   already is.
7. **Prove it on a scratch home**: a goal set ("create a file with
   exactly this content, then mark the goal complete"), the run
   driving unattended past where turn 100 would have killed it, the
   tool completing the goal, the run ending naturally; then a
   delegated child showing its own cap still set; every observation
   quoted.
8. **Supervision becomes machinery** (the owner's ruling: a
   non-delivering task is a failed node the parent completes or
   kills-and-respawns): a child bound to a plan node dispatches the
   node on spawn; a child ending `MaxTurns` or `Failed` **fails or
   demotes the node automatically** with the outcome as the named
   reason — the vocabulary exists (`graph.fail`, `graph.demote`,
   `settle`, where a fact outranks a failed report) but today the
   transition is left to the parent's model noticing text. The
   parent's rendered result names the node's new state.
9. **Absorb or respawn — the model chooses, the machinery bounds.**
   A demoted node is re-offered by `admit_now` (already built); a
   per-node re-dispatch ceiling (2) bounds the respawn loop — beyond
   it the node stays failed and the parent's remaining options are
   absorb the work itself or surface the node to the owner by name.
   The quota-fire law moves down a level: the main loop is uncapped,
   a runaway *branch* is not.
10. **Pin and gate**: the harness cases ride the face work; the gate
    runs whole before any push carrying code — `cargo fmt --all
    --check`, `cargo clippy --all-targets -- -D warnings`,
    `cargo build --release`, `cargo test`, exit codes read directly.

## Acceptance criteria

- [ ] **C1 — the main session is uncapped.** The loop's condition no
      longer consults a turn ceiling; a run ends on the report, the
      owner's stop, a refusal, or an approval wait; the `max-turns`
      event and the cap receipt are gone from the main path. Pinned
      by a test running past the old default.
- [ ] **C2 — delegation keeps its cap.** Children are bounded by
      `delegation.max_turns` (config-authoritative) under the
      depth-stepped ceiling; an uncapped parent yields
      `parent_remaining = None` and the ceiling holds; the
      inheritance tests pass unchanged. Pinned.
- [ ] **C3 — the settings move.** `delegation.max_turns` replaces
      `runs.max_turns` in the settings surfaces; the face's per-run
      budget override is gone. Pinned by a settings test.
- [ ] **C4 — the goal starts.** Setting a goal on a session with no
      run in flight starts the run with the goal as the task; with a
      run in flight, the set reply names that the goal rides the next
      turn. Pinned by a test.
- [ ] **C5 — the goal tool.** The `goal` tool exists (show /
      complete / clear), is declared on every run, writes the
      completion to the store and the log, and the face names the
      completion. Pinned by tests.
- [ ] **C6 — the pending goal.** A goal set on the welcome attaches
      to the session the first send creates. Pinned by a test.
- [ ] **C7 — the face tells the truth.** The cap card is gone; the
      goal chip shows active / done; the context-health readout is
      the long-run signal. Pinned by harness cases.
- [ ] **C8 — delegation is byte-identical where a parent is capped.**
      The existing inheritance behavior for a capped parent passes
      the suite unchanged — the cap moved, its delegation semantics
      did not.
- [ ] **C9 — the harness.** `tests/face/harness.mjs` gains the goal
      chip's done / active cases and the receipt-without-cap. Harness
      green with the new cases counted.
- [ ] **C10 — the live proof.** On a scratch home: the goal's run
      driving past turn 100 unattended, the tool completing the goal,
      the natural end; a delegated child showing its own cap; every
      observation quoted in the recap.
- [ ] **C12 — the supervision wiring.** A child bound to a plan node
      dispatches the node on spawn; a child ending `MaxTurns` or
      `Failed` marks the node automatically — `fail` or `demote` with
      the outcome as the reason — and the parent's rendered result
      names the node's state; `settle` still verifies on landed facts
      regardless of the mark. Pinned by tests over the child-end
      path.
- [ ] **C13 — the respawn bound.** A per-node re-dispatch ceiling
      (2): beyond it the node stays failed, and the parent's options
      are absorb or surface the node to the owner by name — the
      quota-fire law at the node level, pinned by a test driving the
      ceiling.
- [ ] **C14 — the seam surfaces.** The plan graph's card names the
      failed / demoted nodes with their reasons, and the parent's
      result names the choice taken (absorbed / respawned /
      surfaced). Pinned by harness cases.
- [ ] **C11 — the gate.** fmt, clippy `-D warnings`, release build,
      `cargo test` — exit codes read directly, the whole suite.

## Files to be touched

- `src/agent.rs` — the loop's uncapped condition, the `goal` tool
  declaration and handling, the `max-turns` event's removal from the
  main path.
- `src/subagents.rs` — the child bound's config source
  (`delegation.max_turns`), the uncapped-parent path pinned, and the
  supervision wiring: child spawn dispatches the bound node, child
  end marks it, the re-dispatch ceiling.
- `src/plan_graph.rs` — the demote/respawn helpers' call sites and
  the ceiling's home.
- `src/route.rs` — the clamp's removal from the main path; the
  delegation value's read.
- `src/settings.rs` — the settings' move.
- `src/gui/bridge.rs` — the goal-starts-work path, the pending-goal
  mapping, the completion store write, the per-run budget override's
  removal.
- `src/gui/server.rs` — the routes the goal tool and the settings
  need.
- `src/gui/assets/app.js`, `app.css`, `index.html` — the cap card's
  removal, the goal chip, the settings rows.
- `src/tools.rs` — the `goal` tool's schema.
- `tests/` — the pins; `tests/face/harness.mjs` — the cases.

## Out of scope

- A token-budget replacement (the 2026-11-03 deferred question) —
  context health *is* the budget now; this plan retires the turn
  proxy on main.
- The huddles rooms' goals — rooms already drive their own turns.
- Cron scheduling — a goal-driven run is within-session.
- A spend meter or wall-clock guard on the uncapped loop — named
  here so the omission is a decision: the owner's stop is the bound,
  and if a meter is ever wanted it is its own increment.

## Verification

```sh
# the pins and the suite
cargo test

# the face
node tests/face/harness.mjs

# the gate (exit codes read directly)
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo build --release
```

The live proof drives the release binary on a scratch home and quotes
every observation: the goal's run starting, running past turn 100,
completing through the tool, ending naturally; a delegated child's
cap holding; the receipt without a cap card.

## Linked artifacts

- `docs/features/tools.md` — the `goal` tool.
- `docs/features/gui.md` — the receipt without the cap, the goal chip.
- `FUNCTIONAL-SPECIFICATIONS.md` — run behavior with a goal.
- `TECHNICAL-DOCUMENTATION.md` — the settings' move and the loop's
  contract.
- `docs/recaps/` — the proof.

## Risks

- **The pathological loop.** An uncapped main run that models in
  circles burns quota until the owner stops it. The honest mitigations
  are the ones that remain: the model's own stop discipline, the fold
  keeping context sane across however many turns it takes, approvals
  still asking, and the owner's stop in every mode. The residual is
  accepted by this ruling, named here so it is a decision.
- **Checkpoints over very long runs**: a checkpoint per turn is the
  existing cadence; a 300-turn run makes 300 of them. The checkpoint
  cap and the archive's own bounds already handle volume; the proof
  watches for it.
- **The launch-session mapping's other users**: `set_goal`'s
  empty-session mapping serves the huddles' goal path too — the fix
  scopes to the pending-new case and pins the huddles path unchanged.
- **Delegation's uncapped-parent path goes from rare to always** —
  the `None` branch is the one every delegation now takes; its tests
  (C2/C8) are what make that safe.
- **The respawn ceiling is a judgment**: too low and a hard node
  surfaces to the owner prematurely; too high and a doomed node
  burns child budgets. Two is the starting number, named in code
  like the floors — the calibration stream is where it gets fit.
