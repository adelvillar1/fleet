---
name: learning-loop-first-plan-drafted-2026-11-08
description: "SUPERSEDED 2026-11-08 — the Learning feature's first plan was drafted as a first slice (the lesson host and the measure half), the owner challenged its scope and ruled for the whole feature in one plan; see [[learning-loop-plan-whole-feature-2026-11-08]] for the rewritten plan"
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

The owner's steer (2026-11-08), after the LearnHarness design landed: *"draft it"* — the first plan for the Learning feature. It is DRAFTED and GATED: `docs/plans/2026-11-08-learning-loop.md`, status draft, uncommitted; plan-gate PASS (10/10 covered, 5/5 in scope), arch-gate PASS (2 conventions gaps — the Learning surface and the prober role, offered as doc updates).

**The slice's shape (seven criteria, C0–C6):** C0 the mockup first (the Learning rail section and pane — the active lesson's coarse plan, the graph, the practice entry, the history — measured, read, owner-read before the pane's code); C1 the lesson session kind on the `Start` record (`src/session.rs`); C2 the prober (a home-side role under `<home>/roles/`) and the probe loop in a new `src/lesson.rs` (asks, captures each answer with thinking and confidence, brackets the current state, pins the goal); C3 the records ride the substrate (probe answers/checks as facts — the `probe` type — and calib rows — a `learn_probe` op; held propositions as facts — the `proposition` type — with the `premises` field validated through `Remember::parse`; the recall and the dream pass apply); C4 the feeds' first wiring (the confusions become demand classes; the capability gaps land through the improve store and the drafter; the recurring confusions feed the clustering beat's shape proposals); C5 the gate; C6 the docs (`docs/features/learning.md` new, the surfaces doc's rail inventory, the memory doc's proposition type).

**The owner's challenge, and the honest answer:** *"does the plan include as much functionality from the source project?"* — NO, and the answer was given plainly: the plan is the first slice, roughly the front third (the host + the measure half). Designed but deliberately phased after it: the plan (the minimal tree, the validated Mermaid DAG), the teaching loop (single steps, checked, repaired), the practice (unnamed retrieval, one problem per turn), the graph's full shape (the extraction pipeline, coverage gating, notes import, the graph view), the face pane's code (the mockup gates it). Not needed at all: the host-specific machinery (the Electron shell, the provider/subscription connections, the dictation, the updater, the statusline) — fleet's equivalents exist. The phasing follows the pedagogy: the probe's precision is the ceiling on the plan's quality, so measure first.

**The open options the owner has not chosen:** (a) keep the phasing (the measure slice lands, then the plan+teaching slice — the first full lesson end to end, then the graph/practice); (b) expand this plan to carry the FULL lesson loop (measure + plan + teaching in one plan — bigger, but the learner's first lesson runs end to end sooner); (c) a different split (the measure + the graph's extraction together, deferring the teaching). The assistant's recommendation: keep (a) — the teaching loop is where the theory's demands concentrate (single steps, checked, repaired, no meta-narration) and deserves its own plan's attention rather than being one phase of a big one.

**SUPERSEDED the same day:** the owner chose (b) in the strongest form — *"let's make the plan with the deferred sections included, I don't see the point of planning this way for work that coding agents do. work fast, fail fast, fix fast"* — and the plan was rewritten whole with every phase (see [[learning-loop-plan-whole-feature-2026-11-08]]). This entry survives as the record of the first-slice framing and the scope question that produced the ruling.

**Why:** the owner's greenfield framing ("learning is part of my work process", a first-class work-menu feature) plus the three rulings (project knowledge, self-improvement feedstock, capability feed) make this the feature's first plan; the scope question is the owner's to settle before the plan activates.
**How to apply:** the rewritten plan is the live one; this entry's scope question is answered. Related: [[learnharness-deep-dive-2026-11-08]], [[learning-is-project-knowledge-and-feedstock]], [[waza-deep-dive-and-improvements-plan-2026-11-08]].
