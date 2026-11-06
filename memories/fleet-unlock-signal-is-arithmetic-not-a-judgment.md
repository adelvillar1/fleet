---
name: fleet-unlock-signal-is-arithmetic-not-a-judgment
description: "2026-10-19 — the owner asked how the \"unlocked\" signal is generated and whether sys1 could produce it; it is not generated at all today (nothing calls the scheduling API), and when wired it is a function return rather than an event, because the score is derived on every read. sys1 would be the wrong tool — it is a classifier, the unlock signal is three booleans, and using a reporter to generate a fact inverts the repo's own \"facts gate, reports corroborate\" law. sys1's legitimate role is the functional half…"
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

Found on 2026-10-19 answering the owner's "how is the unlocked signal being
generated? we could use sys1 to get us that, correct?"

**It is not generated at all today.** Nothing in production calls
`is_unlocked`, `ready`, or `dispatchable` — that is the deep dive's headline
finding (see [[fleet-delegation-architecture-deep-dive]]). There is no unlock
signal because there is no dispatcher to receive one.

**When it is wired, it is not a signal — it is a function return.**
`is_unlocked` is `score_zero && resource_free && deps_verified`: three
booleans over declared graph state. `dispatchable(bound, running)` returns the
current answer at the moment it is asked. Because the score is derived on
every read rather than stored, a completion that landed since the last pass
is already reflected — no notification, no invalidation, no event. The
orchestrator consults it at its natural scheduling points, which is after
each tool-result batch.

**sys1 would be the wrong tool, and the third reason is the load-bearing
one.** sys1 exposes exactly one tool, `sys1_classify`, a classifier
(`services/sys1/service/mcp_server.py:3`).

1. *Slow.* KI-044 measured a sys1 call at 1,160 and 1,302 ms. Spending that
   per node per completion to learn a boolean is not a tradeoff.
2. *Non-deterministic.* The same graph state could answer differently on
   different calls, and graph state changes on every completion, so there is
   no stable key to memoize on — the route cache exists precisely because
   sys1's verdicts are stable enough to key by task hash.
3. *It inverts the repo's own law.* The delegation-steering plan states it
   plainly: "Facts gate; reports corroborate." The unlock signal is a *fact*;
   sys1 is a *reporter*. Using a reporter to generate a fact is exactly the
   inversion that law forbids, and it is the same category of mistake as
   asking a model whether a file exists.

**Where sys1 does belong in this design:** the functional half of the
fact-gate. Unit tests and nomenclature are both machine-checkable (`cargo
test`, and `nomenclature::check` at `src/nomenclature.rs:1206`); "does this
node's delivered work actually integrate" is a judgment, and that is the one
part of the gate that legitimately wants a reporter. It is also what
`Delta::verify` gestures at with its three log-shape heuristics, and what the
delegate beat already does when grading a delegation's outcome.

**The real change this implies.** `plan_batch` computes a wave plan once and
then executes it as fixed. A completion that unblocks new work cannot be
honoured by a plan that has already been emitted — so the constraint is not
notification, it is that the wave plan must be **revisable mid-flight**: after
each completion, re-consult `dispatchable` and admit newly-unlocked nodes
into the remaining slots, still bounded by `DELEGATION_CONCURRENCY` and still
serialized on the resource edge. That is a contract change to one function in
`agent.rs`, and it must exist before any of the graph's scheduling logic is
reachable from a real run.

**Why:** the owner's instinct to reuse an existing component was reasonable
and the answer is no, for a reason that generalizes — check whether the job
is a fact or a judgment before agreeing that a reporter can do it. Anyone
wiring the dispatcher needs to know the unlock signal needs no generator at
all, and that the actual work is the revisable wave plan.

**How to apply:** when a "signal" is described, first ask whether the value
is already derivable on read — if it is, the notification is a fiction and the
real work is wherever the value is *consumed* on a stale assumption (here,
`plan_batch` treating its own output as immutable). Related:
[[fleet-graph-score-double-counts-shared-deps]] for the derived-vs-push
question this rests on, [[fleet-graph-is-one-per-plan]] for the scoping
ruling, and [[fleet-delegation-architecture-deep-dive]] for the four missing
seams this is part of.
