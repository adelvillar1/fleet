---
name: tests-get-a-seam-never-a-process-global
description: "feedback 2026-10-18 — a test that needs a different cwd/env gets a #[cfg(test)] seam on the struct (the G5 point_at_memories precedent), never std::env::set_current_dir, which is process-global and breaks every parallel test in the binary; also: eyeball regex bulk edits, they hit signatures"
type: feedback
date: 2026-11-05
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Two failures from the GUI revamp's G9 trust-trio tests (2026-10-18), one
generalizable:

**A test that needs a different working directory gets a `#[cfg(test)]`
seam, never `std::env::set_current_dir`.** The trio's bridge reads its tree
root from the process cwd (the production rule — the same root the tools
write to), so the obvious test changed into a temp tree and back. That
mutated a process-global every other test in the binary reads: the
parallel `the_rank_route_orders_candidates_by_what_was_typed` test failed
because the bridge's corpus root resolves off the cwd. The fix is the G5
precedent the repo already had: `Bridge::point_at_memories` exists for
exactly this reason, so the trio got `point_at_cwd(root)` — the production
rule stays out of the test's way and no global moves. **How to apply:**
when a struct reads anything from the process (cwd, env, HOME), the test
seam is a `#[cfg(test)]` setter on the struct, and the comment should say
why the seam exists. Same class as the G4 bridge-fixture flake (shared
temp home) — both are tests reaching past their own fixture into shared
state.

**Eyeball every bulk regex edit's insertion site.** Adding two fields to
every `AgentOptions { … }` literal via a `compact_now:` line regex also
matched `drive()`'s `compact_now: bool` PARAMETER in `src/main.rs`,
producing `checkpoints: None,` inside a function signature (a type
position) and a 9-vs-11 argument error at two call sites. A struct field
and a function parameter can share a line's shape. **How to apply:** after
any scripted multi-file edit, compile before believing it, and read the
diff hunks at the boundaries — the failures are always at the edges the
regex was too loose to see.

**The 2026-10-26 form: a REAL-WORLD state change poisoned the process
cache, and every stub test inherited it.** The layerfs work registered
THIS repository in the real `~/.fleet/config.json` (the motivating case,
live). Three consequences, all fixed the same day: (1) the live
delegation claimed the once-per-process `gate()` cache with the REAL CLI,
so every stub test beside it inherited the real CLI instead of their own
stub — the delegation path now resolves the machine facts, registrations
and CLI FRESH per call (`read_machine`, `read_registrations`,
`Gate::resolve` — the cache is gone from that path); (2) plain delegation
tests' premise ("this repo is not registered") died with the
registration — `run_on`, the shared harness for the plain family, now
pins a scratch FLEET_HOME (the lock guards the SET, the restoration guard
outlives the await, the lock guard does not — a MutexGuard held across an
await is a clippy -D error); (3) tests whose premise is the ABSENCE of a
world fact must pin that absence themselves, because the world changes
under them mid-plan.

**How to apply:** when a struct reads anything from the process (cwd,
env, HOME), the test seam is a `#[cfg(test)]` setter, and production paths
tests exercise with ambient env must resolve FRESH — a process-global
cache is a channel from one test's (or one live run's) world into
another's. Related: [[gui-revamp-browser-face-decision]] (where these
happened), [[fleet-gate-exit-code-trap]] (the gate catches both of these
immediately), [[shared-file-commits-can-sweep-another-sessions-work]]
(the same verify-what-you-staged discipline),
[[a-debug-print-in-a-blocking-path-deadlocks-the-process]] (the same
live-round family).
