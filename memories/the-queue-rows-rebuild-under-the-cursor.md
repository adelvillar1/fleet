---
name: the-queue-rows-rebuild-under-the-cursor
description: "project 2026-11-03 — the queued-message rows' send/edit/drop buttons did nothing because paintQueue() rebuilt every row from textContent='' on each call and paint() called it on every event, so during a run the row under the cursor was destroyed and recreated mid-click; FIXED in F5 (b2a4f9f) with paintSeat's key pattern (face.queueKey), pinned by harness case CA5"
type: project
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

Reported by the owner on 2026-11-03: *"a queued message doesn't respond to a
send, edit, or delete on the gui, nothing happens when you click on those
buttons."*

**The mechanism.** `paintQueue()` (`src/gui/assets/app.js`) rebuilt every
row from scratch — `el.queueList.textContent = ''` then
`items.forEach(...)` — and `paint()` called it. `paint()` runs at the
**end of every event** (`apply(event)` → `advanceProcess(event)` →
`paint()`). During a run events stream in continuously (tokens, progress,
heartbeats), so the row under the cursor was destroyed and recreated many
times a second. A click is a mousedown and a mouseup; if a rebuild lands
between them the mouseup hits a **different element** and no click event
ever fires.

The three verbs were wired correctly (send-first → `moveQueued`, edit →
splice + composer, drop → splice). The handlers were fine; they were
orphaned by the repaint. This is the same class as KI-394, which this
repo has been bitten by before.

## FIXED — F5 of the composer-actions plan (b2a4f9f)

The key pattern `paintSeat()` already uses:

```js
var key = items.join('\u0000');
if (key === face.queueKey) return;
face.queueKey = key;
```

with `face.queueKey = ''` reset on the empty branch, so a re-queue
rebuilds. The chip count and note still repaint every event; only the
rows are guarded. Pinned by harness case **CA5**
(`theQueueButtonsSurviveARepaint`): queue two messages while a run is in
flight, drive heartbeats/turns through the reducer, then assert edit
returns the words to the composer, drop removes the row, and send moves a
row to the head.

**It compounded with F2.** While reproducing the queue I could not get a
message to queue at all: the face's state pill read **"idle"** and the log
was empty, while `/api/state` for that same session reported `running:
true`, a standing `approval`, and 113 events. The root cause was found
later and is **not the replay path** — the harness proves `replay()`
paints both (CA2). It was `/api/resume` **refusing a session with a run
in flight** and `openWork` treating that refusal as "don't load". See
[[the-approval-wait-blocks-every-run-control]] and
[[composer-actions-plan-drafted-2026-11-03]] (F2).

**So the queue had two ways to look dead:** the rows rebuilt under the
cursor, and after a reload the composer did not know to queue. Both are
fixed.

**The audit's full shape** (the owner asked to check every composer
action): the in-flight controls are **six** (stop, steer, pause, fold,
pin, resume — all one control file, all deferred while a seat waited —
now interruptible, see
[[the-approval-wait-blocks-every-run-control]]); the run-scoped choices
(mode, provider pin, plan mode, turn budget, goal) apply to the **next**
run and now say so (F3); `rewind` refuses by name during a run (the
honest shape); and `/compact`'s description now names the turn boundary
(F4).

Related: [[a-deferred-action-is-an-error-to-the-owner]] (the ruling this
was found under), [[a-repaint-that-reassigns-under-the-handlers-orphans-the-clicks]]
(the sibling lesson), [[gui-client-verified-by-a-vm-harness]] (how to pin it).
