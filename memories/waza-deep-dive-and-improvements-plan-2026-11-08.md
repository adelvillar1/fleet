---
name: waza-deep-dive-and-improvements-plan-2026-11-08
description: "project 2026-11-08 — tw93/Waza read against fleet: the deep dive LANDED (docs/architecture/deep-dive-2026-11-08-waza.md, 424742b) and the five-area improvements plan is DRAFTED (docs/plans/2026-11-08-waza-improvements.md, uncommitted, awaiting approval)"
type: project
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

The owner's steer (2026-11-08): *"let's use the same pattern with this repo https://github.com/tw93/Waza and compare what exists in fleet and what is worth improving"* — the third comparison dive in the series (Anthropic memory, DoubleAI self-improvement, now Waza).

**Waza in one line:** tw93's engineering habits as eight small skills — think, ui, check, hunt, write, learn, read, health — each with an outcome contract, hard rules, and a gotchas table drawn from real failures; plus twenty rules (Need/Rules/Code/Debug/Review/Ship/Retro) written to paste into an agent's instructions, a resolver index, and a chaining model. It is a habits layer for general coding agents; no memory store, no face, no calibration stream.

**The deep dive's headline:** the correspondence is closer than expected — Waza's twenty rules land almost one-for-one on fleet's hard rules and machinery (rule 8 = the `.githooks/plan_check` + the gate's exit-code reading; rule 11 = the plan-worktree machinery; rule 16 = hard rule 7; rules 19/20 = the eval corpus and the failure-shape taxonomy + the clustering beat; "no unverified claims" = the "say only what you have read" ruling; the three-failed-attempts rule = the compaction fold). **Five improvement areas:** (1) the review skill's contract layer (the outcome contract, depth/risk classification, the finding quality gate, the status-first sign-off, the sibling sweep, the isolation rule — a skill edit, not code); (2) the face's statusline (context/budget/model over the state document the context-health plan already lands); (3) the review-rounds cap + the model-switch eval re-run; (4) the house skills' `references/` folders; (5) the chaining doc.

**The plan** (`docs/plans/2026-11-08-waza-improvements.md`, status draft, uncommitted): seven criteria C0–C6, five phases — the review contract (C0/C2) first, then the statusline (C1), the cron's model-switch eval re-run (C3), the references (C4), the chaining doc (C5), the gate (C6). arch-gate PASS (8 conventions gaps); plan-gate 11/11 covered with the Risks flag waived by disposition.

**Open for the owner:** the plan's approval; and the small-set tension Waza's "eight skills" bar poses against fleet's sixty-plus — a ruling, not the dive's call. Also noted: Waza's deliberate no-memory-skill decision is the inverse of fleet's biggest asset.

**Why:** the comparison series' value is the reverse-check — Waza's habits validated fleet's hard rules wholesale, and its formalized contract layer is the one thing fleet's scattered review discipline lacks.
**How to apply:** on approval, flip the plan active and land the phases; the review skill edit lands in BOTH `~/.agents/skills/review-changes/` and `~/.zcode/skills/review-changes/` in one commit (the drift risk). Related: [[self-improvement-program-merged-and-delivered-2026-11-07]], [[house-build-loop-sticky]], [[plan-gate-structure-and-association-traps]].
