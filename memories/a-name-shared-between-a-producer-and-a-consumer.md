---
name: a-name-shared-between-a-producer-and-a-consumer
description: "feedback 2026-10-26 — when one side writes a name and another side reads it, tests that pin each side's own fixture agree with themselves and disagree with each other, so nothing reddens; one shared constant plus a pin that the name the reader spawns is a name the repo ships"
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

A name that exists in two places — written by one side, read by the
other — is a contract with no test on the contract itself. Fleet's
LayerFS install copied `layerfs-register.mjs` into the home while the
driver looked for `register.mjs` there; every unit test on either side
passed, because each test wrote its own fixture under its own name. The
only thing that failed was the real one: a clean-environment delegation
named "the register script is not at …register.mjs" and fell back to
running without the substrate. The same shape recurred minutes later in
the probe, which validated a worker endpoint it never exported.

**Why:** each side's test is locally true — the installer copies what it
lists, the driver finds what its fixture placed — so the suite is green
at every level while the integration is broken by a string. This is the
sibling of [[live-and-reconstructed-declarations-must-agree]], with the
difference that decides the lesson: there, adding to one path reds the
byte-equality test; here, nothing reads both sides, so NOTHING reddens.
A shared literal is not shared state; it is two literals that happen to
match today.

**How to apply:** when a string crosses a boundary (a filename, an env
name, a route, a key), make it ONE constant both sides import, and add a
pin that asserts the two ends name the same thing — for a script, that
the name the driver spawns is a file the repository actually ships
(`the_script_names_are_the_names_the_repository_ships`). Grep for the
literal before trusting either side's test, and treat "each side has its
own test" as the signal that the contract between them has none.
