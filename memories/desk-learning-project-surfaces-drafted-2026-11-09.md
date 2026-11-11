---
name: desk-learning-project-surfaces-drafted-2026-11-09
description: "project 2026-11-09 — the desk + learning redesign is BUILT in THREE owner-driven revisions; the final shape (0.6.19, served on 0.6.20) is the app's own work column (deliverables / lessons), the artifact in the CENTRE, the ONE chat composer pinned below, and properties+settings in the right pane that is EMPTY with nothing selected; 0.6.18 was burned"
type: project
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

Owner request 2026-11-09, after the inbox redesign: *"redesign both learning and
desk — they need the composer and to be more chat centric … a rail with projects
and deliverables … Special care on dead buttons and dead ends."* Then three
corrections on sight, each a re-cut: the composer to the centre (0.6.17), and
finally *"the composer component should remain identical to session, and
huddles … the rail should be reused from the other surfaces like sessions …
what changes is the objects shown on the rail … the deliverable goes on the
center screen with properties and settings on the right pane … does this match
the existing design system?"* — shipped as **0.6.19** (0.6.18's assets never
uploaded; see [[the-release-is-the-owner-s-no-op]]).

**The final shape (both surfaces, one pattern):**

- **The list column is the APP'S work column** — the same one sessions and
  huddles ride (same head, rows, fold, counts), with two new tenants:
  `paintProjectColumn('desk'|'learning')`. Objects: projects as `rooms-group`
  groups, deliverables/lessons as `work-row`s; a desk row carries the
  deliverable's real state (`1 refusal stands`, `all five stages done`, `not
  started`), an unparseable project names itself in the list. No second rail.
- **The centre is the ARTIFACT**, not just its conversation: the desk's
  deliverable cards (stage rail, claims, coverage, outline, review, end band)
  then its thread; learning's section then its thread. Nothing open → the
  surface's welcome built on the conversation's OWN `.empty` component.
- **The ONE composer is pinned at the bottom**, literally the chat's
  `<form id="composer">` moved into the surface's `.column` (`placeComposer`)
  and home again on the next route; only its focus (placeholder + send target)
  changes. Never draw a `.pw-composer`.
- **The right pane is properties + settings** in the pane's `card` / `facts` /
  `ghost` idiom, and it exists only while an artifact is open (`the work` tab
  absent otherwise; `face.wantContext` defers the tab switch until a lesson
  read lands). The owner's later word: *"if there is nothing on the project
  folder or no item is selected then the right side pane should be empty"* — so
  the pane's old shelf/greeting is gone from it too (`paneContextAvailable()`
  false with no artifact, and the shelf's content moved to the column and the
  centre's welcome).

**Design-system rule that came out of this:** when building UI, ask *does this
match the existing design system, does it conflict with it* — the welcome is
`.empty`, the pane is `.card` + `.facts`, settings are `.ghost`, and the
composer's run bar collapses by a **container query** on its own width
(`@container (max-width: 620px)`) because a work column is narrower than the
chat's.

**Verify path:** `PWS the desk/learning composer is the conversation's own`,
`PWS the composer comes home to the conversation`, `PWS the desk composer makes
a brief when nothing is open`, the pane's properties/settings pins; harness
1080/134, red-proved on the composer placement and the surface send. The desk
had NO deliverable on this machine, so its open layout is harness-proven only.
recap: `docs/recaps/2026-11-09-one-composer.md`.

Related: [[a-surface-is-a-conversation-in-the-centre-and-a-context-pane-on-the-right]],
[[a-chat-composer-is-borrowed-never-rebuilt]],
[[the-release-is-the-owner-s-no-op]].
