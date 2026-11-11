---
name: a-surface-is-a-conversation-in-the-centre-and-a-context-pane-on
description: "owner ruling 2026-11-09 — a work surface is chat-centric in the CENTRE (the artifact's conversation and composer), with the app's own collapsible pane on the right whose leading tab is the surface's own work; never the composer parked in a side column"
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

Owner correction 2026-11-09, one message after reading the new desk surface:
*"I think that for consistency, the composer should be in the center … the right
side should have the collapsible pane, like we do in sessions, design, and
huddles. Let's rework it around that, center for the composer and the right side
pane like we do in the other surfaces."*

**The pattern (now the house's for every work surface):**

- **Projects rail on the left** — what the work is organised by.
- **Centre: the artifact's conversation and the composer** — chat-centric, so the
  owner iterates in the place they read.
- **Right: the app's own pane** (`run / files / preview / web`), the SAME
  component sessions, design and huddles carry, opened on arrival and closable —
  with a leading **context tab** (`the work`) whose content is whatever the
  route is about.

**Why:** consistency beats a bespoke third column; a surface that invents its
own right-hand column for the thread reads as a different app, and the pane is
already the app's idiom for "the thing beside the conversation".

**How to apply:**
- Never park a composer in a side column of a work surface; the thread + composer
  sit in the centre, full width of that column.
- The pane's context tab must be route-driven: hide it where there is no
  context, and fall back to `run` when a pane showing it lands on such a route —
  the tab is a navigable control, so a stale one is a dead button
  ([[a-queued-control-that-silently-returns-is-dead]]).
- A pane body swap must hide the other bodies: a paint that returns early after
  showing one leaves the others on screen (found live here).
- Bound long lists (`max-height`) so the composer never falls below the fold.

Related: [[desk-learning-project-surfaces-drafted-2026-11-09]],
[[a-rails-primary-taxonomy-is-the-owners]].
