---
name: self-containment-is-a-standing-contract
description: "The owner's 2026-11-02 ruling — anything added that becomes a dependency for Fleet must meet the self-containment contract (hard rule 12)"
type: feedback
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

On 2026-11-02 the owner ruled, while framing the native shell plan:
**"anything that's added that becomes a dependency for fleet needs to meet
the self-containment contract."** It is now hard rule 12 in `CLAUDE.md`.

**Why:** self-containment is Fleet's founding intent — "no embedded Node
runtime, no staged harness payload, no checkout dependency"
(`2026-10-12-fleet-native-app.md`), "one static binary… No Node runtime, no
harness checkout, no dsh launch path" (`CLAUDE.md:7`). The PWA era leaned
on things outside the app in a few places (the lane wiring printed for the
owner's shell profile; the services root found by a source-checkout walk),
and a native install would have inherited those as dead lanes. The ruling
makes the intent a gate on *new* work, not just a cleanup target.

**How to apply:** before adding a dependency — a crate, a service, a tool,
an env var, a file outside the app — ask where it comes from at runtime. It
must ship in the bundle, be built by the install, or be fetched by the app.
Never the owner's shell profile, a checkout, or a foreign runtime. Its
absence must be a *named degradation* (a dead lane, a skipped step, a
refused install), not a silent failure and not a manual step the owner must
remember. A change that would lean on something outside the app is a design
defect, not a documentation task. The native shell plan's component
inventory (16 rows) is this rule applied to Fleet as it stands; see
[[rust-1991-toolchain-move]] for the toolchain-side instance.
