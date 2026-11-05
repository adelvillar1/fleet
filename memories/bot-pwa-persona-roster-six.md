---
name: bot-pwa-persona-roster-six
description: "2026-10-28 — the bot-PWA persona roster as the owner ruled it: six personas, three of them DOMAINS (scholar = knowledge work, presenter = presentations, advocate = job hunting) beside tower/auditor/clerk; the skills they load already exist in ~/.agents/skills (453), so nothing is missing but the role FILES (~/.fleet/roles/ does not exist) and the session binding"
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

Owner ask 2026-10-28, mid-review of the B0 mockups (see
[[bot-pwa-is-assistant-centric-not-work-centric]]): **"I need one for
knowledge work, one for presentations, and one for job finding related
tasks. there should be a variety of skills related to that on the
canonical agent folder /.agents/skills"** — and, when that was being
checked, a second message that set the fallback and the real point:
**"if they don't we need to do a persona to skills mapping on fleet."**

**The roster, as now drawn in every mockup screen:**

| persona | domain | tint |
|---|---|---|
| `tower` | runs the work (the lead) | accent |
| `scholar` | knowledge work | teal (`--lane-0`) |
| `presenter` | presentations | magenta |
| `advocate` | job hunting | lime |
| `auditor` | checks what changed | amber (`--lane-2`) |
| `clerk` | keeps the ledger | violet (`--lane-1`) |

`scribe` was renamed `clerk` so no two marks share the initial `s` — a
renaming constraint any derived-glyph identity inherits: **one glyph per
persona, so names must not collide on their first letter.**

**The skills are already there — 453 in `~/.agents/skills`.** Verified by
listing the folder, not assumed: knowledge work has `llm-wiki`, `arxiv`,
`grounded-citations`, `documentation-consolidation`,
`documentation-refresh`, `meeting-action-items`,
`weekly-review-planning`, `data-triage`, `obsidian`, `notion`;
presentations has `slide-deck`, `presentation-layout-patterns`,
`branded-presentations`, `powerpoint`, `theme-factory`,
`data-visualization`; job hunting has `jobhunt-deliverables`,
`cv-tailoring`, `freelance-profile-building`,
`cold-email-outreach-deliverability`, `email-outreach-campaigns`.

**So the mapping mechanism exists, the files do not.** `src/roles.rs`'s
`Role` already carries `skills: Vec<String>`, parsed as a comma-separated
frontmatter list, and the roles editor preserves unknown keys — a role
file's `skills:` line IS the persona→skills mapping.

**RESOLVED 2026-10-28** — the six role files were authored in the repo at
`roles/*.md` (the source of truth) and installed to `~/.fleet/roles/`, so
every persona is loadable; a test asserts each `skills:` name resolves in
`~/.agents/skills` and that auditor admits no write tool. Two authoring
traps the first draft hit: role files must use CANONICAL tool names
(`edit`, not the `edit_file` alias), and names must stay distinct on
their first letter. The session binding landed the same day —
[[bot-pwa-implemented-2976098]]. The README's file shape, for the next
persona:

```markdown
---
name: scholar
description: Knowledge work — reads, cites, and says what it did not read.
tools: bash, read_file, write_file
skills: llm-wiki, arxiv, grounded-citations, documentation-consolidation
shapes: resolution, convention
---
Read before you write. Every claim that leaves this role carries its
source or says it has none.
```

**Why:** the owner's read of "a persona" is a *capability bundle*, not a
picture — the domains they named are work the agent does, and the skills
folder is where the capability already lives, so the work is authoring
role files, not inventing skills. **How to apply:** when asked for a new
persona, check `~/.agents/skills` for the domain first and spread its
skill names into the role file; never treat a missing skill as the
starting point without saying the folder was checked. Writing the six
role files was offered and not yet authorised (they land in the owner's
fleet home). Tints: opaque disc + per-theme ink, all ≥5.2:1 — see
[[bot-pwa-b0-mockups-built-2026-10-28]].
