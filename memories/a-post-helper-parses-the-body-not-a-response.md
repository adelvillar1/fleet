---
name: a-post-helper-parses-the-body-not-a-response
description: "feedback 2026-10-31 — fleet's face post() resolves {status, body}, not a fetch Response, so reply.json() threw on every call and five follow-ups were silently dead while the harness (asserting POST bodies only) stayed green; read reply.body, and have the reply-detail path observed too"
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

**2026-10-31, building the design canvas's conversation (T0).** The face's
`post(path, body)` helper already parses the exchange and resolves
`{status, body}`. My new design code wrote `post(...).then(reply =>
reply.json().then(...))` — the natural fetch idiom — so **every** call threw
`reply.json is not a function` inside the promise chain and the `.catch`
swallowed it into a notice nobody was reading.

**What was dead while the harness was green.** Five paths, all silently:
the writer's own read-back (`designWrite` never refreshed the canvas), the
**composed task** (so the run was never started), the live tweak injection,
the export notices, and the provider save's discovery notice — the last
one carried over from the providers landing, where the same misuse had
been shipped a session earlier without anyone noticing.

**Why the harness didn't catch it.** The checks asserted the POST
*requests* — the bodies on the wire — which were all correct. Nothing
asserted the *consequence* of the reply: that the composed task reached
`/api/run`, that a notice appeared, that a re-read followed. A dead reply
path is invisible to a test that only reads the outbound half.

**How to apply:**
- In this face, read `reply.status` / `reply.body`. A plain `fetch` (used
  for GETs) still gives a `Response` — the two idioms coexist, so copy the
  idiom from the neighbouring function instead of from muscle memory.
- When you add a request whose reply drives the next step, **pin the next
  step**, not just the request: `check('the run carries the composed
  task')` on the `/api/run` body is what would have gone red.
- The live trace is the cheap detector: one Playwright run logging every
  `/api/design` and `/api/run` response showed the chain stopping after
  `task` with no `/api/run` at all — minutes of guessing avoided.

Companion, same session: the writer answered the task op wrapped as
`{op, type:"design_write", wrote}` while the face read a flat `task` — the
**response shape** of a read-shaped op should be flat, not a write
envelope, so a caller never has to know which op returns which wrapper.

Related: [[a-green-pin-can-hide-a-broken-first-run]] (a green pin and a
broken first run), [[a-fixture-that-never-rules-is-invisible]].
