---
name: a-task-must-carry-what-the-model-would-have-to-hunt
description: "feedback 2026-10-31 — an "ask about the board" task made the design run browse the console with tools for minutes to count its own pages; the task now carries the board's inventory (each frame's name/id/size/model/pins/review) and the same ask answers in one line and hands back"
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

**2026-10-31 (design canvas T0).** The board's conversation composed a
one-line task — *"About the design board `sample-board`: how many pages are
on this board?"* — and the run spent minutes calling `browse` on
`127.0.0.1:3080`, reading the console's rail, opening `#/design`, trying to
count the frames through the UI. Its own document was on disk the whole
time; nobody had told it.

**The fix is the task's content, not the model's diligence.** The composed
task now opens with the board's inventory — one line per frame, name, id,
size, model, open pins, review state — plus the frame's pins and review
when the request names one. The same ask afterwards answered *"2 pages. —
Sample — Landing hero (`f-6e0cc6ec`, 1280×900) — Sample — Pricing…"* and
handed back, no tool call at all.

**Why it matters beyond this surface.** Every delegated run's context is a
prompt somebody composed. When the run's subject is a stored document the
harness owns, the harness knows the facts and the model does not — so a
task that describes the subject *in words* makes the model reconstruct
them with tools, which is slow, costly, and wrong when the UI doesn't
expose them. The composer is the cheapest place to put ground truth.

**How to apply:**
- Any task about a harness-owned object (a board, a project, a corpus)
  should carry that object's **inventory**, not just its name.
- Keep the inventory **bounded** — one line each, the fields a question
  would need (id, size, state), not the whole document.
- The tell that this is missing: a run whose first moves are
  `browse`/`read_file` on the app itself. A design/CRUD run should not
  need to look in a mirror.
- Composing in Rust (`canvas::designer_task`) is what made this one edit
  cover the composer, the CLI and any cron job at once — see
  [[design-canvas-complete-plan-2026-10-31]].

Related: [[a-kernel-is-not-the-product]].
