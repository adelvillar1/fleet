---
name: a-relay-loops-sender-must-outlive-its-spawn
description: "feedback 2026-10-18 — a tokio watch sender bound inside the block that spawns the loop drops when the block ends, so `changed()` answers immediately forever and the loop leaks a task per iteration (blocking pool saturated at 512 threads, ~1025 sockets). `let _ = x` drops immediately too; the sender needs a binding that outlives the spawn"
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

The relay client's poll loop waits on `shutdown.changed()` alongside its
poll. The sender was created and the loop spawned inside one
`if let Some(relay) = &options.relay { … }` block, with the sender bound
at the block's end — so it dropped the instant the block closed, long
before the loop's first await finished. A `watch::Receiver::changed()`
on a dropped sender returns `Err` at once, every time; the loop's error
arm did nothing and fell through, so each iteration spawned another
`spawn_blocking` poll whose `JoinHandle` was dropped (a dropped handle
does not cancel a task). The pool grew to its 512-thread ceiling and the
relay saw ~1025 sockets, while the process looked perfectly healthy.

The fix was to hoist the sender into a binding that outlives the block
(`let mut _relay_shutdown: Option<Sender<bool>> = None;` assigned inside),
and the comment in the code says why, because the next reader will
absolutely try to "simplify" it back.

**Why:** the failure is silent and the symptom is far from the cause — a
leaked-task leak reads as a network problem or a relay bug, and the
`let _ = shutdown;` idiom that *looks* like "keep this alive" is exactly
the line that drops it. Nothing in the types or the compiler points at
the scope.

**How to apply:** whenever a spawned task waits on a channel, ask what
the sender's scope is — if it is the block that spawned the task, it is
already gone. Prefer a named binding (`_name`, never `_`) held by
whatever owns the task's lifetime. And when a thread or socket count
grows without bound while CPU stays idle, suspect leaked tasks whose
handles were dropped, not a hot loop. Related:
[[a-regression-test-must-fail-when-the-bug-returns]] (the guard that now
pins this), [[tests-get-a-seam-never-a-process-global]].
