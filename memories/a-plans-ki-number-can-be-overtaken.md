---
name: a-plans-ki-number-can-be-overtaken
description: "feedback 2026-10-29 — a plan's pre-named KI number can be consumed by another row before close-out; read the ledger's tail for the next free number at filing time"
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

The resume-record plan (drafted 2026-10-28) named its defect "KI-354",
but by close-out the ledger's KI-354 was the sem1 llama-server instance
and the tail had reached KI-364 — the resume defect filed as KI-365
with a correction note in the plan itself.

**Why:** plan files pin a number at draft time; the ledger moves under
them. Filing against the memorized number silently collides with a row
that now exists, and the checker's cell-count pass will not flag a
wrong-but-well-formed ID.

**How to apply:** before appending a KI row, read the ledger's tail for
the actual next free number (`grep -o "KI-[0-9]*" | tail`), and when a
plan's named number was overtaken, record the correction in the plan
text in the same write. See [[ki-ledger-second-pass-method]] and
[[ledger-rows-lag-their-fixes]].
