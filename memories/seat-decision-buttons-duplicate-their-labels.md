---
name: seat-decision-buttons-duplicate-their-labels
description: "project 2026-11-06, CLOSED 2026-11-10 — the approval seat's decision buttons rendered their label TWICE (\"Yes1 Yes\"); FIXED at 141fe72, MERGED (eb57d8f, KI-467), and the plan file now reads status: completed — C3 verified (both render sites build with no text; `cargo fmt --all --check && cargo build --release` exit 0)"
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

The owner reported (2026-11-06) that "the permissions request card is
duplicating the text of the options". The cause was read in primary source:

- Both seat renderers built each decision button with
  `node('button', 'decision', pair[2])` — and `node` sets `textContent`
  (`src/gui/assets/app.js:1329-1334`) — then appended the key span and
  `document.createTextNode(' ' + pair[2])`, the label again. The button
  read `<label><key> <label>` — e.g. `"Yes1 Yes"`. Reproduced with a
  standalone render: `before: "Yes1 Yes"` / `after: "1 Yes"`.
- **Commit `2ffcd79` (gui-revamp C12) introduced it**: the diff changed
  `node('button','decision',pair[1])` (the key) to `pair[2]` (the label)
  and added the key span plus the label text — leaving the `node` text in
  place. The intent was clearly key badge + label.
- The harness's C12 case asserted only that the buttons' text *includes*
  the key digits, so the duplication passed green — the pin was the gap.

**FIXED and MERGED** (2026-11-06, commit `141fe72`, carried through the
merge to origin/master at `eb57d8f`). `node('button', 'decision')` — no
text — in both renderers, so the content is the key span then the label,
which is what `.decision-key`'s `margin-right` in `app.css` already
intended. **KI-467 filed** (renumbered from KI-462 after the merge's
numbering collision with the burn-down's KI-462; 467 rows, 6 open, checker
green) and the plan
(`docs/plans/2026-11-06-seat-decision-label-duplication.md`) ticked whole.
The new pin asserts each button's **exact text** —
`'1 Yes'` / `'2 Yes, and the rest of this run'` / `'3 No'` — so a
regression reds. The **served face carries the fix** (verified: 2 ×
`node('button', 'decision')`, 0 × the old call in the served app.js).

**CLOSED 2026-11-10 (the board pass).** C3 "the gate and the harness run
green" verified with evidence rather than assumption: both render sites
already build the button with no text (`node('button', 'decision')` at
src/gui/assets/app.js:3530 and :3578), and C3's own command exits 0. The
plan's `status: active` was stale — a claimed fix whose plan box had never
been ticked; when a plan's remaining box is a gate/harness criterion and
the fix site reads clean, tick it with the line numbers as the evidence.

**How to apply.** The class is a builder whose text argument and its
appended children overlap — `node(name, className, text)` sets
`textContent`, so any caller that then appends a text node duplicates it.
When a button's label looks doubled, read the builder's `node` helper
before the renderer. Same family as
[[a-repaint-that-reassigns-under-the-handlers-orphans-the-clicks]] and
[[the-queue-rows-rebuild-under-the-cursor]]. Related:
[[the-approval-wait-blocks-every-run-control]] and
[[ki-380-zombie-tab-2026-10-30]] (the invisible seat).
