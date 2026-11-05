---
status: draft
created: 2026-11-05
updated: 2026-11-05
slug: unattended-goals
---

# Plan: unattended completion — the goal drives the work, the budget stops nagging

## Context

The owner's report (2026-11-05): the turn budget stops goals from
completing unattended, and setting a goal to implement a plan does
nothing. Both halves verified in code:

- **The budget**: `DEFAULT_MAX_TURNS = 100` (`src/agent.rs:147`);
  config `runs.max_turns` can only lower it — a named value above the
  compiled default clamps (`src/route.rs:866-880`) — and the face's
  per-run body override tightens only. Reaching the cap ends the run
  with the `max-turns` event; the face shows the continuation receipt
  and **waits for a human press** (`app.js:2465-2495`). An unattended
  goal dies at turn 100 by design.
- **The goal**: `/goal` and the goal pop store the standing objective
  (`bridge.rs` `set_goal`, the goals store, `GOAL_MAX_CHARS`), and it
  rides every run's prompt (`agent.rs:435`, `1881`, `2011`) — but
  **nothing starts a run from a goal**. A goal set while no run is in
  flight sits in the store until the owner happens to send a task. The
  "implement this plan" goal does nothing, exactly as reported.
- **A third defect found on the way**: `set_goal` maps an empty
  session to `launch()` — the *latest* session. A goal set on the
  welcome (before any conversation is opened) lands on whatever
  conversation the owner had last, not the one they are about to
  create. The comment claims the goal "moves with it" on the first
  task; the mapping does not.

The design keeps what the cap is for — a bounding knob, not a law —
and adds what it never had: a way to keep working when the owner is
not there to press continue. Context health already keeps long runs
sane (the fold, the budget following the model); the unattended chain
is the cost story on top of it, not instead of it.

## The documented contracts this plan amends

The arch-gate is right to flag drift: two documented contracts change
here, deliberately, at the owner's ask ("the turn budget stops goals
from being completed unattended").

1. **"Config can only lower"** (`src/route.rs:866-880`, and the
   settings surface's own wording): a `runs.max_turns` above the
   compiled default clamps to it, because a bound is not a wish. The
   amendment: the value's author **is** the owner — config.json is
   the owner's own file, and an explicitly named raise is the owner
   raising their own bound, which the clamp was never protecting
   against. The face's per-run body override keeps its tighten-only
   rule unchanged (that clamp guards a different boundary: a run's
   choice against the standing policy).
2. **The cap ends the run** (`src/agent.rs:142-146`, the continuation
   receipt's contract in the face): reaching the cap produces a
   continuation receipt and waits. The amendment: under unattended
   mode with a goal, the receipt's continue is **sent by the bridge**
   instead of waiting for the press — the same hand-off, the same
   checkpoints, a visible chain, bounded by a new setting, and the
   owner's stop still ends everything. Defaults off: without the
   setting, today's behavior is byte-identical (C8).

The desk evidence spine, the graph gates, and the approval machinery
are untouched — the chain rides the same run path and asks the same
seats.

## Approach

1. **The config grows up.** `runs.max_turns` may now **raise** above
   the compiled default when the owner names it in config.json — the
   clamp's "config can only lower" rule is amended: owner-authored
   config is authoritative. The face's per-run body override stays
   tighten-only (a run's choice, not a standing policy — the existing
   cleared-after-read rule). Two new settings: `runs.unattended`
   (bool, default false) and `runs.unattended_continuations`
   (default 10) — the chain's bound. `fleet settings` and the face's
   settings surface carry all three.
2. **The chain.** When a run ends at the cap — or finishes with the
   goal not marked complete — and the session carries a goal and
   `runs.unattended` is on and continuations remain, the bridge
   **continues automatically**: the same hand-off the receipt's
   continue button builds, sent as the next run on the same session.
   Every continuation emits an event the face renders (the chain is
   visible, count remaining), and a checkpoint per continuation rides
   the existing machinery. The owner's controls win: a stop pauses
   the chain by name (the receipt says "paused — continue resumes the
   chain"), a steer rides the next continuation.
3. **The goal starts work.** Setting a goal on a session with no run
   in flight starts a run whose task is the goal itself — the owner's
   own words, the session's standing objective as the first
   instruction. If a run is in flight, the goal rides as it does
   today, and the set reply says so. The starting run is the ordinary
   run path: approvals, checkpoints, the log.
4. **The goal completes, explicitly.** A small agent tool — `goal`
   (show / complete / clear) — declared on every run like `remember`.
   `complete` marks the goal done in the goals store **and the log**
   (hard rule 5), ends the chain, and the face names it: the chip
   goes green, the receipt says the goal is complete. Completion
   survives a restart because the store carries it. Without this, a
   chain would only know how to stop, never how to finish.
5. **The pending-goal defect.** A goal set with no session rides the
   pending-new path: it attaches to the session the first send
   creates, not to the launch session. The claimed behavior becomes
   the tested behavior.
6. **The face tells the truth about all of it**: the receipt in
   unattended mode shows the chain's state (continuations left, the
   goal it serves); the goal chip shows done/active; a paused chain
   explains that continue resumes it. No silent anything.
7. **Prove it on a scratch home**: a goal set unattended ("create a
   file with exactly this content, then mark the goal complete"), the
   chain running through at least one cap continuation, the tool
   completing the goal, the chain ending, every observation quoted.
   Then the stop path: a stop mid-chain pauses, continue resumes.
8. **Pin and gate**: the harness cases ride the face work (C9), the
   scratch proof above is C10's record, and the gate runs whole
   before any push carrying code (C11) — `cargo fmt --all --check`,
   `cargo clippy --all-targets -- -D warnings`,
   `cargo build --release`, `cargo test`, exit codes read directly.

## Acceptance criteria

- [ ] **C1 — config raises.** `runs.max_turns` above the compiled
      default is honored from config.json (the clamp amended at
      `route.rs`); the face's per-run body override still only
      tightens; both pinned by Rust tests.
- [ ] **C2 — the settings.** `runs.unattended` and
      `runs.unattended_continuations` exist in the settings surfaces
      (CLI and face), with defaults false / 10, and the face renders
      them.
- [ ] **C3 — the chain.** A run ending at the cap on a goal-carrying
      session continues automatically under unattended mode, bounded
      by the continuation count, every continuation an event and a
      checkpoint; a stop pauses the chain by name; a steer rides the
      next continuation. Pinned by tests over the bridge's end-of-run
      path.
- [ ] **C4 — the goal starts.** Setting a goal on a session with no
      run in flight starts the run with the goal as the task; with a
      run in flight, the set reply names that the goal rides the next
      turn. Pinned by a test.
- [ ] **C5 — the goal tool.** The `goal` tool exists (show /
      complete / clear), is declared on every run, writes the
      completion to the store and the log, ends the chain, and the
      face names the completion. Pinned by tests; the tool loop sees
      it like `remember`.
- [ ] **C6 — the pending goal.** A goal set on the welcome attaches
      to the session the first send creates — the tested behavior,
      with the launch-session mapping gone or scoped to its real
      case.
- [ ] **C7 — the face tells the truth.** The unattended receipt
      shows the chain state and the goal; the chip shows done /
      active; a paused chain explains continue. Pinned by harness
      cases.
- [ ] **C8 — the defaults are safe.** With `runs.unattended` unset,
      behavior is byte-identical to today: the cap stops, the receipt
      waits for the press. Pinned by a test.
- [ ] **C9 — the harness.** `tests/face/harness.mjs` gains the
      unattended cases: the chained receipt, the goal chip's done /
      active, the paused state. Harness green with the new cases
      counted.
- [ ] **C10 — the live proof.** On a scratch home with
      unattended on: the goal-to-run start, a cap continuation in the
      chain, the tool's completion ending the chain, a mid-chain stop
      and resume — every observation quoted in the recap.
- [ ] **C11 — the gate.** fmt, clippy `-D warnings`, release build,
      `cargo test` — exit codes read directly, the whole suite.

## Files to be touched

- `src/agent.rs` — the `goal` tool declaration and its handling; the
  turn-cap exit's event payload (the chain needs the goal and the
  remaining count on the wire).
- `src/route.rs` — the clamp amendment (config may raise).
- `src/settings.rs` — the three settings' declarations.
- `src/gui/bridge.rs` — the chain (end-of-run continuation), the
  goal-starts-work path, the pending-goal mapping, the completion
  store write.
- `src/gui/server.rs` — the settings/routes the chain needs.
- `src/gui/assets/app.js`, `app.css`, `index.html` — the receipt's
  chain state, the goal chip's done / active, the settings rows.
- `src/tools.rs` — the `goal` tool's schema.
- `tests/` — the pins; `tests/face/harness.mjs` — the cases.

## Out of scope

- A token-budget replacement for the turn cap (the 2026-11-03
  deferred question) — context health already carries the memory
  side; the turn cap stays the per-run bounding knob.
- Delegation chains' own caps (`delegation.max_depth`) — untouched.
- The huddles rooms' goals — rooms already drive their own turns;
  this plan is the main session's unattended path.
- Cron/`fleet cron` scheduling — the chain is within-session; a
  scheduled re-kick is a later question.

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

The live proof drives the release binary on a scratch home
(`FLEET_HOME` elsewhere, the machine's credentials in its
environment) and quotes every observation: the goal's run starting,
the cap continuing unattended, the tool's completion ending the
chain, the stop-and-resume, and the defaults-off path staying
byte-identical.

## Linked artifacts

- `docs/features/tools.md` — the `goal` tool.
- `docs/features/gui.md` — the receipt's chain state, the goal chip.
- `FUNCTIONAL-SPECIFICATIONS.md` — run behavior with a goal.
- `TECHNICAL-DOCUMENTATION.md` — the settings and the chain's
  interface contract.
- `docs/recaps/` — the proof.

## Risks

- **An unbounded chain is a quota fire, not an agent** — the cap's
  own law (`agent.rs:142-146`). The continuation count is the bound,
  the goal tool is the finish line, and the owner's stop always wins;
  the defaults are safe (C8).
- **Auto-continue + approvals**: an unattended chain hits the
  approval gate like any run — the seat waits, the chain waits with
  it. The seat's timeout policy is untouched; the receipt says what
  the chain is waiting for.
- **The launch-session mapping's other users**: `set_goal`'s
  empty-session mapping serves the huddles' goal path too — the fix
  scopes to the pending-new case and pins the huddles path
  unchanged.
- **Config raising the cap removes a guardrail** — deliberately: the
  owner authored the value. The compiled default remains the
  un-named fallback, and the face's per-run override never raises.
