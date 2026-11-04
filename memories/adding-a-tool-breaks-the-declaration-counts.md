---
name: adding-a-tool-breaks-the-declaration-counts
description: "every tool OR lane added to fleet's roster must update the expected declaration lists and count assertions in several places (agent.rs, tests/integration.rs, lanes.rs, install.rs, tests/blend1.rs) — it is mechanical, it reddens everyone's gate, and it happened three times in one session"
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

Fleet's tool set is asserted by name and by count in more than one place,
and adding a tool breaks all of them. Three times on 2026-10-17:

- The dogfooding session's `deck` tool reddened **nine** integration tests
  (fixed here as `b96e5e7`).
- The `vision` tool reddened **eight more** — two count assertions in
  `src/agent.rs` (`plain.declarations().len()` and the expected list
  literal), and six more in `tests/integration.rs` (list literals plus
  `declared_tools(...).len()` counts at 9/10/11/13/14 depending on the
  test).
- The **browser lane** reddened a *different* set, because a lane is a second
  roster with its own counts: `lanes.rs`'s registry test (`the_registry_names_
  the_five_lanes…` → six, and blend1 moves from index 4 to 5),
  `install.rs`'s table test, `tests/blend1.rs`'s two-table test (the install
  table and the launch registry), and `tests/integration.rs`'s
  `fleet lanes` command tests (the `┌` card count and the
  "5 lanes · 5 ok · 0 not ok" summary line).

**Why it costs more than it should:** the failures land in a *different
test binary* from the one you are working in. `cargo test --lib agent::`
passes, the lib gate is green, and the red only appears in the full
`cargo test` — which is the one that matters and the one that takes
longest. And in a co-edited repo a red gate is everyone's problem: it
blocks the parallel session too, so a mechanical test-only fix is worth
landing even when the cause is someone else's change (as `b96e5e7` did).

**How to apply:** when adding a tool, grep for the existing tool names in
the test files before running the full gate —
`grep -n '"deck"' tests/integration.rs src/agent.rs` finds every list and
count that needs the new entry. The position matters: the roster's own
order (after `convention`, before `web`). And expect the *count* assertions
to be a different number from the list literals — some count lane tools
too, some do not, so read each one rather than pattern-replacing a single
number. Related:
[[fleet-deck-two-sessions-collide]],
[[shared-file-commits-can-sweep-another-sessions-work]],
[[fleet-gate-exit-code-trap]].
