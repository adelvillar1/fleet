---
name: a-fixture-that-never-rules-is-invisible
description: "feedback 2026-10-30 — two workspace-build debugging traps in one session: a test that forgot to pin FLEET_HOME read the REAL machine's config (six real registrations) and the fixture ruled nothing; and a stub that double-JSON.stringify'd its answer parsed as a JSON string, not an object — both fail at 0.2 s with an empty result, naming nothing"
type: feedback
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 29
superseded-by:
valid-until:
---

Two traps from the workspace-rail build's own `the_rail_document_…` pin,
both producing the same shape — the test runs green-fast with an empty
result and no message names the cause:

1. **The env pin that wasn't there.** The test never called
   `EnvGuard::set("FLEET_HOME", &fixture_home)`, so
   `read_registrations()` read the REAL `~/.fleet/config.json` — six
   real registrations — and the fixture's four projects were never
   consulted. The debug print that settled it was `regs.len()` inside
   the test. The machine's own state is one forgotten guard away from
   every fixture-backed test; [[tests-get-a-seam-never-a-process-global]]
   is the standing rule and this is its sharpest form: a MISSING pin
   doesn't error, it silently substitutes the world.

2. **The double-encoded stub.** The stub script printed
   `JSON.stringify(JSON.stringify(payload))` (the helper built a string,
   then stringified it again), so the parse answered a JSON *string*,
   not an object — the view came back empty and the refusal list stayed
   empty too. A stub's console.log must print the object ONCE; a
   stringified string is not the shape the parser speaks.

**How to apply:** when a fixture-backed test answers empty, print what
the fixture ACTUALLY read (registrations, config path) before touching
the code under test; and in a stub that builds JSON, stringify exactly
once at the print boundary.

Related: [[workspace-history-rail-plan-2026-10-30]],
[[tests-get-a-seam-never-a-process-global]],
[[shell-stub-traps-echo-and-controlled-path]].
