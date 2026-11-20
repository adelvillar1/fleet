---
status: draft
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

- [ ] A `fleet run` session's checkpoint store is trimmed to the last 20
      turns by the 15-minute reclaim, proven by a test that names the
      reclaim spawn (or by an integration run whose store's file count
      stops growing).
- [ ] `chat` and the face keep their current behavior — the timer is
      started exactly once per session, by the same function.
- [ ] The sweep's result is recorded: `cron` and `eval` either start the
      reclaim or the ledger carries a row naming why they should not.
- [ ] The full gate exits 0.
- [ ] KI-575 is closed in `docs/known-issues.md` with the commit named.

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
