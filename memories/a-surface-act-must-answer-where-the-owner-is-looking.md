---
name: a-surface-act-must-answer-where-the-owner-is-looking
description: "A surface act's confirmation must land where the owner is looking — notice() wrote into #log inside panel-chat, hidden on every other lens, so 172 call sites answered invisibly and two reported buttons looked dead while working; it now routes to a #face-say strip when the transcript is not visible."
type: project
date: 2026-11-17
basis: inferred
tier: durable
shape: [rendering, drops-the-row, resolved]
phrases: ["notice appends to face.stream inside #log inside panel-chat", transcriptVisible not the lens name, face-say strip outside every panel, three cases were pinning the silence, stub setTimeout ran callbacks synchronously, second-brain endpoints unmapped in the mock]
missing: []
premises: []
recall-count: 63
superseded-by:
valid-until:
source:
---

The owner's two reports — "under intelligence there is a card that says needs you but the button does nothing" and "under health the button to refresh the second brain does nothing" — were **the same defect twice, and neither button was unwired.**

**The root cause.** `notice()` appends to `face.stream`, which lives inside `#log`, which lives inside `<main id="panel-chat">`. On every lens other than the conversation, that panel is hidden — so a press really did its work and then said so in an element the owner did not have on screen. Proof: `POST /api/memories/consolidate` against the live face returns 200; in the harness the same press appended "the pass landed" to `#log` while `#log.hidden === true` and the memory board's text did not change. **172 of the 227 `notice()` call sites** are on non-chat paths.

**The fix.** `notice` routes on `transcriptVisible()` — `el.log` present and not hidden — NOT on the lens name. That distinction matters: `sessions`, `board` and `inbox` paint inside `panel-chat` and keep the log up, so keying on the lens put their answers on the strip and broke three cases. When the transcript is down, the line lands on `#face-say`: a child of `.face` (outside every panel), fixed above the console, right-inset by `--pane-right` so the drawer never covers it, clearing after eight seconds.

**Three tests were pinning the bug.** ARCH's archive refusal, C0's health transition and PM-D's placeholder all asserted their text inside `#log` on surfaces where the log is hidden. They were enforcing the silence. Each now asserts the visible strip (PM-D also asserts the clipboard).

**Harness fidelity, twice.**
- The stub ran `setTimeout` callbacks **synchronously**, so an 8-second clear fired before any assertion could see the strip. Timers are now queued and drained by `flushTimers()`.
- The mock had **no mapping** for `/api/second-brain`, `/api/second-brain/plugin`, `/api/second-brain/act` or `/api/memories/consolidate`, so a case booting those surfaces silently got `{}`. That is why the health refresh looked broken during investigation — the job picker rendered with no options and posted `job: ""`. Mapped now.

**The sweep, as a permanent case.** 248 acting presses across 19 lenses; a press that navigates is exempt (the new surface answers) and so is a refresh (the repaint answers), both named explicitly. Result: no press does its work in silence. Building it also surfaced **KI-566** — `settingsSections` dereferences `read.soul.present` unguarded while `paintSettings`' completeness check tests `scalars` alone, so a partial read throws inside a fold handler.

Red-proofs: reverting `notice` reproduces `(nothing said)` on the Intelligence press; silencing only the consolidation confirmation reddens the surface-act pin.
