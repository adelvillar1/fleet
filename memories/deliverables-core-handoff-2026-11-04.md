---
name: deliverables-core-handoff-2026-11-04
description: "project 2026-11-04 — a handoff brief for a third artifact core (knowledge work / deliverables) was assessed and verified; the plan is not yet drafted and two owner rulings are open"
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

The owner received (likely from another session) a handoff brief for a
**knowledge work core** — a third artifact core beside sessions (code)
and the design canvas: a brief-to-deliverable loop (brief → grounded
claims → outline → render → review → project knowledge) with the
extraction leg's jobhound floors as the evidence spine. I verified its
load-bearing claims against the tree and they nearly all check:
`src/extract.rs` carries LOCATE→SEGMENT→VERIFY→RECONCILE with
ACCEPT_FLOOR 0.85 / VETO_FLOOR 0.35 (lines 68-69), `src/deck.rs` has
Refusal::Pyramid/MECE/bolding + LayoutFault:713 + compress_text:940,
`src/documents.rs` Route::{Local,Pdf,Escalate,Unsupported} with the
macro-as-XML refusal (test at :340), `src/canvas.rs` project.json at
`<home>/canvas/<slug>/`, the `frame` tool (tools.rs:298), `presenter`
role (roles.rs:699), and the PPTX "honest partial" is quoted from
docs/features/gui.md:118. `src/deliverables.rs` does not exist yet.

Brief errors found: it FABRICATED a "sandcherry-deek-builder" typo (the
on-disk skill is spelled correctly) and named "AutoFit" which is not in
deck.rs — reinforcing [[say-only-what-you-have-read]].

Unsettled before a plan: (1) prose/document deliverables have NO
refusing checks — deck's law is presentation-shaped; (2) briefs name
URLs as source material while the extraction path is out-of-scope for
network — needs a v1 boundary ruling; (3) `shapes:` frontmatter is a
roles.rs schema change the plan must carry.

My recommendations given (owner has not ruled): name the surface
**deliverables** (the codebase already says "deliverable shelf" in
huddles-ux; "knowledge" is overloaded — memory substrate, room
knowledge, project knowledge, corpus); the brief composer lives on the
deliverables page directly, design-canvas idiom, per
[[a-deferred-action-is-an-error-to-the-owner]]. Next step when the
owner says go: draft docs/plans/2026-11-04-deliverables.md, mockups
first.
