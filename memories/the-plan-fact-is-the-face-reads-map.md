---
name: the-plan-fact-is-the-face-reads-map
description: "project 2026-11-08 — the Learning pane reads the lesson plan by parsing the `lesson-plan` fact's own body (the teaching-order line and the Mermaid DAG), not a parallel store; the writer's format is the contract the face parses"
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

The Learning pane's lesson section (the coarse view, the position, what the learner holds) is read from the `lesson-plan` fact the lesson wrote — `plan_shape()` parses the body's `teaching order: a → b → c` line and the Mermaid DAG's node labels and edges, and the check facts' `held: true` lines mark the steps the learner has held. There is no second store and no re-derivation: the plan the learner saw IS the map the pane shows.

**Why it matters:** the writer's body format is now a contract a second consumer parses. A change to `record_plan`'s body (the arrow character, the `graph TD` header, the label quoting) silently breaks the pane's map while every test stays green — the tests pin the writer's output and the parser's reading separately, never the pair against each other end to end.

**How to apply:** when the plan's body format changes, change `plan_shape` with it and pin the pair (a fixture fact's body through the parser). The same shape applies to any fact body a surface reads back — the `check` facts' `held:` line is the pane's step state, and the `practice` facts' `held:` line is the outcome's. Related: [[learning-loop-implemented-2026-11-08]], [[a-fixture-can-match-the-face-and-miss-the-bridge]] (a hand-shaped fixture matching one side).
