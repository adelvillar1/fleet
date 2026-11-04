---
name: a-plan-marked-met-can-hide-missing-behaviour
description: "feedback 2026-10-29 — I checked two onboarding criteria met while the behaviour did not exist (no proof element on the cards, the strategy action still jumped to settings); the completion audit caught it, both landed the same day, and the plan's record now carries the correction rather than a quiet green"
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

2026-10-29, the onboarding slice: `docs/plans/2026-10-29-guided-onboarding.md`
went to `completed` with all fourteen criteria checked and the gate
genuinely green (fmt/clippy/build/test 0, harness 365/40, the door
verified live in Chromium). The **completion verifier then refused it**:
criterion 6 required the capability cards to appear "each with a visible
proof" and `paintOnboardCards` rendered no proof element at all;
criterion 12 required the roster step to "end in the confirmation
preview naming the window fallback", and `onboardStep('roster')` simply
did `window.location.hash = '#/settings'`.

**Why it happened:** the criteria were written in the plan's prose and
the prose was richer than the build — I read "each with a visible proof"
as satisfied by the proof *ladder* section describing presence checks,
and read "ends in a confirmation preview" as satisfied by the disclosure
paragraph naming the floor. Each criterion had been checked against the
plan's own approximation of itself instead of against the element in the
face.

**How to apply:**
- **Check a criterion against the artifact, never against the plan's
  restatement of it.** For every UI criterion, name the element that
  proves it (a class, an id, a rendered string) and confirm it exists in
  the built code — if you cannot point at the element, it is not met.
- **A criterion naming a surface element needs a harness check in the
  same commit.** Both gaps were invisible precisely because no check
  asserted them; both were closed by adding one (`OB every card carries
  a visible proof`, `PV the strategy step shows its preview`) alongside
  the code.
- **When it is caught, correct the record in the plan, not just the
  code.** The plan's criteria now say what landed, and an "Audited after
  the build — two criteria were marked met before they were" section
  names the miss. A quietly re-greened plan destroys the audit's value.
- The same audit found a real bug the green had hidden: the
  discovered-key press posted a *key write* with a placeholder value;
  it is now a `prove` op that touches nothing and proves with one
  bounded call.

Related: [[a-green-gate-covers-what-was-built-not-what-was-left-out]] (a
green gate reports only on the tests that exist),
[[guided-onboarding-implemented-e838241]],
[[the-screenshot-is-the-visual-review]].
