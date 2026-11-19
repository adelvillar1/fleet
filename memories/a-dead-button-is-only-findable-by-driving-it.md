---
name: a-dead-button-is-only-findable-by-driving-it
description: "A dead-button review has to drive the DOM and fingerprint every observable channel — document state, calls, localStorage and clipboard — because the broken buttons are broken by their seed or an early return, not a missing listener; the face harness could not see submit-button actions at all until its click learned to submit the owning form."
type: project
date: 2026-11-17
basis: inferred
tier: durable
shape: [rendering, drops-the-row, resolved]
phrases: [every visible button on every lens, fingerprint hidden class aria text calls localStorage clipboard, type=submit button submits its owning form, copy the fact name reverts its label on a timer, state-gated silence is not a dead button, SWEEP_STILL named silences]
missing: []
premises: []
recall-count: 19
superseded-by:
valid-until:
source:
---

The owner's dead-button review, 2026-11-17: "every single one tested against the action it's supposed to trigger."

**The static map is useless for this.** 310 `node('button'` sites, 440 click listeners, 73 `dataset.act` — a grep cannot tell a wired button from a dead one, because the broken ones are broken by what they were *seeded* from or by an early `return`, not by a missing listener. Six "unwired" hits from an AST-ish scan were all `type=submit` buttons wired through their form.

**The only method that works is driving the DOM.** Boot each lens, click every visible button, fingerprint the whole document (every element's `hidden`, class, `aria-expanded`/`aria-selected`/`aria-pressed`, text), plus fetch calls, localStorage, AND `clipboardWrites`. A press that moves none of them is dead.

Three traps, each of which produced false positives until fixed:
1. **The stub didn't know click→submit.** A real browser submits the owning form when a `type=submit` button is pressed; the harness's `El.fire` only bubbled the click. That made 20 real buttons (the onboarding door's "Wire it", rendered on all 20 lenses) look dead. Fixed in `El.fire`; pinned by "FORM pressing a submit button submits its form". Every pre-existing harness case fires `submit` by hand, which is exactly why none caught it.
2. **A press whose effect is a revert is invisible.** `copy the fact name` writes the clipboard then reverts its own label on a `setTimeout` the stub fires immediately — it looked dead until the fingerprint counted clipboard writes.
3. **State-gated silence is correct.** `fit` on a board already at the fitted scale, re-pressing the current inbox tab, `designFit` with no bounds. These are named in `SWEEP_STILL` with reasons, never waived generically.

**Result of the sweep:** 373 buttons across 20 lenses → 0 dead; settings cards 27 cards/40 presses → 5 dead; memory fact cards 20 presses → 0 dead.

Two real defects found and fixed (KI-563): the tier card's last-secondary `↓` looked live but returned silently — now `disabled` with the reason in its title; and `add secondary` returned on an empty/already-present picker — the picker now offers only what the walk doesn't carry (`syncAddable`), so the press cannot aim at a value it would ignore.

KI-564 records the harness gap itself. The permanent regression net is two cases: `everyButtonOnEveryLensDoesSomething` (drives 373 buttons, asserts the count so a sweep that drives nothing fails) and `pressingASubmitButtonSubmitsItsForm`.
