---
name: canonical-skills-library-agents-skills
description: "Owner ruling 2026-10-16 — ~/.agents/skills is the canonical skills library; fleet loads it DIRECTLY as of 0fb609c (symlink retired), FLEET_SKILLS_DIR redirects, proposals stage fleet-local"
type: project
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner ruled (2026-10-16) that `~/.agents/skills` is the canonical source of skills — the cross-harness library, so skills are portable. Implemented in fleet commit `0fb609c`: the loader reads the library directly (`resolve_skills_dir`: `$FLEET_SKILLS_DIR` → `<user home>/.agents/skills`), the frontmatter parser speaks the library's dialect (227 of 444 files carry nested/list blocks that now load), `<FLEET_HOME>/skills/` holds only proposal drafts, the gate applies proposals INTO the library, and automatic retirement drafting is a no-op (KI-215). Verified: 441/444 real-library skills card; the 5 skips are 2 non-skill dirs + 3 name-mismatches.

**Why:** "canonical" means one home — author and edit in the library only; the earlier symlink from `~/.fleet/skills/` was retired the same day.

**How to apply:** when creating or editing a skill for any harness, write it in `~/.agents/skills/<name>/SKILL.md`; frontmatter must carry `name` matching the directory plus `description`. This is knowledge, not credentials — it does not conflict with [[own-home-credentials-ruling]]. Related: [[fleet-harness-orientation-doc]], [[orchestrator-routes-through-hard]].

**Update (2026-10-16, later):** the 444-file library exposed KI-216 — the loader's skip diagnostics were bare stderr painting over the TUI's welcome pane; fixed in ac2d74e (`load_reported` reports to the caller, the TUI renders one SkillsLoaded card, the soft-cap growth line is headless-only). The five permanent skips are library facts, not fleet defects: `creative-ideation` (names itself `ideation`), `deli-autoresearch` (names itself `Deli_AutoResearch`), `dev-decisions.bak-20260926-124047` (backup dir), plus `tools` and `.git` (no SKILL.md). If the owner ever wants the first two loadable, fix the frontmatter `name` or directory in the library.

**Update (2026-10-17): the parser did not speak YAML block-scalar indicators, and every skill written the ordinary way was silently corrupted — fixed in `6db305e`.** A `description` written with `>` (the natural way to write a multi-line one) was read as the literal `>` followed by the block, so the model saw a description that began with the indicator and carried newlines. `presentation-layout-patterns` was among the affected; the ones that happened to use a plain scalar read fine, which is why it went unnoticed. The library **looked like it loaded fine** — 442 skills, no error — while every folded description was wrong, so this is the "verify the round-trip, not just the load" failure. `>` now folds (one value, indent stripped), `|` stays literal. Two subtleties worth carrying: the fold flag must outlive the **first** continuation line or only line one folds; and `|` keeps its lines *as written* rather than stripping the block's common indent — a deviation from full YAML that predates the fix and that the nested-block dialect depends on, named in the test rather than quietly tightened. After the fix: 442 load and zero descriptions begin with the indicator. When authoring a new skill, a plain single-line `description` is the safe shape; `>` now works too.
