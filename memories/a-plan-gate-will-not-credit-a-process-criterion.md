---
name: a-plan-gate-will-not-credit-a-process-criterion
description: "feedback 2026-11-07/08 — the plan-gate's coverage model does not credit a criterion whose body step names it in prose, so every plan's gate-and-docs criteria come back MISSING and the house's accepted plans carry the same flags with a recorded waiver rather than a reword"
type: feedback
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

Across five consecutive plans (owner-inbox, component-catalog, and the
hybrid-retrieval drafts), `dev-decisions plan-gate` flagged the **gate**
criterion ("the four-leg gate runs clean") and the **docs** criterion as
MISSING even though the plan's Approach names both by step ("step 6: the
four-leg gate, once, at the end"; "step 7: the feature doc and the tech
doc's module reference"). The flags are stable across runs (p≈0.15–0.36) and
the accepted `2026-11-07-owner-inbox.md` carries the identical C8/C9 MISSING
flags with a recorded waiver. The coverage model does not credit a criterion
whose body step names it in prose — it wants the criterion's assertion in a
section the mapper associates with it, not a step that merely says the words.
The hybrid-retrieval plan was the fifth, and its waiver was recorded the same
way before the work started — the pattern is now the house's default answer,
not a per-plan judgment.

**Why:** a gate flag that is systematic is not a defect in the plan — it is a
limit of the tool. Chasing it by rewording the same sentence burns a turn and
still comes back MISSING, and the accepted plans show the house's answer is a
disposition, not a rewrite. The useful signal the gate DOES give is the
reverse: a criterion it flags that the body genuinely never commits to is a
real gap.

**How to apply:** when plan-gate flags a process criterion (the gate, the
docs, a pre-condition like the mockup) that the Approach names by step, record
a waiver disposition (`dev-decisions disposition plan-gate <plan>
--status waived --reason "…"`) and move on — do not reword the criterion
chasing coverage. The real work is the body's commitment, which the plan's own
verification section states. Related:
[[plan-gate-structure-and-association-traps]] (the structural traps — a
literal `## Approach`, command-led criteria, the section cap, and the
criterion-by-id association rule), [[a-criteria-cap-makes-qa-green-by-omission]]
(a cap you cannot see is a cap you have not measured).
