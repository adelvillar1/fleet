---
name: house-build-loop-sticky
description: "feedback 2026-11-07 — the structure-guard process (pins before edits, preflight, scoped runs, one gate, narration) is sticky across ALL projects, not fleet's; the house-build-loop skill plus its generic preflight template are the universal home"
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

The owner's steer (2026-11-07), after the dream-plan session paid six
compile cycles discovering guards: the structure-guard process "should
be applied to all projects, not just fleet … we need to make them
sticky."

**Why:** a session that discovers a project's guards by failing the
suite pays the expensive compile loop for knowledge a grep names in
seconds; the failure mode repeats on every project the moment a
session works outside fleet.

**How to apply:** the house-build-loop skill
(`~/.agents/skills/house-build-loop/`) carries the universal protocol —
pins before edits, the generic preflight before the first compile,
scoped test runs while iterating, one full gate at the end, a progress
line between cycles. Its `templates/edit-preflight.sh` discovers a
project's guards by pattern (include_str! text-scan sites, named
MAX_*/_TOKENS/_CAP bounds with their pinning tests, list-pin notes) —
no per-project config; SMOKED on a synthetic repo it had never seen
and it found the guard and the pin in seconds. Step 0 of the skill: a
session's first work in a project without the script installs it and
seeds the guard inventory from a two-grep sweep — the inventory is a
CACHE, not the source of truth (the preflight rediscovers every run,
so a stale inventory costs a cross-check, never a wrong answer). The
seeding is deliberately DEFERRED to each project's next session rather
than swept today: the script needs no inventory to work, and writing
inventories for projects without reading their guards would be
guessing. Fleet's instance: docs/architecture/structure-guards.md
(now naming itself as one instance of the protocol, 0264719) +
scripts/edit-preflight.sh (701e256). Related:
[[house-methodology-implemented]], [[the-build-loop-ux-verdict-2026-11-07]].
