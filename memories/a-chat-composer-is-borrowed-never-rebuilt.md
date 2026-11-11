---
name: a-chat-composer-is-borrowed-never-rebuilt
description: "owner ruling 2026-11-09 — a chat is the only way the owner talks to the harness, so every surface reuses the ONE composer (the chat's own element, moved into the surface's column); only the deliverable/focus changes, and the list column is the app's own with different objects"
type: feedback
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

Owner, 2026-11-09, after I shipped a `.pw-composer` per work surface:
*"the composer needs to be pinned to the bottom like the other chat surfaces, as
a matter of fact the composer component should remain identical to session, and
huddles. why we have so many different composers"* — then *"the composer is how
you communicate with the harness, I don't get why you are generating new
composers … the communication is always typed, what changes is the deliverable,
the deliverable goes on the center screen with properties and settings on the
right pane"* — and *"the rail should be reused from the other surfaces like
sessions … what changes is the objects shown on the rail"*.

**The rules:**
- **One composer, one element.** Move the chat's own `<form id="composer">`
  into the surface's column (`placeComposer`) and move it home when the route
  leaves; the surface changes only its **focus** (placeholder + what the words
  address). A second composer is a second thing to keep in step — the owner
  sees it immediately as inconsistency.
- **One list column.** Reuse the app's work column as a tenant, not a bespoke
  rail; different objects (conversations, rooms, deliverables, lessons), same
  head, rows, fold and counts.
- **The artifact is the centre screen; properties and settings are the right
  pane.** Never the artifact in the pane and the conversation in the middle.

**How to apply:** before adding any control that types, sends, lists or pans,
find the app's existing component and borrow it; if the shape does not fit,
the mismatch is the design question, not a licence to fork it. Ask *does this
match the design system, does it conflict with it*.

Related: [[desk-learning-project-surfaces-drafted-2026-11-09]],
[[a-surface-is-a-conversation-in-the-centre-and-a-context-pane-on-the-right]].
