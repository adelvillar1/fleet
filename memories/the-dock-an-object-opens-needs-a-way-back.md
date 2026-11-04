---
name: the-dock-an-object-opens-needs-a-way-back
description: "feedback 2026-10-20 — clicking a card at desk width opened the right dock and the object's "clear" only hid the object, leaving a column the owner never asked for; an inspector that opens itself must close itself, and remember whether the owner had it open already"
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

**The report, verbatim:** "when you click on a card the right side panel shows
as it should, but there is no way to collapse it." Owner, 2026-10-20, after the
memory/skills tables shipped ([[fleet-memory-and-skills-knowledge-tables]]).

**What was wrong.** At `window.innerWidth > 1180`, `openDrawer` calls
`setAside(true)` and shows `#dock-object`. The object's button was labelled
**clear** and only set `dockObject.hidden = true` — the run card, session card
and trust card stayed, so the dock column remained a thing the owner never
asked for. The only way back was a small toggle in the topbar, which is not
where the owner is looking when a panel appears.

**The fix (`6fdb0c1`).** Remember whether the dock was already open when an
object opened it (`face.asideBeforeObject`); closing the object collapses the
dock only in that case, because a dock the owner had open is theirs, not the
object's. The object's button reads **close**, the run card carries its own
**collapse** control, and a hand-clicked dock toggle clears the flag so a later
close does not fold it. Escape closes the object and, when it was what opened
the dock, the dock with it.

**How to apply:** an inspector/inspector-like panel that opens itself as a side
effect of a click must offer its own close where it appears, and must restore
the layout state it found — "the panel is gone" and "the layout is back" are
different promises. Verify by scripting the full cycle (closed → click → close →
closed) rather than by checking the panel rendered. Related:
[[gui-revamp-browser-face-decision]], [[verify-what-renders-not-what-commits]].
