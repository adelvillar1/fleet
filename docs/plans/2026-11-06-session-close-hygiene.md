---
status: draft
created: 2026-11-06
updated: 2026-11-06
slug: session-close-hygiene
---

# Plan: the session-close hygiene

## Context

The disk filled on 2026-11-06: the checkpoints grew to 72 G because nothing
prunes them. The recap's sweeps (the process registry, the build cache)
and the daily maintenance (the target, the tmp, the corpus commit) do not
cover the checkpoints, and no pass disposes of the memory corpus at the
session's close. The owner's rulings: a hygiene pass runs once a goal is
completed and a recap is written — the memory disposed, the disk cleaned
— and the standing principle is **if you spin it up, you shut it down
cleanly**: the agent that spawns a process owns its shutdown, the Chrome
a validation run drives included.

## Approach

1. The run's close shuts down the processes the run spawned — the
   `process` tool's children, cleanly — so nothing the agent spun up
   outlives the run. The lanes already close on the run's exit, and the
   browser lane already terminates its Chrome.
2. A `scripts/session-hygiene.sh` runs the pass:
   - The checkpoints: prune to the retention — the last 20 turns per
     session, and a 5 G total cap, oldest first.
   - The memory: one consolidation pass (`fleet memories --consolidate`)
     — the old working facts compressed into episodic summaries, the
     expired archived.
   - The disk: the tmp retirement and the build cache sweep.
   - The processes: the stale Chrome, lanes and background processes
     from the closed session — the orphans the run's close could not
     reach.
3. The bridge runs it on the `GoalDone` event — the goal completion.
4. The recap skill runs it at the session's close.

**Decision-model check.** None — the pass is a deterministic script. The
consolidation's own judgments are the memory cycle's, not a new lane call.

High-level steps:

1. `src/processes.rs` / `src/agent.rs`: the run's close shuts down the
   spawned processes.
2. `src/checkpoints.rs`: a `prune` (the retention).
3. `src/cli.rs` / `src/main.rs`: `fleet checkpoints --prune`.
4. `scripts/session-hygiene.sh`.
5. `src/gui/bridge.rs`: the `GoalDone` hook.
6. The recap skill's step.
7. Pins + the gate.

## Acceptance criteria

- [x] C0 — the run's close shuts down the processes the run spawned.
- [x] C1 — the bridge runs the hygiene pass on the `GoalDone` event.
- [x] C2 — the pass prunes the checkpoints to the retention (the last 20
      turns per session, the 5 G cap, oldest first).
- [x] C3 — the pass runs one memory consolidation pass.
- [ ] C4 — the pass retires the tmp and sweeps the build cache.
- [x] C5 — the pass sweeps the stale processes (the Chrome, the lanes, the
      background) from the closed session.
- [x] C6 — the recap step runs the pass.
- [x] C7 — the pins and the gate.

## Files to be touched

**Backend / API:**
- `src/processes.rs`, `src/agent.rs` — the run's close shuts down the
  spawned processes.
- `src/checkpoints.rs` — the prune.
- `src/cli.rs`, `src/main.rs` — `fleet checkpoints --prune`.
- `src/gui/bridge.rs` — the `GoalDone` hook.

**Scripts:**
- `scripts/session-hygiene.sh` — the pass.

**Tests:**
- `src/checkpoints.rs` `mod tests` — the retention.
- `tests/integration.rs` — the hook and the pass.

**Documentation:**
- `~/.agents/skills/write-session-recap/SKILL.md` — the close step.
- `docs/features/install.md` — the hygiene pass.
- `TECHNICAL-DOCUMENTATION.md` — the runbooks.

## Out of scope

- The machine-wide caches (the uv/Hugging Face) — the repo hygiene's
  territory, and the lane weights must survive.
- The daily maintenance's legs (unchanged).
- The checkpoint capture's per-snapshot cap (unchanged).

## Verification

**C0 — the run's close.**
```bash
cargo test processes
```
Expected: a run that spawned a background process shuts it down at the
close, and the report names what it stopped.

**C1 — the hook.**
```bash
cargo test goal_done
```
Expected: a `GoalDone` event runs the pass.

**C2 — the retention.**
```bash
cargo test checkpoints::tests::prune
```
Expected: a fixture store keeps the last 20 turns per session and the
newest 5 G.

**C3/C4/C5 — the pass.**
Expected: the consolidation runs, the tmp retires, the cache sweeps, the
stale processes are named.

**C6 — the recap step.**
Expected: the recap's close runs the pass and quotes its line.

**C7 — the gate.**
```bash
cargo fmt --all --check && cargo clippy --all-targets -- -D warnings \
  && cargo build --release && cargo test
```
Expected: all four exit 0.

## Linked artifacts

- `~/.agents/skills/write-session-recap/SKILL.md` — the close step.
- `docs/features/install.md` — the hygiene pass and the retention.
- `TECHNICAL-DOCUMENTATION.md` section "Operational runbooks" — the
  pass.

## Notes

- The 2026-11-06 incident: the checkpoints 72 G, the per-turn 64 MB
  snapshots, no retention.
- The retention's numbers (20 turns, 5 G) are the same rule the
  2026-11-06 manual sweep used.
- The principle's reach: the run's close shuts down the `process` tool's
  children; the lanes already close on the run's exit; the browser lane
  already terminates its Chrome (`services/browser/src/browser/
  __init__.py:154-158`); the hygiene pass sweeps what escaped.

## Progress

- C0, C2, C7 landed 2026-11-06: `Registry::shutdown` + the run's close, `checkpoints::prune` (the last 20 turns per session, the 5 GiB store cap), `fleet hygiene`, the pins `shutdown_stops_what_the_run_spawned_and_names_it` and `prune_keeps_each_sessions_last_turns_and_the_stores_cap`; the gate 1917/0 lib, clippy 0, fmt 0, build 0.
- C1 (the GoalDone hook), C3 (the consolidation), C4 (the tmp and the build cache), C5 (the machine-wide stale processes), C6 (the recap step) - C1 landed 2026-11-06: the bridge's event loop runs the retention on the `GoalDone` event, off the async context, naming what it freed. The pin is the event loop's own — hard to drive headless — so the box stays open until a live proof quotes the hook's line.
- C6 landed 2026-11-06: the recap skill's new 8d runs `fleet hygiene` at the session's close and quotes its line.
- C3 (the consolidation's model spend at every goal completion) and C5 (the machine-wide stale processes) remain; C4's tmp and build-cache legs ride the recap's existing 8c sweep and the daily maintenance.
- C3 and C5 landed 2026-11-06 with their live evidence: `fleet hygiene` ran the consolidation (the scratch home's honest empty) and the sweep stopped a spawned orphan by name — `hygiene: stopped the orphaned 74116: .../venvs/sys1/bin/python ...` — after the first run's sweep stopped nothing because ps's space-padded columns hand a bare splitn-on-whitespace an empty ppid field, a bug the live orphan caught and the parse now collapses. C1's live proof: the GoalDone hook's own line — `fleet: hygiene: 5 checkpoint turn(s) pruned, 1303 bytes freed` — quoted from the face's stderr after a real run completed a real goal, the store settling at 21 turns.
