---
name: session-naming-ladder-six-rungs
description: "A session's name comes from a six-rung ladder in the owner's order — rename, goal, first deliverable requested, first plan executed, first message about the work, raw first message — and the subject rung exists because 42% of this archive's sessions open with a harness instruction that names no work at all."
type: change
date: 2026-11-18
basis: confirmed-by
tier: durable
shape: [rendering, resolved]
phrases: [sessions should pick up the name of the first deliverable that was requested or the first plan that was executed, title_ladder, first_plan, subject rung, names_no_subject, sessions-titles.json]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

The owner's 2026-11-18 naming ruling, implemented in `title_ladder` (src/gui/bridge.rs) with six rungs in this order: owner rename → standing goal → first deliverable requested → first plan executed → subject → raw first message.

Two reversals and one addition:
- The plan rung was `last_plan` (took `.next_back()`, with a comment arguing "named by where it landed"); it is now `first_plan` (`.next()`).
- The deliverable rung is new: `desk::first_by_session(home)` reads every desk once per panel into a session→name map, taking the OLDEST bound deliverable by `created` (not `updated`, not slug).
- The subject rung is new: `SessionSummary.subject` is the first owner message that names the work rather than the machinery.

Why the subject rung exists: measured across ~/.fleet/sessions, 42% of owner sessions (67/161) open with a harness instruction ("Reply with exactly the single word: done", "Call the subagent tool exactly once…"). The ladder's old fallback was the raw first message, so those rows were named by a smoke test the owner could not recognise. The worst case was a real conversation that OPENED with a probe: 20261116T130741Z-51e3fb87, 20 owner turns of real work, titled by its probe instruction for days.

`session_recall::names_no_subject` (narrow, two guards): the first line must open with a directive verb (call/use/run/reply/say/…) AND contain harness vocabulary (tool/subagent/bash/vision/single word/exactly once/…). A `?` always means it names something, so questions are never skipped; without the machinery word, "Use only this supplied corpus and cite the line numbers" keeps naming its row. `is_continuation` catches "continue"/"ok"/"go on", and `is_house_word` already caught "warmup". A KNOWN LIMIT is asserted in the test rather than papered over: a bare shell command with no tool word ("run pwd and then run ls") is not caught, because widening the verb list would start skipping real instructions — a row named by a command beats a row named by nothing.

Measured effect on the real archive: of 133 owner rows, 52 were rescued to a real subject, 77 still on the raw first message (mostly genuine single-message probes plus 7 rows with no owner message at all, which are honestly unnameable), 4 owner renames. The motivating session now reads "reviews and health are still having the same icons on the main rail" / rung `subject`.

The tooltip in app.js carries a `subject: 'the first message about the work'` word so a derived name says what named it.
