---
name: a-new-backend-path-needs-a-test-that-drives-it-not-a-stubbed-rou
description: "feedback 2026-11-09 — a new server-side path (desk_chat: session creation, a seeded log, a dispatched run) is unproven while the face harness only stubs its route; the harness proves the page's POST, never the handler, so the handler needs a bridge test that stands the command door open"
type: feedback
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

Feedback 2026-11-09 — the desk's iteration shipped in 0.6.14 with the recap
honestly recording "no desk deliverables exist on this machine, so the desk's
iteration is proven by the harness and the route's refusals rather than a live
run". That was not a proof: **the face harness stubs `/api/desk/<slug>/chat`**,
so it verified the page's POST and nothing about the handler. `desk_chat` —
session creation, the brief seeded into the log, the deliverable's `session`
field, the run dispatched with the project as its folder — had never executed.

**Why:** a stubbed route is a fixture shaped by the consumer; the producer can
be wrong and every case stays green (the same shape as
[[a-fixture-can-match-the-face-and-miss-the-bridge]]). For a new load-bearing
path, "the harness covers it" is a claim about the page, not the feature.

**How to apply:**
- For any new backend path, write the bridge test: `Bridge::detached`, then
  `bridge.connect()` to stand the command door open **with no agent behind it**
  — the run is accepted and handed over, never executed.
- Assert the real artefacts: the session's `start` kind, the log's first `User`
  record, the store field written back, and the exact `Command::Run(session,
  task, folder, …)` the door received.
- To exercise a SECOND press, record the finish yourself through the same seam
  every event crosses (`record(&bridge.state, &bridge.events, &session,
  json!({"type":"finished",…}))`) — a `UiEvent::Finished` broadcast carries no
  session and will not clear this session's running flag.
- Canonicalize expected paths (`/var` vs `/private/var` on macOS).
- Prefer this over creating live data just to press once — it is repeatable and
  it names the failure precisely.

Related: [[live-behaviour-belongs-to-the-product-not-the-harness]],
[[desk-learning-project-surfaces-drafted-2026-11-09]].
