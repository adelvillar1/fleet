---
name: fleet-cross-session-dependency-problem
description: "2026-10-19 — the owner raised cross-graph dependencies (two or more fleet sessions working in tandem); nothing detects a collision today because there is no cross-process write coordination; RESOLVED by the one-graph-per-plan ruling, which makes two sessions two executors of one graph"
type: project
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

**Resolved the same day.** The owner's next message ruled the graph is
scoped **one per plan document**, not per session — see
[[fleet-graph-is-one-per-plan]]. Two sessions working one plan are
therefore two *executors of one graph*, not two graphs needing linking,
and the registry / external-edge / discovery machinery designed below is
no longer needed. What survives from this file is the diagnosis (the
silent race is real and unfixed) and the one recommendation that stands
regardless: fix the write-path lock first. Cross-*plan* dependencies,
should they ever be needed, use a plan slug as identity — a far smaller
and more deliberate surface than cross-session ever was.

The original analysis follows.

The owner raised this on 2026-10-19, immediately after ruling on the
delegation design direction in [[fleet-delegation-architecture-deep-dive]]:
a dependency graph could have dependencies on nodes from *other* graphs, so
two graphs could be linked through external dependencies — "I have two or
more fleet sessions working in tandem and they could be independent, but
what happens if they are not?"

**The answer today is that nothing detects it, and the failure is silent.**
There is no cross-process or cross-session write coordination at all. The
one-writer discipline the docs describe is per-turn and per-process —
`plan_batch` keeps two writes to one path out of the same *wave*, which is
inside a single agent loop. `write_file` takes no lock. And there is no
registry of live sessions, so nothing even knows another session exists. If
two sessions both edit `src/foo.rs`, they race, last writer wins, and
neither is told. A cross-session dependency expressed the way dependencies
are expressed today — English prose in a `goal` string — is invisible to the
harness.

**The seam already exists.** `Unlock.unresolved` is documented as
"Prerequisites that are not nodes in the graph" (`src/graph.rs`). Today an
unresolved prereq is a declaration error. An external dependency is exactly
a prereq that resolves *outside* the graph, so the field is the hook — it
just currently refuses instead of looking up.

**The per-session persistence recommendation makes this harder, not
easier**, and that was owned explicitly: one graph file per root run gives
traceability and no clobbering, but it makes a cross-session dependency
inexpressible. The owner found a real flaw in it.

**Three options, with their costs:**

- **(a) Per-session graphs plus a cross-graph edge registry.** Graphs stay
  per-root-session for traceability; a prereq that is not local resolves
  against a small shared registry mapping `(root_session, node_id)`, and the
  score computation reads the other graph read-only. Keeps everything the
  owner asked for and adds edges.
- **(b) One graph per work-effort, N executors.** If two sessions are
  genuinely working one decomposition they are not two graphs — they are one
  graph with two executors; sessions become executors and the graph is the
  work. Cleanest model, but `Graph` is a plain struct with `&mut self`
  methods, so it must become concurrency-safe or sit behind a lock, and the
  non-atomic `save` turns from an inconvenience into a correctness bug.
- **(c) Per-session graphs plus shared milestones.** Sessions depend on a
  named gate both agree on rather than on each other's nodes — a small shared
  file with a status. Far less machinery (no cross-graph score, no cross-graph
  cycle detection, no discovery), but coarse: you can say "wait until the
  parser is done", not "wait until node `build-parser` in that session
  verifies". Held as the fallback if the registry proves too heavy.

**Recommendation: fix the resource edge first, regardless of which option
is chosen.** Two sessions racing on one file is silently broken *right now*
and is the failure the question most directly hits. The mechanism already
exists in this codebase — `src/memory.rs` does per-fact locking through
`src/filelock.rs` — so applying a per-path flock to the write path is small
and contained. It is also strictly necessary for (a), because a cross-graph
score computed from a torn read of another session's graph is worse than no
score.

Then (a), because it is the only option giving a real dependency on another
session's specific node — with three properties built in from the start
rather than deferred: **cross-graph cycle detection** (the existing detector
walks one graph via `reaches`; if A depends on B and B on A both block
forever and nothing notices — two sessions deadlocking on each other is the
nastiest failure in this design); **a dead dependency must resolve** (if B
dies mid-run, A blocks forever on a node that will never verify, so there
must be an explicit "the graph I depend on is gone" path); and **session
identity and discovery** (no registry exists today, and cross-graph edges
need addressable identities — that registry is also what makes (c) possible
later).

**This reorders the build list** in [[fleet-delegation-architecture-deep-dive]]:
the write-path lock moves ahead of all the graph work, and the session
registry moves ahead of the `plan` tool, since nothing can reference a
session that has no address.

**Why:** the owner runs two or more fleet sessions in tandem, so
cross-session interaction is a real operating condition rather than a
hypothetical — and the current behaviour is a silent race with no detection,
which is the worst failure class. Anyone planning dependency-aware
delegation has to know that the urgent cross-session problem is the missing
path lock, not the missing dependency edge.

**How to apply:** if cross-session delegation comes up, do not start with
cross-graph dependency edges. Start with a per-path flock on the write path
using the existing `filelock.rs`, then the session registry, then decide
between (a) and (c). Treat cross-graph cycle detection, dead-dependency
resolution, and session discovery as mandatory parts of (a), not follow-ups.
And name the new coupling surface out loud: one session can now be blocked
by another session's behaviour, and "my run is stuck and the reason is in
someone else's session" has to be legible in the surface, not just in a log.
