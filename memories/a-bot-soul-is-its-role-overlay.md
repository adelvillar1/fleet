---
name: a-bot-soul-is-its-role-overlay
description: "owner ruling 2026-11-03 — bots should have an optional soul/personality; the machinery already exists as the role's `overlay` (a bounded system-prompt body that rides the run), and the owner's own SOUL is the general guardrail over the bots, within which each bot keeps its own voice — truthful, honest, respectful, solution-oriented"
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

Owner ruling 2026-11-03, verbatim: *"also bots should have a soul or
personality, optional of course, but could make things really interesting."*

**The machinery already exists under another name.** A role's **`overlay`**
(`src/roles.rs:45`) is a bounded (4,000-char) system-prompt body, editable in
the roles editor (`RoleFields.overlay`), and it rides the run whenever that
persona is the speaker. Absent, the bot is plain — which is exactly the
owner's "optional". So this is not a new feature; it is the existing seam
made visible and authorable.

**Fleet also has the owner's own soul.** `<home>/SOUL.md` — the identity
slot, sanitized and bounded, read per run as `identity` (`src/home.rs:45`).
So a run's prompt today carries **both** the owner's soul and the lead
bot's overlay. In a huddle that is either a feature (your values shape
every bot) or a blur (bob stops sounding like bob) — and it is the first
question to settle.

**The owner answered the SOUL question (turn 6), verbatim:** *"the owner's
SOUL id the general guardrail over bots, but bots can express freely within
the confines of that SOUL as long as they remain truthful, honest,
respectful, and solution oriented, then they can have their own 'voice'."*
So the composition is settled: **the owner's SOUL rides every turn as the
foundation; the bot's overlay is the voice inside it; the four virtues are
the standing constraint between them.** The four are named — truthful,
honest, respectful, solution-oriented — and the shipped `SYSTEM_PROMPT`
already carries truthful and honest in substance, so what is missing is the
explicit clause and the other two.

**Both consequences are now resolved in the plan**
(`docs/plans/2026-11-03-huddles.md`, C12 — see
[[huddle-plan-drafted-2026-11-03]]): the shipped default carries the four
virtues as a complete prompt so a short custom SOUL does not drop the
harness brief, and the composition closes with a standing constraint after
the overlay so the guardrails outrank the voice. The two open items below
(voice visibility, the voice check) are the authoring surface, not the
composition.

1. **The replacement trap.** `composed_prompt` builds the system prompt as
   `identity.unwrap_or(SYSTEM_PROMPT)` — a written `SOUL.md` **replaces** the
   compiled harness brief (read-before-write, verification discipline,
   decide-then-act, batching, honest-about-unverified-claims), it does not add
   to it. An owner who writes a three-line guardrail would silently drop all
   of that. Either the shipped default carries the four virtues as a complete
   prompt (and the settings identity slot shows them as the starting text), or
   the SOUL's semantics change to ride beside the harness prompt.
2. **The voice comes last.** The overlay is appended after the base
   (`agent.rs:1629` composes base → contract → goal → skills → memory →
   overlay → gui line → folder), and later prompt text tends to dominate
   earlier text. If the four virtues are non-negotiable, the cheap structural
   answer is a **closing constraint** after the overlay — *"Within this
   huddle you speak with your own voice; the guardrails are truthfulness,
   honesty, respect, and staying solution-oriented. They outrank the voice."*

**The two decisions still open (the voice-visibility pair):**

1. **Make the voice visible before it speaks.** A bot's soul is invisible
   until it posts; the huddle head could carry a one-line **voice** per
   member (the role's `description`, or a `voice:` line) so you know who you
   are convening. This is the persona card's substance surfaced in the
   member stack.
2. **A voice check.** Editing a soul is blind today — you write 4,000
   characters and hope. A "hear bob" press that sends one greeting into a
   scratch session would make the personality *authored*, not guessed.

**Why:** the owner's "could make things really interesting" is a request for
a bot to have a recognizable character, and the honest answer is that fleet
already carries one — the gap is that it is invisible and untested, not
missing. **How to apply:** when the owner asks for a personality, reach for
the role overlay first and say so; offer the voice line and the voice check
as the authoring surface; and treat the SOUL as the guardrail that rides
every turn, with the four virtues named as the constraint the voice lives
inside — which means the prompt composition needs a closing constraint, not
just an earlier base. Related:
[[huddle-surface-ideation-2026-11-03]] (the surface the soul rides),
[[bot-pwa-persona-roster-six]] (the role files that carry the overlays),
[[a-bot-name-is-free-and-its-avatar-optional]] (the other half of bot
identity, ruled the same day).
