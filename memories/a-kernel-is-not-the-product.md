---
name: a-kernel-is-not-the-product
description: "feedback 2026-10-31 — I built the design canvas's kernel and reported it as the integration; the owner's correction ("missing a lot … just distilled the very basics … I want the whole shebang") means a port is judged against the source's full product surface, not against the minimum that works"
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

**2026-10-31 (the design canvas).** The port landed green, was used, and the
owner's verdict was: *"it's missing a lot from the original design-canvas. it
looks as if you just distilled the very basics of it. I want the whole shebang
of design-canvas."* The accounting backed it up — the source has ~25 product
features, the copy had ~12 — and the missing ones were not polish but the
*workspace*: the conversation, multi-direction generation, assets, brand kits,
reference DNA, page sets, the 8-score review loop.

**Why this happens.** A survey that says "the portable core is the canvas and
its contract" is true about *code* and wrong about *product*. The code
portable at ~925 lines was only the renderer; the product's value sat in the
layers the survey filed under "the SaaS platform" — the chat that drives
refinement, the composer's many ways to seed a generation, the assets and
brands and references that make output good. When the owner says "disregard
the commercial issues", they mean the *billing and hosting*, not the
*workflow*.

**How to apply.** Before building a port, enumerate the source's user-facing
features as a checklist and put each one in exactly one bucket — **ported**,
**replaced by a native leg**, **deliberately dropped (say why)** — and
present that list for confirmation, not a summary sentence. A green gate and a
live render prove the kernel works; they do not prove the product arrived.
Then sequence so the hinge (here, the board's conversation) is first: every
later feature leans on how runs are seeded.

**Follow-on steer, same day:** *"make sure that we port to rust whatever
merits porting"* — the lesson's second half is that the Rust/JS boundary must
be argued per candidate (byte-identical across consumers, touches the home, or
is a security boundary → Rust), not decided once by slogan. See
[[design-canvas-complete-plan-2026-10-31]].
