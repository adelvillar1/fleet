---
name: hires-are-a-catalog-with-engagements
description: "owner ruling 2026-11-04 — a niche hire is just storage, so a hire already made is brought up rather than authored again; engagements are standing or temporal; and a huddle is a KNOWLEDGE SPACE (lessons and process improvements, not just deliverables)"
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

The owner's three corrections, in one exchange:

> *"2) a hire could be temporal, if it's a skill that's really niche, but it
> doesn't cost you anything because it's just storage space, if you have a
> catalog of hires you can bring them up instead of creating them."*

> *"3) because I'm using coding/token plans cost is not a concern, getting the
> job right is the primary concern because I can use many models that cost
> less because the heavy decision making load is on the lanes"*

> *"1) I would expect that a huddle would not only bring deliverables, but
> lessons learned, process improvements, etc. a huddle is a knowledge space."*

**The catalog (landed f9c2676).** The authoring door now searches the home's
roles FIRST and offers a match with a **convene** press; a genuinely new
capability still gets the form. A hire is storage, so reuse is free — and
[[the-room-hires-its-own-consultant]]'s "the model authors it" is completed by
"and only when the catalog does not already hold someone".

**The engagement (landed f9c2676).** `Join` carries `temporal: bool`. A
standing hire stays in the catalog; a temporal one joins for this huddle and
is released when the room is archived — its role file stays, so the next
engagement brings it up instead of authoring it. Pinned by
`a_temporal_hire_leaves_when_the_room_ends`.

**Cost is OUT.** I had proposed per-room spend visibility as the thing that
makes "hire freely" safe. The owner's answer: on coding/token plans the cost
is not the constraint, and the heavy decision load is on the lanes (sys1 and
friends), so many cheaper models can carry the work. **Do not raise cost as a
reason to gate a huddle behaviour** — that framing is wrong for this owner.
Related: [[fleet-anti-hermes-constitution]] (the standing anti-goal is
complexity, not spend).

**The knowledge space (the next build).** The owner's point 1 is the biggest
of the three and is still open. A huddle is not a chat that produces files; it
is a room that accumulates — lessons learned, process improvements — and the
next room should inherit them. Fleet already has exactly two knowledge
substrates, and they map cleanly:

| kind of lesson | Fleet's home | how it is recalled |
|---|---|---|
| a fact about the world ("the launch page's hero needs a decision") | a memory fact (`memories/<name>.md`) | ranked by meaning into the next relevant run |
| a better way of working ("when reviewing contrast, do X") | a skill / skill proposal | declared to the model, loaded on demand |

So the design I proposed and the owner has not yet ruled on: **(1)** the room
shows its knowledge — what it recalled and what it produced (facts written,
skill proposals filed) beside the outcomes; **(2)** a **distil** action at
the end (or any time) turns the room's lessons into facts and skill proposals
through the existing writers, with the owner reviewing the proposals as now;
**(3)** the rail's outcome count widens to include them, so a room that
produced no files but taught the home three things reads as *three*, not
zero; **(4)** the next room benefits automatically — facts recall by meaning,
skills declare by trigger, no new store.

**Why:** the owner is describing the room as an *occasion for distilling into
Fleet's existing memory*, and the context that gets the right knowledge
recalled — not a new artifact type. I had been counting deliverables, which
is one kind of output and not the point.

**How to apply:** when the owner says a surface is a knowledge space, look for
the existing substrate that already stores that kind of knowledge before
proposing a new one — here memory facts and skills, both with writers and
recall already built. And never re-raise per-room cost: the owner's plans make
it a non-constraint.

Related: [[huddle-plan-implemented-12-of-14]] (the plan's state and the
outcomes half still open),
[[delivery-is-not-behind-the-scope]] (KI-410's fix and the outcomes gap),
[[a-delivery-is-not-a-file]] (the delivery kinds),
[[the-room-hires-its-own-consultant]] (the hire the catalog now precedes),
[[a-capability-gap-is-a-hiring-decision]] (the ruling that opened this).
