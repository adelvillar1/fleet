---
name: ledger-rows-lag-their-fixes
description: "feedback 2026-10-24 — KI-190's designed fix landed 2026-10-15 (e1796da, 412 commits before its row closed) and the row never closed; my own audit note (\"the blocker is lifted, the fix is ordinary work now\") was itself stale — verify rows against commit history, including the audit's own conclusions"
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

The unlocked-items session opened with KI-190 ("apply the handed-over fix
once the sibling's eval.rs round lands") — and the fix had been in the tree
for nine days: `e1796da` (2026-10-15, "the eval fit leg is designed, not
accidental") already carried the content-keyed lane entries, the
walked-confidence pin, and the beat-owned third-vector cell. 412 commits
later the row still read open, and my same-day audit note ("the blocker is
lifted — applying the designed fix is ordinary work now") verified the
blocker but never checked whether the WORK had also landed. Two staleness
layers deep before anyone read the scenario.

**Why:** a ledger row is a write-time snapshot exactly like a recap, and an
audit note ABOUT a row is a snapshot of a snapshot — each layer can lag the
code independently. The triage pass had already named this shape
("a KI row's next-action can be STALE", KI-017) and it recurred anyway,
this time in my own writing.

**How to apply:** when a row's next-action says "do X once Y lands", grep
for X itself before planning Y — `git log -S` on the fix's signature phrase
finds it in one call. And never report a row as "unblocked/actionable"
without reading the code it points at: the blocker lifting and the work
landing are two separate facts, and each needs its own evidence. Related:
[[a-ledger-row-can-name-the-wrong-cause]], [[fleet-ki-triage-pass-one]].
