---
name: fleet-compaction-budget-is-model-blind
description: "SUPERSEDED — see fleet-compaction-budget-is-window-aware. This file recorded the model-blind 100k budget as an open problem; the owner ruled \"it needs to be window aware\" and it shipped in ba08055 (2026-10-16)"
type: project
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

Superseded by [[fleet-compaction-budget-is-window-aware]].

This file recorded the open problem: fleet folded at a fixed
`COMPACTION_TOKEN_BUDGET = 100_000` whatever the model, its roster carried
no context-window field, and the own-home ruling forbade reading zcode's
`provider_config.json` where the windows live — the cause of the owner's
"compacting 5x more than necessary" on step-5-preview's 1M window.

The owner ruled on 2026-10-16: "it needs to be window aware." It is now
implemented (ba08055) — a compiled-in per-model window table in `route.rs`,
`compaction::budget_for`, and the window read off the committed route. The
successor file carries the design and the numbers.
