---
name: live-and-reconstructed-declarations-must-agree
description: "feedback 2026-10-25 — fleet has TWO tool-declaration paths (live declarations() and reconstruction base_declarations()) and the what-the-model-saw byte-equality test enforces their agreement; a tool added to one path only reds ten integration tests at the next full gate"
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

2026-10-25, adding the extract tool: it joined `declarations()` (the
live path, with the admitted()/disabled filtering) but not
`base_declarations()` (the unguarded list `reconstruct_document` reads
to rebuild the request from the session log). The
what-the-model-saw test — which asserts the reconstructed tools array
is byte-equal to the captured request's — failed TEN integration tests
at the next full gate, because the log could no longer reconstruct
what the model actually saw.

**Why:** the model-visible ⟺ logged rule is enforced by byte-equality
between the live request and the reconstruction, and the two
declaration lists are both inputs to that proof. `base_declarations`
mirrors the live path deliberately ("in the live path's position"
comments mark the slots), so a tool added to one side is a divergence
the tests exist to catch.

**How to apply:** when adding or moving a tool declaration, grep BOTH
`base_declarations` and `declarations` in src/agent.rs and add it to
each in its correct slot (unguarded in base_declarations when the
tool's own result names its absence; guarded by admitted()/disabled in
the live path when the run's scope should hide it). Then widen the
mechanical fixtures: the declaration-count assertions in agent.rs
(the N-tool count) and tests/integration.rs (per-body counts, the
lane-tail slice `[11..N]`, and the expected-name vectors after
`"deck"`). Related: [[tool-declaration-assertions-read-names-not-just-counts]],
[[adding-a-tool-breaks-the-declaration-counts]],
[[audit-claims-pinned-against-captured-bytes]].
