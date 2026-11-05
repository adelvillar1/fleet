---
name: fleet-anti-hermes-constitution
description: "The owner's named anti-goal for fleet (2026-10-23, restated 2026-11-01) — never become Hermes Agent, a harness they enjoyed deeply but that became so convoluted and difficult to configure they stopped using it; the guards are structural, the standing test per addition is "does the owner need to know this knob exists?", and a surface is judged by whether the TASK finishes on it"
type: feedback
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

Owner statement 2026-10-23, closing the lineage conversation: **"one
thing I didn't want it to become was Hermes Agent, something that I
enjoyed deeply but that became so convoluted and difficult to configure
that I ended up not wanting to use any more."** This is complexity death
by configuration entropy — the default ending for extensible systems:
every capability adds a knob, every knob needs a doc, every doc ages,
and one day the configuration surface IS the product. The owner did not
leave Hermes because it broke; the cost of telling it what they wanted
exceeded the value of what it did.

**Restated 2026-11-01, sharper, from inside fleet:** *"I built this
harness, and I know functionality and settings, but the settings
experience feels like a wall, even to me, imagine users that have never
interacted with the harness. that's what drove me away from hermes
agent to begin with, and that's why I built this harness."* The
constitution is not about other systems — the owner's own surface is
already the wall when a screen shows the inventory before the task.
That makes every settings decision a test of this memory, and the
settings bar became explicit the same day: *"more like zcode, deepseek
harness, less like hermes."*

**Why:** fleet is at exactly the age where this death begins — 45+
modules, 57 plans, a settings board with a dozen cards — and entropy
accumulates one reasonable feature at a time, the way it did in Hermes,
which was also once a clean tool its owner loved deeply.

**How to apply:** the guards are structural in fleet, not aspirational —
zero-config first run, one settings writer (validate before rename; a
refused write never lands half of itself), the roster → render → doctor
pattern (one declarative file, not a dotfile scavenger hunt), "no form
asks for a value fleet already has"
([[no-form-asks-for-a-value-fleet-already-has]]), and the doctor
naming what is wrong instead of hiding it in logs. Apply the standing
test to every addition: **does the owner need to know this knob
exists?** If the answer is "only sometimes, and fleet can infer it" —
infer it, don't expose it. Add the 2026-11-01 test for surfaces: **does
the TASK finish on this surface?** A chained setting that sends the
owner to three screens by memory has already failed it, however clean
each row is ([[a-chained-setting-completes-on-one-surface]]). Also:
Hermes' skills still live in ~/.hermes/skills and still contribute —
absorb the good ideas, never the configuration surface. Related:
[[fleet-origin-lineage]], [[zcode-vs-hermes-prompt-comparison]],
[[the-surface-keeps-the-machinery-and-hides-the-answer]].
