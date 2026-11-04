---
name: fleet-router-rules-everything-else-plan
description: "2026-10-21 plan docs/plans/2026-10-21-router-rules-everything-else.md — APPROVED, BUILT and PUSHED in 4bd2f18 (plan status completed, gate 1488); the four router rulings are now the code"
type: project
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

The owner APPROVED the plan on 2026-10-21 and the whole slice landed the same
day: `9c72bbb` (C0 rename + windows, committed pre-plan and ratified),
`bf604e7` (the plan, activated), `4bd2f18` (the build, pushed), plan file
`status: completed`, recap at `docs/recaps/2026-10-21-router-rules-everything-else.md`.
The four rulings are now the code — see
[[fleet-router-rules-everything-but-the-leader]] for the standing rules and
[[orchestrator-routes-through-hard]] for the narrowed 2026-10-16 ruling.

Build notes worth keeping:
- C1's tail is `route::with_tail`, applied in select/fallback/workload_decision;
  an existing test caught a real edge — an empty preference table must decide
  its head walk BEFORE the tail is appended, or the reserve becomes the head
  of a walk nobody declared.
- C4's boundary, chosen when a roles test failed: a child routes itself
  (`route::prepare_child`, `Walk::Judged`) only when the run has a home AND
  the parent's client is routed; an unrouted parent (every test double) hands
  the child the same client. The parent's pin rides the `Router`
  (`Router::pin`), read through `client.router()`.
- C5 was amended mid-build per the plans README: a `fleet chat` log opens
  before its first message exists, so it starts as the pinned pair's model or
  `auto`; the end records name what served.
- C6 is `route::resolve_model_pin` — a bare id on two providers is refused
  listing every `provider/model` spelling.
- The gate's first run died on ENOSPC — target/debug held 159 GiB of stale
  test binaries; `cargo clean --profile dev` freed it, and the fix became
  [[fleet-recap-ends-with-cache-sweep]].
- Named gaps: KI-278 (a scheduled run's child keeps the parent's client —
  cron's `improve_home` is deliberately None and doubles as the route home)
  and C8's deferrals (sys1's `composed` label; effort-per-workload,
  [[fleet-workload-effort-direction]]).

**Why:** the owner corrected the earlier attempt to build without presenting
the plan ([[plan-is-a-file-not-prose]]); this plan is the instance that did it
right — presented, approved, built, completed.
