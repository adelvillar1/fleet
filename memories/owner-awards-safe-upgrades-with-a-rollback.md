---
name: owner-awards-safe-upgrades-with-a-rollback
description: "Owner ruling pattern 2026-10-30 — 'award the seat and we roll it back if needed': when evidence is neutral-safe (identical decisions, zero mis-fires) and the rollback is one line, the owner grants the capability the gate withheld"
type: feedback
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

2026-10-30, after the J6 replay measured the curate judgment at identical decisions / zero mis-fires but "no domination" (three of four votes under the confidence gate, no early fire), the plan's contingency shipped advisory — the lane votes, the cosine decides. The owner overrode it: **"award the seat and we roll it back if needed."** The seat was awarded (`CURATE_JUDGED_DECIDES = true`, b16597a) with the rollback pinned in the same commit.

**Why:** the owner distinguishes two evidentiary situations. Broken evidence (the clef shadow A/B: 32% workload agreement, real downgrades) means don't promote — that ruling stands. But *neutral-safe* evidence (identical decisions, zero mis-fires) plus a one-line rollback means the conservatism of holding the gate costs more than the risk of the seat: the upgrade is welcome, the telemetry to detect a mis-fire is already being written, and the rollback is trivial. "If needed" is a real commitment — the rollback must be shaped before the award, not after.

**How to apply:** when a plan carries a contingency whose failure branch downgrades a capability to advisory, don't leave the downgrade as the permanent default waiting for future evidence — present the award-with-rollback option explicitly when the evidence is safe-but-not-dominating. Shape the rollback first (pin both behaviors side by side, write the telemetry that would justify a rollback), then flip the constant on the owner's word. The award's next word strengthens the mechanism rather than hedging it: *"use 3 sys1 judges and best of 3 wins"* (caa5f2f) — when the owner grants a capability on neutral evidence, expect him to also upgrade HOW it decides (a majority panel over a single judge), so design the seat so the vote mechanism is swappable without touching the composition. Related: [[clef-decision-model-assessment-2026-10-27]] (the broken-evidence counter-case), [[curator-judgment-plan-2026-10-30]].
