---
name: huddle-surface-ideation-2026-11-03
description: "project 2026-11-03 — the owner's ideation ask for a \"huddle\" section in the fleet app, sharpened across three turns to a Slack-shaped channel (members exchange messages, a bot addresses one with @name, deliverables post in-line, the existing right pane previews them; active/inactive/ended; one session per huddle; breakout huddles; bots bring their skills; sys1 judges destructiveness; the composer's ask/autonomous/full-access modes stay) plus three rulings — a bot's NAME is free and its AVATAR is optional, a…"
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

Owner ask 2026-11-03, verbatim: *"on the fleet app, the main rail currently
carries under work conversation (current and latest active conversation),
design (ported from design-canvas), and sessions (work sessions that are
development centric). We have a bots functionality that has no surface in
this app. I've been thinking that the right place would be a huddle section.
the design for this should be one that encourages conversations and work
with one or multiple bots for the same huddle, there could be multiple
active, inactive, and ended huddles. Deliverables should be tied to these
huddles (session with the owner and one or more bot). Help me ideate this
functionality."*

**This is an ideation turn, not a plan.** The response offered three shapes
(A the huddle is a *place* — a new `#/huddles` work destination with a
huddle rail grouped active/inactive/ended, the open huddle beside it, and a
deliverables strip; B the huddle is a *brief* — you name the deliverable
first and the huddle ends when it is handed over; C the huddle is a *team* —
standing members with cross-huddle memory and scheduling), recommended **A's
surface with B's spine**, and named six decisions that change the build
(how a bot takes a turn; one session per huddle or many; declared vs
collected deliverable; what ends a huddle; rail placement and the bot face's
fate; who may convene).

## The owner's rulings, as they landed (three turns)

**Turn 2 — the Slack metaphor** (verbatim): *"Imagine a conversation on slack
where different people are exchanging in a discussion and submitting and
collaborating on files. A huddle includes the conversations between the
members, if a bot needs to direct a message to a member the bot would use @
and the bot name so that the bot knows the conversation is aimed at them, the
deliverables would be posted in-line in the conversation, a right side pane
like the one we already have would allow for previews, browsing, etc."*

**Turn 3 — the answers to the three open decisions, plus three new rulings:**

1. **Members see everything.** *"a bot has access and can read all messages
   on huddle"* — the transcript is the shared room, not per-bot channels.
2. **`@` is a directive, an ask, or a review request — and bots may request
   reviews from each other.** *"a @ is a directive or an ask, but a bot can
   also suggest and request reviews from other members of the huddle."* So
   bot→bot review requests are in, not just owner→bot routing.
3. **Bots bring their skills, not their tools.** *"bots should have the
   skills they need from those assigned to them and skills related to the
   bot and the problem shape."* This answers the tools-vs-skills question:
   the huddle's tool set is shared, and a bot's difference is its **skill
   bundle** — its role's assigned skills plus skills ranked for the problem
   shape (the ranking seam is `tool_router.rs`'s sem1 ranking).
4. **One session per huddle.** *"yes, one session per huddle"* — Slack has
   one history per channel, and Fleet's session is the log.
5. **Breakout huddles exist.** *"breakout huddles are possible if the work is
   multi-phase or splits into other deliverables"* — a huddle tree, riding
   delegation's parent/child seam.
6. **The destructive directive stays, and sys1 judges it.** *"the destructive
   directive must remain, I also think that sys1 should judge what is
   destructive or not."* This **supersedes the static class list** the bot
   face ships today ([[one-goal-many-personas-destructive-is-the-only-gate]]):
   `write_file` and the bash destructive list are the current static
   classes, and the owner wants the judgment made by sys1 per action — with
   the verdict riding the seat as data, as it does now.
7. **The composer's modes stay.** *"The huddle can use the current composer
   modes of ask, autonomous, and full access."*

**Turn 4 — identity: the name is free and the avatar is optional.**
*"we also need to revisit the avatar proposal I had, because I might want to
rename my designer bot to 'bob' and give it an avatar, and that doesn't
change what fleet is."* The owner's own framing is the ruling: **a bot's name
and avatar are presentation, not contract** — renaming a designer bot to
"bob" and giving it a picture changes nothing about what fleet is. This
reopens the 2026-10-20 mockup's "no portraits" position
([[bot-pwa-is-assistant-centric-not-work-centric]], and the bot-PWA plan's
D10 which lists *"Portraits or illustrated avatars"* as ruled out) — the
owner is the one who ruled it, and they are revising it. The structural
argument in that mockup still holds and should be answered, not ignored:
personas are user-authored (`<home>/roles/<name>.md`, any name), so a
derived mark works for one role or forty while an image-per-persona stops
working the moment a new role file appears. The honest shape is **optional,
owner-supplied** — an avatar when the owner supplies one, the derived mark
when they do not — and the mockup's own escape hatch (a `mark:` frontmatter
key naming a glyph from a closed set) is the cheaper version of the same
idea.

**Turn 5 — the soul.** *"also bots should have a soul or personality,
optional of course, but could make things really interesting."* The
machinery already exists under another name: a role's **`overlay`**
(`roles.rs:45`) is a bounded (4,000-char) system-prompt body, editable in
the roles editor (`RoleFields.overlay`), and it rides the run whenever that
persona is the speaker. Absent, the bot is plain. Fleet also has the
owner's own `<home>/SOUL.md` identity slot (`home.rs:45`), read per run as
`identity` — so a huddle turn today carries **both** the owner's soul and
the lead bot's overlay, which is either a feature (your values shape every
bot) or a blur (bob stops sounding like bob). The response put three
decisions to the owner: whose soul rides a bot's turn (always / only on
your turns / a bot's overlay suppresses it); make the voice visible before
it speaks (a one-line voice per member in the huddle head); and a **voice
check** — a "hear bob" press that sends one greeting into a scratch session,
because editing a soul is blind today.

**Turn 6 — the owner answered, and named the guardrail.** *"the owner's SOUL
id the general guardrail over bots, but bots can express freely within the
confines of that SOUL as long as they remain truthful, honest, respectful,
and solution oriented, then they can have their own 'voice'."* So the
composition is settled: **the owner's SOUL rides every turn as the
foundation, the bot's overlay is the voice inside it, and the four virtues
are the standing constraint between them.** The response surfaced two
structural consequences the plan must answer: a written `SOUL.md`
**replaces** the compiled harness brief (`identity.unwrap_or(SYSTEM_PROMPT)`)
rather than adding to it, so a three-line guardrail would silently drop the
harness's read-before-write and verification discipline; and the overlay is
appended **last** in `composed_prompt` (`agent.rs:1629`), so if the four
virtues must outrank the voice the prompt needs a **closing constraint**
after the overlay, not just an earlier base.

## The audit's findings — the machinery is half-built, and this is the real gap

1. **The persona machinery exists and the bot face already uses it.** A
   session carries `personas` (join order, first = lead), a standing `goal`,
   and `Join`/`Leave` records; the bot PWA creates these today. The console
   reads `personas` and `goal` in its session rows already.
2. **Only the LEAD persona's role overlay rides a run.** In
   `src/agent.rs:1453` and `src/gui/bridge.rs:10515`, the run resolves the
   first persona's role overlay; every Assistant record is authored by the
   lead. A persona added with `+` is logged as joined and then never
   consulted — no tools, no skills, no voice. So "work with one or more bots"
   is a **mechanism** question, not a rendering one. The owner's `@` ruling
   is exactly the missing mechanism: a mid-run speaker switch, with a log
   record (hard rule 5).
3. **Delegation is the existing "another bot takes a turn" mechanism.** A
   delegated child resolves a role *before spawn*, carries that role's
   narrowed tool declaration and prompt overlay (`subagents.rs:1849`), and
   returns its result to the parent's log. A bot speaking in a huddle could
   be exactly that — a scoped child run under the huddle's session — rather
   than a second agent loop. This is also the breakout-huddle seam.
4. **Deliverables exist as scattered seams, none tied to a conversation:**
   the `send_file` tool (*"use it when the deliverable is the file itself"*),
   the `#/file/<path>` deliverable address, the design exports and hand-off
   packets. Collection from the log is nearly free; declaration up front is
   the stronger product.
5. **The Slack metaphor's elements already have seams** — per-turn persona
   bylines (`Record::Assistant.persona`), the lead role overlay, roles as
   tool/skill contracts, `send_file`'s in-line artifact card, the pane's
   `files · code · preview · explore` tabs, and `#/file/<path>` into the
   pane's preview. The genuinely new seams are the `@mention` as a mid-run
   speaker switch (with a log record), per-member **skill** scope, the
   sys1 destructiveness judgment, and the surface itself.

## Precedents this ideation answers (per [[a-new-plan-must-answer-the-past-plans]])

- **The bot face is assistant-centric, not work-centric**
  ([[bot-pwa-is-assistant-centric-not-work-centric]]) — the console huddle
  must be the *work* counterpart, not a copy of the persona inbox.
- **Both faces must stay available**
  ([[both-faces-required-console-and-bot]]) — a console huddle must ride the
  one bridge it already has, never the bot face's port or a second process
  (hard rule 12, [[self-containment-is-a-standing-contract]]).
- **The 2026-10-29 bots-page direction** ([[bots-page-on-the-personas-rail]])
  is the unbuilt draft that grouped sessions by persona with needs-you and
  handed-back — the huddle is its successor shape, and the draft's finding
  (the channels read carries no conversation; the sessions read carries
  personas) still holds.
- **The 2026-10-28 destructive ruling** ([[one-goal-many-personas-destructive-is-the-only-gate]])
  is reopened by the owner's sys1-judges-destructiveness ruling — the static
  class list is the thing being replaced, and the seat's `destructive` data
  field is the seam the verdict rides.
- **The 2026-10-20 "no portraits" ruling** is reopened by the owner's
  avatar/naming ruling — optional and owner-supplied is the shape that
  answers the mockup's structural argument.
- **Mockups-first** is the owner's own method for a surface rethink
  ([[plugins-ux-plan-drafted-2026-11-03]]) — three screens before the plan.
  The owner confirmed it for the huddle: *"yes, mockups are important."*
- **The owner's own ruling on where a live behaviour is verified**
  ([[live-behaviour-belongs-to-the-product-not-the-harness]]) — a huddle's
  turn-taking must be proven in the product, not only in the harness.

**Why:** the owner's instinct is that bots have no home in the work app, and
the honest answer is that the *surface* is the easy half — the run mechanism
for multiple bots is the half-built part. **How to apply:** when the owner
picks a shape, draft the plan with a "Precedents this plan answers" section,
and settle the turn-taking decision before drawing anything. The proposed
mockups were four: the huddle rail (active/inactive/ended, member marks,
deliverable counts, new-huddle composer); the open huddle (head with
goal/voice/members/state, transcript with @mentions and an in-line
deliverable, the composer with the mode chips and @ picker, the pane
previewing the deliverable); the seat (sys1's destructive verdict rendered
as data, with the approval controls and the mode's effect); and a breakout
(the parent's child huddles shown as a tree). The soul ruling carries into
the plan's prompt section: the SOUL is the guardrail, the overlay is the
voice, and the four virtues need a closing constraint because the overlay is
composed last. Related:
[[bot-pwa-persona-roster-six]],
[[one-goal-many-personas-destructive-is-the-only-gate]],
[[a-memory-index-line-is-not-the-plans-state]].
