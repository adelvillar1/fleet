---
name: canonical-skills-library-is-agents-skills
description: "the canonical source of skills is ~/.agents/skills - the owner's cross-harness library; fleet loads it directly (FLEET_SKILLS_DIR redirects) and speaks its frontmatter dialect, with proposal drafts staged fleet-local and the gate writing applies into the library."
type: project
date: 2026-10-16
basis: confirmed-by
tier: durable
shape: [docs]
phrases: [canonical skills, skills library, where do skills live, install a skill, SKILL.md source, agents skills]
missing: []
recall-count: 86
superseded-by:
valid-until:
---

The owner's ruling (2026-10-16): `~/.agents/skills` is the canonical source of skills, so skills are portable across harnesses. Fleet loads the library directly as of commit `0fb609c` — the earlier symlink stopgap (`harness-orientation` linked into the fleet home) is gone, and `<FLEET_HOME>/skills/` now holds only proposal drafts. The loader speaks the library's dialect: 227 of its 444 files carry nested or list frontmatter (`metadata:` blocks, `platforms:` lists) and load as opaque values beside the flat `key: value` lines. The gate's apply path writes applied proposals into the library; the drafter and curation stay fleet-local, and automatic retirement drafting is a no-op against the library (KI-215 — the usage scan reads fleet's sessions only). This is knowledge, not credentials: the own-home ruling still bars every other harness's config and credential store, and does not reach this library.
