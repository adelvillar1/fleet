---
name: a-wait-loop-that-never-reads-the-pipe-dies-at-the-buffer
description: "feedback 2026-10-27 — spawn_with polled try_wait() while the child wrote into a piped stdout nobody read, so ANY CLI answer past the 64 KiB OS pipe buffer blocked the child and burned the whole 120 s budget on a call that answers in milliseconds; drain the pipes while the child runs, and let a cap pin (the 256 KB text fixture) be the thing that catches it"
type: feedback
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The 256 KB text-cap pin in the LayerFS files-instrument test reddened
on its first run with a 120.46 s duration — exactly `CONTROL_BUDGET`.
The cause was in `spawn_with` (`layerfs_tools.rs`): it spawned the CLI
with piped stdout/stderr, then looped on `child.try_wait()` — and
**nobody read the pipes until exit**. A child that writes more than the
OS pipe buffer (64 KiB on macOS) blocks mid-write, never exits, and the
wait loop spins until the deadline kills it. So every CLI answer past
64 KiB — a big `fs read` window, a big `fs list`, a wide diff — hung
for the full budget and returned `Timeout` on a call that answers in
milliseconds. No test had ever produced a >64 KB answer, which is why
it survived to a cap pin.

**Why:** the deadlock has no error of its own — the symptom is a slow
call and a budget-shaped timeout, both of which read as "the wire is
slow" or "the store is big". Any spawn loop that `try_wait()`s must
drain concurrently, because the pipe buffer is a hard bound on how
much a child can produce before it needs a reader.

**How to apply:** after `spawn()`, `take()` stdout and stderr and
`read_to_end` each on its own thread *before* the wait loop; join the
handles after exit (and after `kill()` on the deadline path — a killed
child's pipes close and the readers return). Keep the budget loop for
the timeout, not for the child's progress. And keep cap/bound pins in
the suite even when they look bureaucratic: this one was written to
assert a 256 KB truncation note and instead caught a
process-wedge defect that a live `fs read` would have hit. The run went
120 s → 1.3 s the same day.

Related: [[layerfs-history-instrument-2026-10-26]] (the session and the
plan whose pin caught it),
[[instrumentation-must-measure-what-the-bound-bounds]] (a bound on the
wrong call hides exactly this),
[[a-test-that-passes-on-an-idle-machine-has-not-been-tested]] (a class
of defect no ordinary fixture reaches).
