---
status: completed
created: 2026-11-20
updated: 2026-11-20
slug: run-checkpoint-reclaim
---

# `run` starts the checkpoint reclaim timer too

Fixes KI-575. `run` is the mode the harness itself drives — every Fleet
session's own `fleet run` — and it is the one mode whose checkpoint store
grows without bound.

## Context

`spawn_reclaim` is the 15-minute timer that trims a session's checkpoint
store to its last 20 turns. It is started by `chat_command` and by the
face, and **not** by `run_command`. The checkpoints module's own doc calls
the store bounded by that timer, so on the `run` path the claim is false:
the store grows for the life of the run.

The shape is the one the ledger keeps recording (`RECURRENCE` rows,
F3 in the deep dive): the newest entry point is missing a piece its older
siblings have.

## Approach

1. Read `run_command` in `src/main.rs` and the two call sites that start
   the reclaim (`chat_command`, the face's bridge/agent spawn), and name
   the exact seam: same `home.join("checkpoints")` root, same interval,
   same retained-turn count.
2. Start the same reclaim in `run_command` at the point where the session
   is created and the checkpoint root exists — one call, the same
   function the other two use, no new timer logic.
3. Pin it: a test that drives `run_command`'s session setup (or the
   smallest seam it can reach without a real provider) and asserts the
   reclaim handle is spawned for the run's home — the assertion the
   module's doc already promises. If the seam is not testable without a
   provider, factor the spawn behind a small `fn` and test that.
4. Sweep for any other mode that creates a session without the reclaim:
   `cron`'s fire path and the eval harness are the two candidates; the
   plan's acceptance covers naming what the sweep found, not necessarily
   fixing a third path.

## Acceptance Criteria

- [x] A `fleet run` session's checkpoint store is trimmed to the last 20
      turns by the reclaim — proven twice: `checkpoints::tests::
      the_reclaim_timer_trims_a_store_that_grew_past_the_keep` (a 25-turn
      store trims to 20 on the timer's first pass and the newest turn
      survives) and `both_session_entry_points_start_the_checkpoint_reclaim`
      (both `main.rs` session entry points start it, read from source).
- [x] `chat` and the face keep their current behavior: `spawn_reclaim`
      now delegates to `spawn_reclaim_every(store, RECLAIM_INTERVAL)`, the
      call sites are untouched, and the 15-minute default is unchanged.
- [x] The sweep's result is recorded: `cron::execute` and the eval
      harness do not start the reclaim; both are single-shot runs, so the
      shared store is trimmed by the next reclaim-equipped process —
      recorded as KI-577 (status: noted).
- [x] The full gate exits 0 on the tree that carries the fix: fmt 0,
      clippy `-D warnings` 0, `cargo test` **2648 passed, 0 failed** across
      the workspace, `cargo build --release` 4m07s.
- [x] KI-575 is closed in `docs/known-issues.md`, naming `src/main.rs`'s
      call beside `chat_command` and the face, with the guard test as the
      next action. The ledger is at 578 rows / 3 open.

## Files to be touched

- `src/main.rs` — `run_command`'s session setup
- `src/checkpoints.rs` — only if the reclaim needs a named, testable entry
- `docs/known-issues.md` — KI-575's close text (and any sweep row)

## Out of scope

- Changing the 15-minute interval or the 20-turn retention.
- The reclaim's eviction policy.

## Verification

1. `cargo test -p fleet --lib checkpoints` while iterating.
2. The new pin test, run alone and in the suite.
3. The full four-leg gate once, at the end.

## Linked artifacts

- KI-575 — the open row with the same next action.
- `docs/architecture/deep-dive-2026-11-20-fleet-state.md` — F2.
