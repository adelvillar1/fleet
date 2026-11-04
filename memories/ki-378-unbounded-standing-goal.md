---
name: ki-378-unbounded-standing-goal
description: "KI-378 — the standing goal was unbounded (a 10k paste stored verbatim where it rides every run's prompt); bounded at the writer (2,000 chars) and pinned"
type: project
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

Fleet's standing goal had no bound: `POST /api/goal` accepted 10,000
characters and stored them in `sessions-goals.json` verbatim, and the
goal rides **every following run's system prompt** — a paste gone wrong
into the `+goal` chip was a silent per-run cost until someone re-set it
(the face's displays clip it to 42 chars, so the size was invisible
while the cost was real).

**Why:** the writer (`bridge::set_goal`) trimmed and filtered but bounded
nothing, and the composer's task field is the same class at one-shot
scale. A bound at the writer is the house's own rule — the writer
refuses what it cannot serve honestly.

**How to apply:** `Bridge::GOAL_MAX_CHARS` (2,000 chars) now refuses
past the ceiling, naming the bound and why in the refusal; pinned by
`the_goal_refuses_past_its_ceiling` (a 2,001-char goal refuses, a
2,000-char one lands). The task-field send bound is the owner's triage
(KI-378's row names it). Every free-text field that rides a prompt or
spends model budget needs a named ceiling at its writer — grep for
`set_` and `file_from_tool` writers before assuming one exists.
