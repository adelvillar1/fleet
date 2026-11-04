---
name: a-misplaced-case-case-falls-through
description: "feedback 2026-10-29 — a new `case 'x':` inserted immediately after a label in a switch that relied on fall-through (case 'finished': had no body of its own) made every finished run execute the new case's body; the symptom was eight unrelated checks going red, far from the edit"
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

2026-10-29, adding the onboarding's inline door: the reducer's event
switch had `case 'finished':` sharing a body with the cases after it
(a deliberate fall-through). My new block was inserted **immediately
after that label**, so `case 'needs_setup':` plus its body became part
of finished runs — every run that finished hid the log, popped the
welcome, and re-read onboarding. Eight receipt and `what-the-model-saw`
checks went red, none of them naming the edit.

**Why:** a `case` label with no body of its own is invisible at a glance
in a long switch; inserting after the label reads as "inserting a new
case beside it" while actually changing which cases share the body. And
the damage surfaces in distant tests because the fall-through now runs
code that was never written for it.

**How to apply:** before inserting a case, find the CASE'S OWN BODY —
the first non-label line below it — and insert either above the
neighbouring label or after the end of the body, never between two
labels. When a cluster of unrelated tests reddens right after a
one-function edit, diff the switch's case order first: a mis-placed case
is a one-line cause for eight reds.

**Second form, 2026-10-31 (the design canvas): a SHADOWED case is worse
than a mis-placed one — it is silent.** Wanting to record the board's
chat turns when a run finished, I added `case 'message':` and
`case 'finished': case 'failed':` near the top of the switch. JS takes the
FIRST matching label, so the existing handlers further down never ran:
live messages and finish events went dead, no test reddened (the harness
drives the finish event and reads its own consequences, not the
conversation's), and the only symptom would have been cards that stopped
appearing. The fix: never add a label that already exists — read the whole
switch for the event name first, then hook **inside** the existing case's
body (one line: `if (face.designPending && event.text) …` in `message`;
`if (face.designPending) flushDesignReply(…)` in the shared
`finished|failed` body). Grep `case '<event>'` before inserting one, and
prefer an in-body hook to a new label every time.
