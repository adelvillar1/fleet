---
name: fleet-memories-tests-need-their-own-home
description: "feedback 2026-10-18 — an integration test that spawns the real fleet binary against a scratch corpus must pin FLEET_HOME in the child, because resolve_root's home-first rule prefers the ambient ~/.fleet/memories over the test's fixture; and a flake that passes in isolation can still have a nameable mechanism — replay the preserved fixture before accepting machine load"
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

KI-227, found by the batch-3 sweep's gate on 2026-10-18: three `fleet memories`
integration tests read 0 facts and failed under the full suite's parallel load
while passing in isolation and in most runs — the same profile as the repo's
documented flake family (KI-027, KI-119, KI-204), which had been attributed to
machine load.

The mechanism was nameable, not environmental. `memory::resolve_root` checks
the fleet home's own corpus BEFORE the working-directory walk (KI-087's
home-first rule), and the moment `~/.fleet/memories` holds any `.md` — a
consolidator's summary, a `remember` write, another session's fleet process —
it wins. Those tests spawn the real binary with `current_dir` on a scratch
corpus and no `FLEET_HOME`, so they read whatever the ambient home held at
that instant instead of their fixture.

**Why:** the fixture was innocent and the evidence said so — the preserved
failing corpus replayed 6 facts against the same binary, and an instrumented
`resolve_root_from` showed the child's cwd and home were both correct, so the
root it CHOSE was the ambient one. A flake that passes in isolation and fails
under load is not automatically environmental; if the failure has a mechanism,
name it before accepting load as the cause.

**How to apply:** the fixture hands back a FLEET_HOME and every spawned child
carries `.env("FLEET_HOME", &root)`, so the home-first rule lands on the
fixture's own corpus by construction rather than by the ambient home happening
to be empty. Pin it with a test that writes a competing corpus into a scratch
`~/.fleet/memories` and fails when the pin is removed — that test is what
proves the pin is load-bearing. Same family as
[[tests-get-a-seam-never-a-process-global]] (tests reaching past their own
fixture into shared state), and [[fleet-gate-exit-code-trap]] for the gate
discipline that surfaced it.
