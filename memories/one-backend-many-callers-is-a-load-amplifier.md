---
name: one-backend-many-callers-is-a-load-amplifier
description: "feedback 2026-10-29 — a saturated shared backend answers \"absent\", not \"busy\": the deck tool reported \"no embedding backend\" and the rank route returned an EMPTY reply (blank panics) purely because 8901 was queueing; serialize live-embed tests behind one gate"
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

2026-10-29 (fleet, KI-340's class widened): three tests reddened in one
gate run while `curl 127.0.0.1:8901/health` answered ok. One backend, many
parallel callers, one 10-second embed timeout — and the two failure
SHAPES both lied about the cause:

- the deck suggest tool got `None` from the embedder and printed "this
  machine has no embedding backend, so there is no vector match" — a
  **busy** backend read as **absent**, and the tool's honest degradation
  path made the wrong diagnosis look designed;
- the rank route's handler outran its client's 30 s budget and the reply
  came back EMPTY, so both panics printed **blank messages**
  (`assert!(x, "{reply}")` with `reply == ""`). A blank assertion message
  is a tell: the observable never arrived, it was not "slow".

**Why:** the code was right and the machine was wrong, but nothing in
the failure text said so — a saturation class that manufactures
plausible-looking product errors ("no backend", "empty response") is worse
than a timeout, because the first reading sends you to the wrong file.

**How to apply:** in tests, treat every shared subprocess/backend with a
per-call timeout as a load amplifier — put ONE gate (a `cfg(test)` tokio
mutex in the module that owns the backend; async tests await it, plain
`#[test]`s take `blocking_lock`, because clippy's `await_holding_lock`
refuses a std guard across awaits) in front of every caller that hits it
live. Cargo runs test binaries sequentially, so one static spans the
class. Keep per-request budgets only as a hang deadline (30 s → 120 s), and
never widen a budget to "fix" a queue — that is
[[a-test-that-passes-on-an-idle-machine-has-not-been-tested]] all over
again. In product code, a health probe that answers "no backend" on a
TIMEOUT should name the timeout, not the absence.
