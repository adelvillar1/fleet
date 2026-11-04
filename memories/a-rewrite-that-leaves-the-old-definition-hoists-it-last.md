---
name: a-rewrite-that-leaves-the-old-definition-hoists-it-last
description: "feedback 2026-10-30 — rewriting a JS painter while leaving the old definition in the file means function hoisting makes the OLD one win: the new rail code never ran, the board showed the pre-rewrite waiting card, and the freshly-read new code looked correct while being dead"
type: feedback
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

While landing the workspace-history rail, the new `paintWorkspaceHistory`
was written above the old one and the old definition was left in place.
**JS function declarations hoist and the LAST definition in scope wins**,
so the old single-project painter silently served every call: the board
kept showing the pre-rewrite waiting card, one harness case failed, and
the code I had just read (the new painter) was dead on arrival. The
debug cost several rounds of "the stub resolved, the flag was set, why
is nothing painted" before `grep -c 'function paintWorkspaceHistory'`
answered 2.

**Why:** the symptom contradicted the code as read, which pushes
debugging toward fetch chains, stubs, and state flags — anywhere but a
second definition of the same name. A syntax check passes; nothing
reddens; the old behavior just persists.

**How to apply:** when replacing a function by writing a new one, DELETE
the old definition in the same edit — or grep the file for duplicate
declarations the moment rewritten code "doesn't run". The harness's
per-case failure naming the observable (not a throw) is the shape this
takes.

Related: [[an-undefined-call-hides-in-a-branch-nothing-exercises]],
[[workspace-history-rail-plan-2026-10-30]].
