---
name: canonical-skills-library-is-agents-skills
description: "the canonical source of skills is ~/.agents/skills - the owner's cross-harness library; fleet loads its copies from <FLEET_HOME>/skills/ and a skill's one home is the library, the fleet-home copy installed from it."
type: project
date: 2026-10-16
basis: confirmed-by
tier: durable
shape: [docs]
phrases: [canonical skills, skills library, where do skills live, install a skill, SKILL.md source, agents skills]
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner's ruling (2026-10-16): `~/.agents/skills` is the canonical source of skills. A skill is authored there once; what fleet loads from `<FLEET_HOME>/skills/` is installed from the library — `harness-orientation` is the precedent, symlinked from the library into the fleet home so the loader sees it without a second home. This is knowledge, not credentials: the own-home ruling still bars every other harness's config and credential store, and does not reach this library. The drafter's proposals stay fleet-local under `<FLEET_HOME>/skills/proposals/` until the gate applies them.
