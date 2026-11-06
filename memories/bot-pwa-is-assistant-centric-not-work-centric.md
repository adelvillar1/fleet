---
name: bot-pwa-is-assistant-centric-not-work-centric
description: "owner ruling 2026-10-28 — the bot-PWA surface must be ASSISTANT-centric (personas are the axis), not a condensed work-centric console; the inbox now groups by persona, the workspace left the list, dispatch asks \"who takes it\"; and the blocking dependency it exposes — a session records no persona, only roles.rs exists and only inside a delegation"
type: feedback
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

Owner correction 2026-10-28, on the first pass of the B0 mockups (see
[[bot-pwa-b0-mockups-built-2026-10-28]]), verbatim: **"this is just
rehashing what fleet does which is work centric. The whole bot idea is
assistant centric."** Then, on the dependency below: personas should be
associated with bot-assigned conversations, and the inbox must say so.

**What was wrong.** The first pass grouped sessions by *state*, put a
workspace path in every row, and dispatched with "what should it do?" —
the console's frame with a phone's chrome. It was the exact failure the
plan's own risk section names (a plan building against a frame that
already exists instead of the one it is for).

**The inversion the mockups now carry** (`docs/plans/mockups/2026-10-20-bot-pwa/`):

- **The unit is a persona, not a session.** Six, and three are DOMAINS
  rather than jobs — `scholar` (knowledge work), `presenter`
  (presentations), `advocate` (job hunting) — beside `tower` (runs the
  work), `auditor` (checks what changed) and `clerk` (keeps the ledger).
  Conversations group under them; a persona rail filters in one tap. The
  roster, its skill mapping and the missing role files:
  [[bot-pwa-persona-roster-six]].
- **The row leads with the persona's mark, then what it said.** The
  workspace is gone from the list; it appears once in the conversation's
  meta line (`the gate on the fix · in ~/Projects/fleet`). Where the work
  happens is not who you are talking to.
- **The conversation header is the persona** (mark, name, role); the
  conversation's own title moved to that meta line.
- **Dispatch asks "who takes it" first**, then the brief, then the folder.
- **Urgency is an entry point, not the ordering.** Needs-you is a compact
  two-line strip above the personas, never the screen's structure. (This
  sits beside [[the-owners-work-is-the-view]] and the 2026-10-23 ruling
  that project-first grouping was wrong as the *work* surface.)
- Opening a persona shows what it is — role, tools, skills, memory
  shapes — not a mascot.
- **The persona's identity is a derived mark, not a portrait**: its serif
  initial on an opaque tint from the console's lane palette, with a
  per-theme ink computed to clear 5.2:1 in paper, console and graphite.
  The owner asked for recognisability and then named the bar —
  "the tint needs good readability on both themes for this experience" —
  so a tint is only allowed to be a *design choice* once it also passes
  as a measurement.

**The blocking dependency, and it was real — now closed.** fleet already
had the persona primitive: `src/roles.rs` → `Role { name, description,
tools, skills, shapes, overlay }`, read from `<home>/roles/<name>.md`,
rendered by `fleet roles`, and it narrows tools and prefixes the prompt.
But a **session log recorded no persona** — it records `kind` (scheduled
/ delegated / owner) and channel, and a role existed only *inside* a
delegation, as the `subagent` tool's argument. The smallest step named
here grew, on the owner's inter-persona ruling, to `goal` + a `personas`
SET + join records in the log — and all of it LANDED 2026-10-28 in
`2976098`: `create_with_persona`, `Join`/`Leave` records, the summary's
folded set, and the inbox carrying both facts. See
[[one-goal-many-personas-destructive-is-the-only-gate]] and
[[bot-pwa-implemented-2976098]]. Still open: whether an edited role file
retro-changes existing conversations, whether a persona owns a model, and
the vocabulary — fleet says *role*, the plan said *bot profile*, the
owner's usage settled on *persona*.

**Why:** a bot surface that mirrors the console's work frame has no
reason to exist, and the owner reads it as rehash first. **How to
apply:** design assistant-first surfaces around identity (who you talk
to), demote the work's location to a detail, and before drawing such a
surface check which identity primitive the backend actually persists —
a drawn persona is worthless if no session can carry one. Related:
[[fleet-persona-bots-fit-as-bot-profiles]] (the original assessment and
the Bots-vs-Access naming split), [[bot-pwa-b2-b3-landed-2026-10-26]]
(the backend the surface reads), [[plan-brainstorming-before-development]].
