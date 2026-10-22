---
status: active
created: 2026-10-22
updated: 2026-10-22
slug: skills-the-model-can-actually-reach
---

# Skills the model can actually reach

## Context

The owner asked how skills are invoked. The answer measured: purely
model-driven — every skill rides the system prompt as a name+description
pair and the model calls `use_skill(name)` when a task matches — and it
has **never fired**: the usage scan behind the Skills panel reports 0
used, 442 never selected, across 20 sessions. The owner chose the fix
that makes the model-driven path work: trigger-shaped lines, and a
smaller list the model has to hold.

Three facts shape the work:

- All 442 skills ride every run's prompt today (`- name: description`
  per skill, ~16k tokens per run on a 128k-window model — 12% of the
  window before the owner has said anything). `SKILLS_PROMPT_CAP` does
  not exist; only a growth *line* fires above 50.
- The descriptions are written for a human browsing a library ("Use
  when needing to encrypt sensitive credentials…"), not as triggers a
  model can match a task against.
- Retirement is a deliberate no-op against the canonical library
  (KI-215): the usage scan reads fleet's sessions only, and the library
  is cross-harness. The owner's ruling for this plan: **fleet suggests,
  the owner retires on the skills page** — so retirement is a
  fleet-local, reversible state, and the library is never written by
  anything but its owner.

## Approach

Three changes, each in the order the surface meets them:

**The line the model reads.** The prompt's skills section becomes
trigger-shaped: `- \`name\` — use when <trigger>`, where the trigger is
the description with its leading "Use when/Use this/Use for" framing
already carried by most entries, and a mechanical re-frame for the rest
(lower-cased first word, the noun phrase kept). No library file is
rewritten; the derivation happens at prompt-build time, so a library
edit shows up in the next run without a migration.

**The list the model holds.** A cap (`SKILLS_PROMPT_CAP`, default 60)
on what a run declares, selected by the skill's own mtime — the
owner's recent work floats up — with the retired filtered out first and
the growth line naming how many are held back and that `fleet skills`
lists them all. A capped run costs ~2k tokens instead of ~16k.

**The retirement the owner performs.** A `retire` action on a skill
row, a fleet-local `skills-retired.json` under the home written through
the one writer, reversible from the same row, and the usage scan's
"never selected" number as the suggestion that sits beside it. A
retired skill leaves every run's declared list and stays in the
library, unchanged, one click from returning.

The voice is approved before a line is built: the prompt block before and
after, and the skills page's row with its retire action, are quoted in
this plan's approval request.

## Acceptance criteria

- [ ] **C0 — the line is trigger-shaped.** The prompt's skills section renders each entry as `` - `name` — use when <trigger> `` with the trigger derived from the description by the stated rule — strip a leading `Use when`/`Use this`/`Use for`, lower-case the first word, and clip at a word boundary near 160 characters (the trigger is the match signal, not the body; the body arrives on the call). Pinned by a unit test over four description shapes: each of the three frames, one with no frame, and one long enough to clip.
- [ ] **C1 — the list is capped and the cap is honest.** A run declares at most `SKILLS_PROMPT_CAP` (60) skills, ordered by the skill's own mtime (newest first) with the retired filtered out before the cap, and the growth line names how many are held back and where the full list lives. Pinned by a unit test: 70 skills declare 60, the newest 60, and the line's count is 10.
- [ ] **C2 — the owner retires, fleet-locally and reversibly.** `POST /api/skills/retire {name}` (and the row's action) writes the name to `<home>/skills-retired.json` through the one writer; the loader then excludes it from every declared list; the library file is byte-identical before and after (pinned by a test that hashes the SKILL.md); the row gains `retire`, a retired section lists them with `unretire`, and the panel's counts move (declared drops, retired rises).
- [ ] **C3 — the suggestion is the scan's own number.** The skills panel shows the usage scan's never-selected count beside the retire action on those rows (the suggestion is the number already measured), and `docs/features/skills.md`, CLAUDE.md's skills line, and the functional spec's skills section say what changed.
- [ ] **C4 — it is visible where it counts.** After the retirement, the next run's `skills` record in its session log declares one fewer skill and does not name the retired one; the served `/app.js` carries the row's `retire` action and the retired section; and one prompt line in the record reads `` `name` — use when … ``. The evidence is read from the running face and the log, per the live check in Verification, not from the unit tests alone.

## Fidelity to the current UI

The owner's standing constraint for this build: the affordances mirror
the panel that exists, and introduce no new visual language. The exact
patterns, read from the panel's own assembly
(`paintSkills`, `skillRow`, `skillsDecisions`):

- **The row action** — `rowAction('retire', …)` appended after `copy` in
  the row's existing `.row-actions` div: the same helper, the same
  `.row-action` class, the last position because it is the destructive
  one. The in-flight and refusal states mirror the decisions queue's
  approve: disabled with `retiring\u2026`, then
  `notice('the retire was refused', reply.body.error || '\u2026')` and
  `readSkills(true).then(paintSkills)`.
- **The counts strip** — one more entry in `surfaceStrip`'s existing
  array: `[retired, 'retired', retired ? 'down' : null]`.
- **The retired section** — a `boardCard(12, 'retired', <count>,
  <sentence>)` with `.lib-card`, appended to the existing
  `surfaceRail([skillsDecisions, skillsSkips, skillsCuration])` after
  curation; each row the decisions queue's `.rail-row` shape with an
  `unretire` `.row-action`.
- **The head sentence** gains the retirement clause in the panel's own
  voice; no new CSS class is introduced anywhere.

## Files to be touched

`src/agent.rs` (the prompt section, the cap), `src/skills.rs` (the
loader's retire filter, the derivation helper), `src/settings.rs` (the
retired list's writer), `src/gui/bridge.rs` + `src/gui/server.rs` (the
retire route), `src/gui/assets/app.js` + `index.html` (the row action,
the retired section), `CLAUDE.md`, `FUNCTIONAL-SPECIFICATIONS.md`,
`docs/features/skills.md`.

## Out of scope

Rewriting the 442 descriptions in the library (the derivation happens
at prompt time; a per-skill `Update` proposal remains the path for a
library edit, and it is the owner's gate that applies it). Making the
usage scan read other harnesses' sessions (KI-215 stays). Changing the
merge rule. An owner-side explicit invocation (the `/skill` command) —
that was option 1 and remains its own plan.

## Verification

The unit tests named above, and the gate.

**The live check (C4's home).** On the running face, in this order:
retire one skill from the skills page; read the next run's `skills`
record in its session log — the declared count is one lower and the
retired name is absent; read the served `/app.js` for the row action
and the retired section; and read one prompt-shaped line from the
skills record the log pins. The usage scan is the measure that matters
beyond this: after the trigger-shaped lines and a capped list, the
scan's `used` count should leave zero — and if it does not, the numbers
will say so rather than the claim.

## Linked artifacts

- `docs/features/skills.md` — the module's own record, updated with the prompt's shape and the retire path.
- `docs/architecture/surfaces.md` — the Skills surface's row, updated with the retire action.
- KI-215 — the ruling this plan works inside: the library is cross-harness, so fleet suggests and the owner retires.
