---
name: fleet-compaction-budget-is-window-aware
description: "fleet's compaction budget now follows the serving model's declared context window (1060c19 on master, 2026-10-16; originally committed as ba08055, which a rebase left dangling) — a compiled-in per-model table in route.rs feeds compaction::budget_for, which folds at 3/5 of the window clamped to [100k floor, window-32k edge]; step-5-preview went 100k -> 614,400"
type: project
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

The owner's report (2026-10-16): "we were having models like step 5 preview
with 1M token window set at 200K making things really slow because it was
compacting 5x more than necessary." Two independent causes, and fixing only
one of them does not help:

1. **The zcode-side window** (fixed): step-5-preview's
   `properties.contextWindow` traced 128000 → 1000000 → 1024000 across the
   backup chain and is now correct — see
   [[zcode-provider-config-degrades-on-validation-failure]]. That window
   governs zcode's own UI, not fleet.
2. **The fleet-side budget** (fixed, `1060c19` on master — the original
   commit `ba08055` is not an ancestor of HEAD, so cite the master SHA): fleet folded at
   `compaction::COMPACTION_TOKEN_BUDGET = 100_000` regardless of the serving
   model. On a 1M-window model that is ~10x more folding than the window
   requires, and every fold costs a full summarize request and leaves a
   lossier conversation.

**The design (the owner's word: "it needs to be window aware"):**
- `route::context_window(provider, model) -> Option<u64>` reads a compiled-in
  `CONTEXT_WINDOWS` table in `route.rs`. The lookup is case-insensitive.
- `compaction::budget_for(window)` derives the budget: three fifths of the
  window, clamped to be at least the 100k floor and at most window − 32k.
  No window declared → the floor, which is the pre-window behaviour.
- `Agent::serving_window()` reads the window off the **committed route**
  (`client.route_decision()`), not the options — so a run whose router
  failed over compacts against the model that actually served, not the one
  it asked for.
- Result: step-5-preview 100k → **614,400**; step-3.7-flash → **157,286**;
  GLM-5.3-Flash → **600,000**; deepseek-chat → **96,000** (edge-held on its
  128k window).

**Why the table lives in fleet rather than being read:** the own-home ruling
(see [[fleet-roster-migrated-from-router-kit]]) forbids reading another
harness's config at runtime, and zcode's `provider_config.json` is where the
windows actually live. So the values were taken once from the owner's own
config and are *declared* here — a drift between the two is a declaration to
update, not a second truth to reconcile. Every model in the compiled roster
is covered except the two that are deliberately not: the blend1 vendored
lane (its window is the grounding provider's) and liquid. A test fails if a
roster model silently drops out of the table, so a typo cannot quietly
revert a model to the floor.

**The 32k reserve is for a typical long response, not the model's maximum
output.** A budget sized to `maxOutputTokens` would fold every turn on
step-3.7-flash (256k window, 256k max output) — a model that *can* emit 256k
does not emit them every turn.

Note the two token fields are different things and easy to confuse: the
window is `config.properties.contextWindow`, the output cap is
`config.optionSpecs.maxOutputTokens.max`.
