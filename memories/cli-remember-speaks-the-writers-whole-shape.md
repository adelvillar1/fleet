---
name: cli-remember-speaks-the-writers-whole-shape
description: "fleet remember accepts --phrases and --missing so the CLI writes the direction channel's two inputs, not a subset that leaves the ranking's largest term dark on every fact the surface writes."
type: project
date: 2026-10-15
basis: inferred
tier: durable
shape: [change]
phrases: [the trajectory channel reads phrases and it is the largest term at 0.30; the CLI spoke a subset and every fact it wrote carried no direction; the writer validat…]
missing: []
recall-count: 3
superseded-by:
valid-until:
---

Found 2026-10-15 while closing the recall loop: the write path had accepted phrases and missing since slice 19, but the CLI's remember surface exposed neither, so every fact written from the terminal carried an empty phrases list and scored trajectory 0.00 - the direction channel the owner's ruling made the largest single term was dark on the whole CLI-written corpus. The fix adds the two flags, split on commas the way --shape is, with the writer's own validation (six items, 160 characters each) refusing a malformed list rather than the CLI truncating silently. The lesson generalizes: a surface that speaks a subset of a writer's shape does not merely omit a field, it silently biases every record that passes through it.
