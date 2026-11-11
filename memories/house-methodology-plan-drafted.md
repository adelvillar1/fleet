---
name: house-methodology-plan-drafted
description: "The plan to make the house judgment lessons travel — skills in ~/.agents/skills + a seedable fact pack — drafted and gated 2026-11-05"
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

The owner asked (2026-11-05) how much of fleet's development rigor is
product vs skills vs memories, then steered: plan both closers, with
the skills going into the canonical folder `~/.agents/skills`. The
tier answer was verified in code: Tier 1 product (plan→graph→gates,
desk evidence spine, calibration) rides every project; Tier 2 skills
transfer; Tier 3 (the judgment lessons) is corpus- and
convention-local — resolve_root_from (memory.rs:734-781) is
home-corpus-first then ancestor walk, so a jobhound run never sees
fleet's scar tissue.

The plan (`docs/plans/2026-11-05-house-methodology.md`, status draft,
9 criteria, plan-gate flags dispositioned-waived, arch-gate PASS):
six `house-*` skills in ~/.agents/skills (right-fixes,
evidence-discipline, declaration-is-not-execution, load-sensitive-
tests, mockups-first, immediacy) each carrying its rule WITH the
measured failure; a fact pack at `~/.agents/house/facts/` (NOT
~/.fleet/memories — the home corpus SHADOWS every project corpus when
it holds facts, resolve_root_from's own rule; the seed script refuses
that target by name); `scripts/house-seed.py --project <dir>` lands
the pack through fleet remember (procedure, not binary); dedup against
the existing 459 (verify-fix-actually-ran etc. stay authoritative);
scratch-project recall proof. Plan-gate flags waived with the
rotation evidence recorded (same signature as the desk plan's waiver).

**Why:** the owner's desk-brief ruling — process travels with the
operator — is only half-true today; this plan makes the judgment
lessons travel without binary changes.

**How to apply:** on owner approval the plan goes active; author the
skills dedup-first; never land the pack in ~/.fleet/memories. See
also [[knowledge-work-core-named-desk]],
[[canonical-skills-library-is-agents-skills]].
