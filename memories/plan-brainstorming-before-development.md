---
name: plan-brainstorming-before-development
description: "owner direction (recorded 2026-10-16/19, restated in the editorial-desk redesign) — before development, brainstorm and write a plan first; the owner has corrected this twice ("you should have created a plan first instead of jumping into development", "I need as part of the plan mockups of the ui"); for UI work the plan must carry mockups; 2026-10-20 an approved mockup applies the whole grammar, not just the picked screen"
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

The owner's standing direction, stated twice and recorded in
[[fleet-gui-editorial-desk-redesign]]: **brainstorm and write a plan before
implementation.** The two verbatim corrections: "you should have created a plan
first instead of jumping into development," and "I need as part of the plan
mockups of the ui."

Note on provenance: during the 2026-10-20 second-brain analysis I described
this rule in my own message rather than quoting an owner turn from that
session. The rule itself is the owner's, from the earlier passes; only the
2026-10-20 framing was mine.

**Why:** the owner wants to steer design and sequencing before code exists —
a wrong shape discovered after implementation costs a rewrite, and the owner
has repeatedly rejected shipped surfaces that were committed, served and not
what was approved ([[verify-what-renders-not-what-commits]]). A plan is also
the only artifact that can be approved by someone who has not seen the running
face.

**A plan can be rejected for what it omits, not only for what it gets wrong
(2026-10-20).** Three plans for the settings surface went back before one was
approved: the first scoped only the router (the owner: "we also need a similar
configuration surface for providers, for sys1, sdm1, sem1, and gen1 … I
shouldn't be writing json files"), the second listed surfaces but not the input
rule ("if we already have, for example a model name, then the user shouldn't be
forced to type it again"). So before writing the plan: inventory **every**
surface the request touches — for a settings ask, walk the crate for
config reads, env names, and CLI-only writers, and name which need treatment
and which are state rather than settings — and carry cross-cutting input rules
(pick vs type) as their own section, not as an implication of a layout.

**Mockups are a gate, not a plan attachment (2026-10-20).** The owner
asked the bot-PWA plan ([2026-10-20-bot-pwa.md] in the repo) to state that
mockups are needed **before building any part of the GUI**. The plan now
carries a mockup gate: static HTML mockups for every screen and state
under `docs/plans/mockups/<date>-<slug>/`, each rendered at the widths the
feature is for, with a dated owner-approval line in that folder's README;
no file in `src/gui/assets/` (and no router shell change) is written until
that line exists. The ASCII wireframes inside the plan are the brief and
cannot be the artifact a surface is accepted against. Backend increments —
reads and routes with tests, no visible surface — may proceed during the
review.

**A mockup the owner can navigate beats prose about a mockup (2026-10-20).**
When the owner said the second-brain panel was "rather underwhelming", the
deliverable given was a real interactive mockup — the full app shell, both
directions switchable, populated with the project's own `tower_data.json` — and
the source was left untouched pending the pick
([[fleet-second-brain-visual-redesign-awaiting-pick]]). That is the shape this
rule wants for a visual complaint: real data, real widths, nothing written in
`src/gui/` until the owner picks. The known failure it prevents: prose plans
about layout get approved on the words and rejected on the render.

**An approved pick is a grammar, and it carries to the next surface
(2026-10-20, the memory-and-skills pass).** The owner said memory and skills
"also [look] a bit underwhelming … compared to second brain they look like
meh" and asked for actions in the cards. The rebuild was delivered straight
to `src/gui/` with no mockup round — the Atlas composition (instrument in the
head, figures strip, dense left table, right rail, per-row actions) was
reused and the shared `surface-*` classes extracted
([[fleet-memory-and-skills-knowledge-tables]]). The owner then asked "I
thought we were working on the memories & skills using atlas … I really liked
what you were doing" — a signal that the mockup, not just its result, is the
artifact they want to *see* for each surface. Two rules follow: once a
direction is picked for the product, apply it to every sibling surface as the
default grammar (do not design each from scratch), and when rewriting a
surface the owner has called underwhelming, show the populated mockup of
that surface before shipping it.

**A gate names who approves, not who produces (2026-10-28).** B0 of the
bot-PWA plan sat open for a week while sessions reported it as "the
owner's gate"; the owner corrected it — "B0 is in fact session owned, the
session should have come up with best in class bot interfaces for me to
review. These mockups should come from deep research." So: when a plan
carries a mockup/approval gate, producing the artifact is the session's
job and must arrive unprompted (built from research where the plan says
so), and only the dated approval line is the owner's. What the mockups
should be is their own memory:
[[bot-pwa-b0-mockups-built-2026-10-28]],
[[mockup-review-matrix-is-a-tool]].

**How to apply:** when the request is a feature or a redesign, the deliverable
of the first turn is a plan (docs/plans/, with the status/updated frontmatter
the repo uses), not code. Brainstorm alternatives explicitly before choosing —
the "design it twice" posture the owner already endorsed for the GUI. For any
UI work the plan must include ASCII mockups and the interaction contract for
every new object (`data-act`, [[fleet-gui-editorial-desk-redesign]]). The
second-brain rework is the current case where this applies:
[[fleet-second-brain-is-seven-fields-in-the-gui]]. Related:
[[a-green-gate-covers-what-was-built-not-what-was-left-out]],
[[rebuild-is-not-commit-push-or-ship]].
