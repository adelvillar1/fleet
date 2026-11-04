---
name: a-fallback-path-hides-the-defect
description: "feedback 2026-10-19 — a designed graceful-degradation path is indistinguishable from a defect that triggers it; the type-ahead's zeros looked exactly like the documented no-backend fallback, so every surface check confirmed "working as designed" while a dead keep-alive connection was disabling the feature"
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

The composer's semantic type-ahead has a documented honest degradation: with no embedding
backend, `/api/rank` answers all zeros and the lexical ranking stands. That path is also
exactly what a dead keep-alive connection produces — one stale-socket failure, `None`, zeros —
so when the feature silently stopped working in real use, every check that read the UI (and the
code's own comments) confirmed "the fallback is working as designed". The defect was only
visible by capturing the actual network exchange: one rank request answered 200-with-zeros in
16 ms, too fast to have embedded anything.

**Why:** a fallback's output is deliberately identical to degraded-by-design output, so "the
system shows the fallback" is not evidence the fallback's *trigger* is legitimate. The
graceful path launders the defect.

**How to apply:** when auditing a feature with a designed fallback, don't accept fallback output
as proof the fallback condition holds — capture what actually triggered it (the wire, the
status codes, the timing; a 200-in-16 ms answer to an embedding request is a tell by itself),
and ask whether the trigger condition is real. The same audit pattern surfaced the rail: the
app "worked as shipped" while shipping the wrong thing. Related:
[[fleet-typeahead-audit-and-fixes]], [[verify-what-renders-not-what-commits]].
