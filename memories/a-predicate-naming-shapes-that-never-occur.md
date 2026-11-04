---
name: a-predicate-naming-shapes-that-never-occur
description: "feedback 2026-10-22 — the sessions "open" fix (d4199f3) hid every resumed history behind the empty state because a predicate named event types that never occur; a check written against an imagined vocabulary hides everything silently, and a feature that works live but not on replay means the two paths' vocabularies have diverged"
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

Three defects stood between the sessions page's "open" and the owner's
spec (land in the conversation with the whole history, first message at
the top, last above the composer). Two are the generalizable ones:

1. **A check that names shapes the data never has hides everything,
   silently.** `replay()` decided "has anything been said?" by looking
   for events of type `owner`, `assistant` or `tool` — but the events
   both paths actually emit are `message` with a `role`. So every
   resumed history was judged "not talked" and the log stayed hidden
   behind the empty state. Only `send()` ever unhid the log, which is
   why a normal conversation looked perfect and the resume path looked
   empty: the common case was covered by a different code path.
2. **A feature that works live but not on replay means the two paths'
   vocabularies have diverged.** The live path streams the assistant's
   words as `token` events and never emits the message card, so the
   replay — which has no tokens — rendered the owner's questions with
   the answers missing between them. The house rule says replay and
   live are one function over one log; when the outputs differ, the
   event shapes have drifted apart, and the drift is invisible until
   someone resumes a session.

The third is a process note: the first fix (re-anchoring the bridge's
launch session to the resumed one, so a session-less state read would
return the history) was refused by an existing test pinning that a
phone naming nothing still gets the session the face booted with. The
test was right; the fix belonged on the face's read, which now names
the session it is showing. When a fix breaks an isolation test, read
what the test protects before working around it.

**Why:** both mechanisms produce a confident wrong story — the gate is
green, the code reads correctly, the log on disk holds everything —
and the only witness is the rendered page with a real session loaded.

**How to apply:** grep the shapes the data actually has before writing
a predicate over them, and when a path works live but not replayed,
diff the two paths' event vocabularies first. Related:
[[verify-face-changes-in-the-browser]] (the browser is the only
witness), [[the-check-and-the-work-must-be-in-the-same-place]] (a check
away from the work it guards).
