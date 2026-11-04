---
name: the-room-hires-its-own-consultant
description: "owner ruling 2026-11-04 — a domain the room has no one in (financial analysis, linear algebra, non-linear optimization) is bridged by authoring a purpose-built persona, and the model does it itself; landed as author_persona"
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

The owner's ruling, after I asked why a room couldn't just spin up a persona it
needed:

> *"let's say you have a problem that requires financial analysis, or linear
> algebra, or non-linear optimization... we don't have any personas that are
> even remotely close to that, the only way to bridge the gap is to onboard a
> purpose built persona"*

This closes the question [[not-every-gap-is-a-persona]] left open. My
separation there was right about skills (answered by `use_skill`) and tools
(widened in the speaker's own role), but I stopped at "a voice gap is the only
real hire" — and then made that hire owner-pressed. The owner's point is that
**a domain is a hire, and the room makes it itself**. Nothing constrains a
huddle to the six roles; the consultant is the bridge, not a workaround.

**Landed in abedc83:** `author_persona` is a model-facing tool, declared on
every run that has a home (deliberately *not* behind the speaker's narrowed
scope — it is the escape hatch scope would otherwise remove). The model calls
it with a name, description, tools, skills and voice; the handler writes the
role through `roles::write`, records the `Join`, makes the new persona speak
next, and leaves it in the home's roles for every later huddle. It is not
asked-for: the model hires when the gap is a domain, and the owner's seat still
approves the call like any other tool, so a self-authored consultant cannot be
handed `bash` and `write_file` without being seen.

**Why:** the gap is not a dead end and not a blocker — it is the room saying
it needs someone it does not have, and Fleet's role machinery
(`roles::write`, `RoleFields`, the settings role op) already makes authoring
one a form rather than a code change. Making the owner press a card turned a
hiring decision into a permission slip.

**How to apply:** when a room hits a domain it lacks, the answer is "who do we
need, and what does that person carry" — and the model writes that person. The
gate is scope, not approval-by-default: auto-hire unless it would expand the
room's power (destructive tools, a new provider, a standing identity the owner
should see), which the seat already covers. This supersedes the owner-pressed
card in [[a-capability-gap-is-a-hiring-decision]] — that ruling stands, its
mechanism does not. And it is the answer to "why wouldn't you just do it":
because I had collapsed three kinds of gap into one card and then made the one
real kind a button.

Related: [[huddle-plan-implemented-12-of-14]] (C5's state, C14's run),
[[a-capability-gap-is-a-hiring-decision]] (the ruling this supersedes in
mechanism), [[not-every-gap-is-a-persona]] (the separation — skills and tools
are answered in place, domains are hired),
[[a-bot-soul-is-its-role-overlay]] (the persona's voice is the role overlay),
[[a-bot-name-is-free-and-its-avatar-optional]] (a persona's name and face are
the owner's).
