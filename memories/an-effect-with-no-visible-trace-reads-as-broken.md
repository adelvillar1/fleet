---
name: an-effect-with-no-visible-trace-reads-as-broken
description: "feedback 2026-10-22/23 — the owner twice reported the approval card standing after a grant; the first fix landed the missing trace, the second was a REGRESSION caused by a reducer that never learned the live session. An invisible effect reads as broken, and a fix that depends on a value nobody updates regresses silently"
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

The owner's dogfooding report (2026-10-22): "the permissions popup
doesn't close after permissions are granted." Root-causing it took three
stacked mechanisms (the answer posted no session so the bridge looked
under the launch session; the card never visibly closed because a
multi-call tool wave opens the next call's seat the instant the previous
is answered — the card just swaps content in place; and the grant left
no trace anywhere). All fixed in 05e3e86, and the fix that satisfied the
report was not only closing the card but landing an acknowledgment row
in the transcript ("✓ allowed this call · bash · …"), the house log
principle applied to the owner's own decisions.

**The product lesson:** an action whose effect leaves no visible trace
is indistinguishable from a broken action. The failure mode is invisible
in the common case — a single-call turn closes and re-opens far enough
apart that nobody notices — and the owner found it by using the app.

**The second report, 2026-10-23, was a regression of that same fix, and
its mechanism is the more transferable lesson.** The face's event
reducer had **no `case 'session'`**, so `face.session` never tracked the
session a run creates; the answered check compared the seat's session
against that stale value and cleared only when the two happened to
coincide — 1-in-2 on a fresh face, always on a face whose last read
predated the run. Two changes fixed it: the reducer learns the live
session as its event arrives, and the clear keys on **the seat's own
session** rather than the loaded one. Logged as KI-295. The general
shape: a fix that gates an effect on a piece of state is only as reliable
as that state's upkeep — audit who *writes* the value a condition reads,
not just the condition.

**Why:** the same symptom returned a day after being "fixed", which
costs the owner's trust in every report that says a card closes. And
both times the owner's evidence was the app, never the code.

**The touch-face form (2026-10-28, the chat review).** The owner reported
the message controls "almost nonexistent": `.row-rewind` (edit, rewind)
carried `opacity: 0` until a hover, the owner's edit floated at the bubble's
shoulder where narrow screens clip it, and replies carried no control at
all — and the surface the owner reads is the PWA on a touch face, where
hover does not exist, so every affordance was invisible in its actual home.
The fix made controls always visible with a `.msg-tools` footer (edit +
copy on the owner's bubble, copy under replies) and the copy button's own
label flip is this same lesson applied to it: an effect with no visible
trace reads as broken. Pin the visibility in CSS (the `.row-rewind` rule
must hold no `opacity: 0`) and the effect in its payload (the clipboard
write), not only in prose.

**The 2026-11-03 form: an accepted control whose effect is deferred.**
The owner reported that **stop does nothing** during a run; the stop was
in fact written and queued correctly, but the run was blocked on an
approval seat and the agent loop reads its control file only before a
model call — so nothing happened until the owner answered the seat, and
the card's "stopping" was a promise the mechanism could not keep. The
same deferral covers steer, pause, fold, pin and resume
([[the-approval-wait-blocks-every-run-control]]). The owner's ruling for
the whole class: *"delayed action is the equivalent of an error in the
eyes of users"* ([[a-deferred-action-is-an-error-to-the-owner]]). So the
test is not only "does the effect leave a trace" but "does the effect
happen when the owner submits it".

**How to apply:** when the owner reports "X doesn't work", the first
question is what visible trace X's success would leave — a line, a
state change, a log row. If the answer is "nothing", the report is
about the missing trace as much as the mechanism. When a fixed symptom
returns, suspect the state its condition depends on before the
condition itself, and prefer keying the effect to the object's own
carried value (the seat's session) over an ambient one (the loaded
session). And before asking whether a control works, ask whether the
owner can SEE it where they read — hover-gated UI on a touch surface
is a control that does not exist. Related:
[[a-fallback-path-hides-the-defect]],
[[the-dock-an-object-opens-needs-a-way-back]],
[[a-predicate-naming-shapes-that-never-occur]] (the sibling: a check
against a vocabulary nobody writes),
[[chat-review-shipped-f191516]].
