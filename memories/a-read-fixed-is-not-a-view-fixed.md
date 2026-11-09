---
name: a-read-fixed-is-not-a-view-fixed
description: "feedback 2026-10-22 — the sessions \"open\" fix anchored the face's state READ to the opened session but left /api/run and /api/plan posting session-less, so a follow-up ran in the launch session; when a view is anchored to an object, every write it issues must name that object too, and the owner finding the twin is the second-instance signal"
type: feedback
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 11
superseded-by:
valid-until:
---

The owner's report was precise and diagnostic: "the open button is not
taking me to the chat experience, it may be loading it in memory but
doesn't let me interaction in the conversation." *Loads in memory* = the
read works; *won't let me interact* = the write does not. The composer
posted `/api/run` and `/api/plan` with no session, so the run was
dispatched against the bridge's *launch* session — a different
conversation, or a refusal ("a run is already in flight") when that one
was mid-run, which is exactly what "will not let me work in it" feels
like.

The previous session had already fixed the same anchoring on the read
side: the state read now names the session the face is showing, and the
launch itself deliberately stays the face's own session (a phone that
names nothing still gets what the face booted with — an existing test
pins that isolation, and it rightly refused the first attempt). The
write beside it was left naming the default.

**Why:** anchoring a view to an object is not one fix, it is two — every
direction the view speaks (reads *and* writes) must carry the anchor. I
fixed one direction, declared the feature done, and the owner found the
twin within a session. And "it loads but won't let me interact" is a
reliable map: the data path works, the command path does not.

**How to apply:** when a surface becomes anchored to a selected object,
audit every call it makes in both directions before calling it done —
grep the fetch/post sites for the ones still using the default. When the
owner reports half a feature working, name which half they described
before theorizing about the mechanism. Related:
[[a-predicate-naming-shapes-that-never-occur]] (the same feature's read
side, and the isolation test that guards the launch session),
[[the-check-and-the-work-must-be-in-the-same-place]] (a fix away from
the work it fixes),
[[verify-face-changes-in-the-browser]] (the owner's report was the only
witness).
