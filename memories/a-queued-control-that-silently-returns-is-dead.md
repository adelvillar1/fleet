---
name: a-queued-control-that-silently-returns-is-dead
description: "feedback 2026-11-09 — four \"the button does nothing\" reports in one day, all a handler that returns early or navigates elsewhere with no visible trace; and the inverse, an op with no control that reaches it"
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

Feedback 2026-11-09 — the owner hit "the button does nothing" repeatedly in one
day, and told me so directly: *"pay very close attention to dead buttons, we
cannot have that."* Every instance was the same shape: **a control whose press
leaves no visible change**.

- the About pane's update press (KI-483/KI-487 — the refusal painted into the
  transcript, two panes away);
- the composer's Send with a message queued (KI-495 — the empty-composer branch
  returned; `stopRun` returned when the local process card was absent);
- the inbox's onboarding "set it up" for every step (KI-496 — it navigated to
  `#/chat`, the pane already on screen);
- and the **inverse**, which is just as bad: the `undismiss` op existed with
  **no control that reached it**, so three dismissed rows trapped the owner's
  inbox at zero until the op was curled by hand.

**Why:** an effect with no visible trace reads as broken
([[an-effect-with-no-visible-trace-reads-as-broken]]); the cheap version is a
guard right for the internal model and wrong for the owner's hand.

**How to apply:**
- When a press can decline, paint WHY where the press stands — the pane's own
  answer area, the chip, the button's title — never only in a transcript.
- **Every op the backend accepts must have a reachable control**, and every
  control must have a real consumer: grep for the verb's caller before
  calling a surface done (a persisted decision with no way back is a trap).
- Read the **live** surface after shipping, not just the harness: three of the
  four defects here were invisible to green tests and obvious in the rendered
  DOM/screenshot (a type name where a heading belonged, a row still in the
  queue, an inbox at zero).
- Pin the declining path in the face harness with a red-proof — the change
  stashed, the case must fail for the right reason.
- Prefer a guard that reads the face's belief (`face.running`) over a local
  cache (`face.process`) that can lag it.
- **A control that works but leaves no trace reads as broken too.** The owner
  dismissed the inbox rows "to try out if it worked" and the empty queue then
  looked like a self-dismissing bug; the press WAS the cause. After shipping an
  irreversible-looking action, name what it did where the owner is looking
  (a count, a toggle showing the hidden set, a line saying "3 dismissed").
- **A press that waits on a slow reply is dead for its first seconds.** A
  decision's reply carries a full read (the doctor probe among it), so the row
  stayed put for seconds and the button looked inert. Paint the outcome in the
  SAME tick (optimistically hide/show the row), say what is riding (`working —
  dismissing`), and let the reply reconcile — a refusal rolls the paint back
  and names itself in the pane. Red-proof it by removing the optimistic paint.
