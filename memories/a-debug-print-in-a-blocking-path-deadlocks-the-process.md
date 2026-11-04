---
name: a-debug-print-in-a-blocking-path-deadlocks-the-process
description: "feedback 2026-10-26 — a debug `eprintln!` added inside a blocking tool path (LayerFS fs_write on the arm's spawn_blocking thread) deadlocked the whole process on the stderr mutex; `sample <pid>` named the Stderr::lock frame, and a hung run's writes are also the signature of a killed CLI holding the Store's single-writer lane"
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

feedback 2026-10-26, from the LayerFS closing-the-seams live proof: a
diagnostic `eprintln!` added inside `Gate::fs_write` — a function that
runs on the arm's `spawn_blocking` thread — hung the whole `fleet run`
at the child's first write. `sample <pid> 3` showed the parked frame
directly: `run_write_file_layerfs → Gate::fs_write → std::io::_eprint →
Stderr::lock → _pthread_mutex_firstfit_lock_wait`. A debug print to
stderr inside a blocking tool path takes stderr's process-wide mutex,
and under the runtime's thread layout that lock can be contended by
another thread's write, so the "harmless print" freezes the feature
under test.

**Why:** blocking paths are not println-safe; the stderr mutex is
process-global and a tool path that runs on a dedicated thread turns a
print into a lock-order inversion.

**How to apply:** never leave (or add) print debugging inside a
spawn_blocking/cadence path — if a raw answer must be seen, thread it
through the function's return (the driver's `Error::Decode` body
already carries it) instead of stderr. When a run hangs, `sample
<pid>` names the parked frame in one command — use it before
theorizing. And distinguish the sibling signature: a run stuck at a
WRITE with reads still answering means a killed earlier run left a CLI
holding the Store's single-writer lane — `pkill` the stale CLI and the
store answers again. Related: the same live proof found
`fs_write` parsing ANY JSON answer as success (the wire's refusal
shape has a `code` field, so a refused write reported "wrote 8 bytes"
with no Commit on the branch) — a spawn driver must check the wire's
refusal shape inside a success-shaped answer, not just parse it.
