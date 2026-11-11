---
name: sys1-validates-inferences-design-2026-11-07
description: "Owner-steered design direction 2026-11-07 — sys1 as the dream validator for inferred facts, upgrading basis to a third `validated` value, never confirmed-by"
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

Design discussion 2026-11-07, following [[memory-dreaming-comparison-2026-11-07]] and [[memory-substrate-deep-dive-2026-11-07]]: the owner asked "what about using sys1 to validate inferences, that would make them trustworthy?" The settled shape (my answer, owner not yet ruled):

- sys1 is the JUDGE, not the source of trust — validation only works evidence-grounded: fact + the session that produced it → sys1 answers supported / contradicted / unverifiable. Abstract plausibility voting is NOT validation. Prerequisite: the evidence pointer on facts (area 3 of the comparison doc — `Remember` carries basis but no source session today).
- A sys1 pass must NEVER stamp `confirmed-by` — that launders machine judgment into owner authority and corrupts the permission model (protected tier, approval-gated supersede, cycle leaves it, provenance 1.0). The honest record is a THIRD basis value (`validated`) between inferred (0.5) and confirmed-by (1.0): earned by evidence-checking, no owner authority, supersedable on justification.
- Fail-safe by construction: validation is an upgrade, so no sys1 answer → fact stays inferred (opposite of the fail-closed destructive gates).
- Carried into the plan: measure before the seat is awarded (curate-panel precedent — labeled set from ledger-proven-wrong facts), prioritize by recall_hit access rows (422 inferred facts, never drain in order; route cache caches verdicts), two open rulings: does `validated` supersede on justification alone (slice-12 rule), and does the cycle leave it like confirmed-by or fold it like inferred.
- Honest limits: nothing validates wrong confirmed-by facts, and transcript validation can't see environment drift (freshness handles time; only a later contradicting session handles drift).
- Article support: Anthropic's measured winning behavior was literally "marked answers confirmed after the grader checked them" — validation upgrading provenance is the pattern that won.

- Article support: Anthropic's measured winning behavior was literally "marked answers confirmed after the grader checked them" — validation upgrading provenance is the pattern that won.

**Naming resolved 2026-11-07 (owner asked "validated or judged?"): `validated`.** `judged` is TAKEN — the route-judgment work landed it as the plan vocabulary's status with judge_plan/plan_judge sys1 task names, and one-word-one-meaning is the house nomenclature rule. Division of labor in the grammar: the ACT is the calib row's op (`validate`), the VERDICT vocabulary is supported/contradicted/unverifiable, and the basis value names the fact's STANDING afterwards. Ladder: inferred (a guess) < validated (evidence-checked) < confirmed-by (the owner stood behind it); provenance weight between 0.5 and 1.0 is the plan's arithmetic. Basis::parse refuses a third value today — a schema change the plan carries. Recommended rulings: a validated fact still supersedes on justification that NAMES the new evidence; the cycle leaves it like the provenance it has partly earned.

**Landed:** the whole design folded into [[memory-dreaming-comparison-2026-11-07]]'s doc (areas 1, 3, the tensions' third item, does-not-settle) at `fa2aaf8` on face-plugin-host — and then IMPLEMENTED (see [[memory-dreaming-plan-implemented-2026-11-07]]): the `validate` task (calib Row::Validate, route-cache third namespace, dream::DreamValidator over LaneSession), `Basis::Validated` at `PROVENANCE_VALIDATED = 0.75` refused on every direct write (the dream pass is the only producer), both open rulings landed as recommended (validated supersedes on justification; the cycle's untouched set names it beside confirmed-by), and the C0 measured-set fixture test pins the pass's gate at `VALIDATE_AUTO_LAND_CONFIDENCE = 0.8`.

**Why:** the owner is steering the memory work toward this; the plan must open from this shape, not re-derive it.
**How to apply:** when drafting the memory improvement plan, open areas 1+3 from this design; use `validated` (never `judged`) for the basis value; get the two rulings before building.
