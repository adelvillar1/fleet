---
name: lane-budget-a-slow-answer-beats-a-missed-one
description: "the lane ask budget is 300 seconds because reliability is the goal rather than latency: a problem that takes 15 or 45 seconds is irrelevant when the alternative is having models redo work because something was missed."
type: project
date: 2026-10-15
basis: confirmed-by
tier: durable
shape: [change]
phrases: [ASK_BUDGET is 300 seconds; a slow answer beats a missed one; the adapter surfaces blend1's differences rather than hiding them]
missing: []
recall-count: 50
superseded-by:
valid-until:
---

The owner's directive, stated verbatim: 'if a problem takes 15/45 seconds, that's irrelevant when you are having llm models redo work because we missed something; blend1 is an attempt at not missing things by using specialized models where it counts.' Two consequences. First, the budget: ASK_BUDGET is 300 seconds and the MCP client's call timeout takes it, so a real grounding ask is not cut off by a transport default. Second, the adapter's honesty rule: blending is pre-authorized when reliability demands it, but the adapter must surface blend1's differences - its verdict, its cost, its route - and never hide them, because a hidden blend is a claim the evidence cannot support.
