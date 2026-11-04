---
name: fleet-skills-invocation-is-model-only
description: "2026-10-22 — fleet skills are invoked only by the model (use_skill over the prompt's name+description list) and the usage scan measures 0 used of 442 across 20 sessions; the owner chose the reachability fix (trigger-shaped lines + a capped list + owner-performed retirement) and the plan is written and gate-PASSed awaiting voice approval"
type: project
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

How a skill gets invoked in fleet today (checked in the code 2026-10-22,
answering the owner's question):

- **The only path is the model's.** Every skill rides the system prompt
  as a name+description pair, and the model calls the `use_skill` tool
  with a `name` when it judges the request matches; the body is fetched
  into context on demand. An unknown name returns a typed error the
  model can read.
- **The owner has no way to name one.** No slash command, no picker, no
  @-mention. The Skills panel reads the library, asks the curator and
  approves proposals; the palette searches; the home chip navigates.
  Typing a skill's name in a task works only insofar as the model then
  decides to call the tool.
- **It has never fired.** The usage scan behind the Skills panel reports
  0 used / 442 never selected across 20 sessions scanned.

**Why:** the measured zero means the library is effectively unreachable
— the model has not once reached for it — and the likely causes are
dilution (442 description-first entries on the prompt), descriptions
written for a human browsing a library rather than as triggers a model
can match against a task, and the model defaulting to tools it already
knows.

**How to apply:** the owner chose the second direction ("let's go with
option 2" — make the model-driven path fire) and ruled the retirement
path: **"you can suggest a retirement and I can retire it on the skills
page"** — fleet suggests, the owner retires, the canonical library is
never written by anything but its owner. Measured constraints the plan
is built against: all 442 skills ride EVERY run's prompt (~16k tokens on
a 128k window — `SKILLS_PROMPT_SOFT_CAP` exists only as a growth LINE
above 50, no cap), and the retirement rule is a deliberate no-op against
the cross-harness library (KI-215 — the usage scan reads fleet's
sessions only), which is why retirement is fleet-local, reversible, and
suggested. **The plan `docs/plans/2026-10-22-skills-the-model-can-reach.md`
is written, plan-surface run, plan-gate PASS (5/5, 1/1), status `draft`
awaiting the owner's approval of two mockups (the prompt line before/
after; the skills row with its retire action)**; nothing is built.
**2026-10-23 UPDATE — APPROVED AND SHIPPED in `6d1a42d`** (plan
status `completed`, gate 1493): trigger-shaped prompt lines (a clip near
160 chars was added to C0 at build time so the criterion matched the
mockup), `SKILLS_PROMPT_CAP` 60 newest-by-mtime, and the owner's
retirement through `<home>/skills-retired.json` — the declared list is
read **per run** (the first reading was once at thread start, so a
retire needed a restart; that was the one mid-build correction), the
library file is hashed before/after, and `unretire` returns it. Live
proof: the next run's `skills` record declared 60 without the retired
name. Found while wiring it: `cron.rs` and `subagents.rs` call
`skills::load(home)` with the FLEET_HOME where the loader expects the
canonical library dir, so a scheduled run and a role declare NO skills
(KI-278/294, open). C0
trigger-shaped lines derived at prompt-build time (strip a leading "Use
when/Use this/Use for", lower-case the first word — no library file
rewritten); C1 a cap of 60 by the skill's own mtime; C2 retire/unretire
fleet-local via `<home>/skills-retired.json` through the one writer,
library file hashed before/after; C3 the suggestion is the scan's own
never-selected count; C4 the live proof (next run's `skills` record
declares one fewer). The owner-side explicit invocation (`/skill
<name>`) remains its own unbuilt plan. Related:
[[canonical-skills-library-agents-skills]],
[[fleet-gui-skills-manager-ruling]],
[[fleet-memory-and-skills-knowledge-tables]],
[[fleet-docs-revamp-direction]].
