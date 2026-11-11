---
name: house-methodology-implemented
description: "The house-methodology plan COMPLETED 2026-11-05 — six house-* skills live in ~/.agents/skills, the six-fact pack at ~/.agents/house/facts, the seed script; the plan is 9/9 at 0bb0440"
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

`docs/plans/2026-11-05-house-methodology.md` is COMPLETE (9/9, 0bb0440,
pushed). What landed, verified:

- **Six `house-*` skills in `~/.agents/skills`** (the canonical library,
  which is Fleet's loader default — `$FLEET_SKILLS_DIR` else
  `~/.agents/skills`, src/skills.rs:6-7 — so they went live with ZERO
  product changes): right-fixes, evidence-discipline,
  declaration-is-not-execution, load-sensitive-tests, mockups-first,
  immediacy. Each carries `shapes:` and its rule WITH the measured
  failure that earned it. `fleet skills` lists all six.
- **The dedup**: `verify-fix-actually-ran`, `regression-claim-verification`
  and `root-cause-first-debugging` stay authoritative for the adjacent
  ground; the house skills reference them and carry only the house
  judgment.
- **The pack** at `~/.agents/house/facts/` — six facts, `type: project`,
  `basis: confirmed-by`, YAML-list `shape:`.
- **`scripts/house-seed.py --project <dir>`** lands the pack through
  `fleet remember` (idempotent by the writer's collision refusal) and
  refuses the fleet home's corpus by realpath, both directions.
- **The scratch proof**: 6 facts ranked, `house-load-sensitive-class-…`
  FIRST for its own query.

**Three defects the build found, each worth remembering:**
1. The pack's first typing was `type: feedback` — which is a PROTECTED
   type, and **protected facts are excluded from the recall ranking by
   design** (docs/features/memory.md: they ride the protected tier of the
   turn assembly instead). A house *lesson* is a project fact. The
   `--recall` proof is what caught it.
2. The seed script's first refusal check compared a re-appended path and
   MISSED: it landed six facts into `~/.fleet/memories` — the exact shadow
   hazard the plan names. Purged by name (not `rm -rf` — the destructive
   gate blocks it), and the check now compares realpaths both ways.
3. Frontmatter values must be unquoted ONCE: passing a folded description
   with its outer quotes through to `fleet remember` produced a
   double-wrapped `""…""` on disk and a topic text starting with a quote.

**Why:** this is the Tier-3 close — the judgment lessons now travel with
the operator instead of living only in fleet's corpus and conventions.

**How to apply:** on a new project, `python3 -I
scripts/house-seed.py --project <dir>` seeds the scar tissue; the skills
route by shape anywhere. See [[house-methodology-plan-drafted]],
[[fleet-home-does-not-move-the-memory-corpus]],
[[process-tiers-product-skills-memories]].
