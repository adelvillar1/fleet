---
name: c20-closed-by-ruling-owner-executed
description: "owner ruling 2026-11-02 — \"C20 is my manual task, so it's been overtaken by events. Close it so that we can complete the plan\": the beta gate (pass three) is the owner's own use, and closing it by ruling — not by a recorded pass — is the correct close; C22 then closed on the empty queue"
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

The closeout plan's C20 (the shakeout's B5, "the beta gate closes on evidence")
was still open when the owner said: *"C20 is my manual task, so it's been
overtaken by events. Close it so that we can complete the plan."*

**What was recorded (2026-11-02):**
- **C20 closed by ruling, not by a recorded pass.** The criterion's substance
  — that the owner's own use is clean — is exactly what no test can stand in
  for; only its recorder moves from the plan to the owner. The plan's row says
  so, and the close is the owner's, not a fabricated pass.
- **C22 closed on the empty queue.** Every row of the residuals table is
  ticked in its owner's status pass or carries a one-line reason with the
  ruling cited; `scripts/plan_check.py` reports "every completed plan's
  acceptance criteria are ticked".
- The plan was complete at `42b5837`, with C18 proven live the same push
  ([[composed-joins-the-judged-set]]).

**Why:** a criterion whose evidence is the owner's own use is not a test to
write — it is a gate to hand over. Recording a pass would manufacture the
evidence the gate exists to gather ([[live-behaviour-belongs-to-the-product-not-the-harness]]);
recording nothing leaves the plan open on something no session can close. The
ruling is the third option, and the owner named it.

**How to apply:**
- When a criterion is owner-executed, close it **by ruling** with the reason
  in the row — never by a harness pass, never by leaving it open as a
  permanent blocker.
- The closing rule that depends on it (C22) closes in the same turn, on the
  empty queue, not on the missing pass.
- The same shape as O1's amendment ([[a-ruling-has-to-name-what-the-check-can-see]]):
  the check the criterion names must be one the named actor can actually
  perform.

Related: [[open-work-closeout-2026-11-02]],
[[live-behaviour-belongs-to-the-product-not-the-harness]],
[[right-fixes-not-easy-fixes-beyond-the-plan]].
