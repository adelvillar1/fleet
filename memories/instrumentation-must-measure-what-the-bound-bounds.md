---
name: instrumentation-must-measure-what-the-bound-bounds
description: "feedback 2026-10-26 — the embed_timing writer timed the corpus load while PROBE_TIMEOUT/EMBED_TIMEOUT bound the embed calls; instrument the call the bound wraps, not the path that contains it"
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

KI-249's discipline is "measure before extending", and the
instrumentation that landed measured the corpus LOAD — one row per load
with facts and ms. But the constants the row exists to set,
`PROBE_TIMEOUT` (5s) and `EMBED_TIMEOUT` (10s), bound the embedder's
HTTP CALLS. The load makes no embed call when the index is cached, so
the rows that arrived (17, p50 41ms) said nothing about the bounds. The
fix landed the missing half: `Memory::load_with` collects the load-time
index build's per-embed wall clocks (`build_embeds_ms`), and both
stream-holding callers record one `embed` row per call.

**Why:** "instrument the path" feels equivalent to "instrument the call"
and is not — a cached path skips the call entirely, so the measurement
is silent exactly when the system is healthy and the bound stays a
guess. Same shape as [[a-flag-that-conflates-a-tree-fact-with-a-machine-fact]]
one layer down: the measurement names a condition that never occurs.

**How to apply:** when adding instrumentation to size a bound, name the
exact call the bound wraps and record THAT call's latency — one row per
call, with a count that makes a tail meaningful. If the natural
recording site is a path rather than the call, the path is the wrong
site. Related: [[fleet-run-control-and-timeout-gaps]],
[[verify-the-arithmetic-behind-a-measured-narrative]].
