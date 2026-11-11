---
name: a-tier-fixture-must-exceed-the-bound-at-its-current-value
description: "feedback 2026-11-08 — a synthetic fixture that asserts \"the core's tail is withheld\" must exceed the surface bound AT ITS CURRENT VALUE; when a bound moves up, the fixture that was deliberately over-budget silently becomes under-budget and its assertion inverts"
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

`the_tier_withholds_the_core_only_when_even_the_deferred_lanes_cannot_fit_it`
builds a synthetic core whose descriptions are
`"a deliberately verbose tool description. ".repeat(N)` — deliberately
over-budget, so the tiering has a tail to withhold. When the hybrid-retrieval
plan moved `MAX_DEFINITION_TOKENS` to 10,700 (the fifth crossing), 24 × 40
repeats ≈ 9,200 tokens **fit under the new bound**, so the test reddened with
"the core's tail is withheld" — the fixture had silently become an
under-budget core. The fix was to enlarge the fixture (40 → 50 repeats, with
the comment naming the bound it must exceed) — and the same test reddened AGAIN
when the learning lane merged master's hybrid work and the bound moved to
10,800, because the fixture's over-budget margin was only ~12 tokens.

**Why:** a fixture that asserts a *mechanism* (the tier withholds) by
constructing an *over-budget* input has an implicit dependency on the bound's
current value. Moving the bound moves the fixture's meaning, and the failure
is a confusing one — the test is about withholding, not about the budget — so
it reads as a broken tier rather than a stale fixture.

**How to apply:** when a bound moves up, grep the suite for synthetic
over-budget fixtures and enlarge them — and enlarge them WELL past the bound,
not a handful of tokens past it. A fixture whose margin is ~12 tokens is a
fixture that will red on the next bound move, and the failure reads as a broken
tier rather than a stale fixture. The fixture's comment should name the bound
it must exceed ("50 repeats is ~10,400 tokens against a 10,800 bound").
The bound's own history is in [[a-declaration-budget-is-a-measurement-too]],
whose third crossing had the same class of downstream effect (the withholding
walk flipping declaration-list expectations back).
