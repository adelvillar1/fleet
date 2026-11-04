---
name: a-deferred-action-is-an-error-to-the-owner
description: "owner ruling 2026-11-03 — a composer action that applies only later ("the next run", "the next model call") is the equivalent of an error in the owner's eyes; every composer control must take effect when the owner submits it, or say so where it stands"
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

The owner, after reporting that **stop does nothing** while a run is
ongoing: *"let's check all of the other composer actions, because they
need to apply when the user submits them. delayed action is the
equivalent of an error in the eyes of users"*.

**Why:** the owner reads a control that does not act now as a broken
control — not as a control with a documented scope. Fleet's composer is
dense with controls whose effect is deferred, and most of them do not
say so.

**How to apply:**
- Audit a composer control by asking what happens at the moment of
  submit. If the answer is "at the next model call" / "at the next turn
  boundary" / "on the next run", that is a deferred action and it must
  either be made immediate or carry its scope in the control itself.
- **The folder already does this right** — its comment says *"a run in
  flight already accepted the folder it is working in"*. Copy that
  pattern; do not leave the scope implicit.
- A control that is deferred *and* blocked behind a pending approval is
  the worst case: the owner has no way to act at all. See
  [[the-approval-wait-blocks-every-run-control]] for the mechanism.

Related: [[an-effect-with-no-visible-trace-reads-as-broken]] (the sibling
lesson — an invisible effect reads as broken),
[[live-behaviour-belongs-to-the-product-not-the-harness]] (the owner finds
these by using the app).
