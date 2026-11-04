---
name: a-declaration-budget-is-a-measurement-too
description: "feedback 2026-10-24 — MAX_DEFINITION_TOKENS stayed at 8,000 while the surface grew from 19 to 28 declarations; the memory tools alone measured 8,034, so every run would have silently tiered behind tool_search. Measure the bound, state the measurement in the constant, and pin the surface against both bounds"
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

The tool-availability plan raised `MAX_VISIBLE_TOOLS` 28 → 32 with a measured
argument (the 30–50 degradation band) but left `MAX_DEFINITION_TOKENS` at
8,000 — a number set when the surface was nineteen declarations. Twelve
declarations later the native surface with memory measured **8,034 tokens**,
so *every* run would have crossed the bound and been tiered behind
`tool_search`: the native tools would have silently moved behind a search
call, which is a worse surface, from a constant nobody re-read.

**Why:** a count bound and a size bound guard the same thing from two sides,
and growing the surface moves both. The count bound got a comment explaining
its number; the token bound kept a round number and no measurement.

**The 2026-10-31 return — both bounds crossed by one landing, and the
second crossing was the dangerous one.** Adding the design canvas's two
tools ([[design-canvas-native-plan-2026-10-31]]) took the native surface to
**9,575 definition tokens** against 9,000 and to **30 declarations**; 30
native plus a joining lane's four then crossed **`MAX_VISIBLE_TOOLS = 32`**,
which meant **a lane's own tools were deferred the moment it joined** — the
tier doing the one thing it must not do. Both were caught only because the
surface-curation plan had left the pins behind: `the_native_surface_fits_its_own_bounds`
(which reddens on the *next* declaration past the budget) and
`a_full_run_calls_the_real_sem1_lane_through_the_mock_api` (a joining lane
declares its four tools). The bounds moved to **10,000 tokens / 36 tools**,
each with the measurement and the reason in its doc comment. The lesson
sharpens: the count bound must clear **native + the largest lane you might
join**, because at that margin the deferral silently turns on for every run
that meets a lane, and every fixed-count assertion in the suite reddens at
once (seven expected-name vectors, a `second[11..28]` slice, three
`declared_tools(..).len()` pins) — the count is the cheapest thing to
notice and the least important.

**How to apply:** when a bound's input set changes, re-measure it rather than
only re-reading the constant — and put the measurement in the constant's doc
comment (`the native surface measures ~6,500, memory adds ~1,550`). Then pin
the surface against *every* bound in one test, so the crossing is a red gate
and not a survey. Related:
[[tool-declaration-assertions-read-names-not-just-counts]],
[[a-fixture-corpus-flatters-a-retriever]] (a threshold is a measurement),
[[fleet-tool-availability-gaps-shipped]].
