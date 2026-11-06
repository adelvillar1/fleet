---
name: fleet-work-column-follows-the-rail
description: "2026-10-23 (ae70ca5) — the work column was anchored to the expanded rail's width so folding the nav to icons left a 170px gutter, and it could not be folded at all; it now reads its left edge from the rail mode and folds to a 48px strip (way back + needs count), remembered in localStorage[\"fleet-work\"]. Also records why no control fits the composer's 794px footer, where the size pair went instead, and the two gate failures this shift produced (KI-287's half-split flag, KI-308's literal `sk-` scan)"
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

Owner, 2026-10-23, right after the work/conversation revamp shipped: *"when the
left rail is collapsed, the work rail should move to the available space, also
the work rails should be collapsible as well."* Two defects, both visible only
by looking at the shipped surface.

**KI-305 — the stranded column.** `.work-col` declared `left: var(--rail-width)`
(236px) and nothing else, while the icons mode is 66px, so folding the nav left
a 170px band of dead page between the icons and the list. Fix: `body[data-rail=
"icons"] .work-col { left: var(--rail-width-icons) }`. **The general rule: a
column positioned against another element's width must read that element's
mode, not its default.**

**KI-306 — the missing fold.** The list was always 320px wide on a wide screen.
Now `body[data-work]` is a three-valued mode: `open`, `collapsed`, or empty
(hidden). `setWork(open)` maps to `collapsed` when `face.workFolded`, so the
mode and the width can never disagree; `--work-width` is 48px under
`collapsed`, and `.stage`'s margin reads `var(--work-width)` in both modes —
one number, one ruler. The strip carries the expand control and the archive's
own `counts.needs` badge, nothing that needs a row's width. The choice is kept
in `localStorage["fleet-work"]` (like the dock's `fleet-aside`) and read before
the first route paints. The back control learned the third state: on a narrow
page it unfolds a folded list, closes an open sheet, and opens a closed one
(head's word is "work", not "away"). Live at 1440: col left 236/full and
66/icons against rail right 236/66, folded width 48 with the stage at rail+48,
fold surviving `page.reload()`.

**KI-307 — SUPERSEDED the same day (25a93f8 → c96d729).** The first build put
`A−`/`A+` on the composer's textarea (12–22px, `--composer-size` on the form,
`localStorage["fleet-composer-size"]`); the owner then corrected the target —
they meant the conversation above the composer — and the conversation now carries
an 85–170% multiplier while the composer control was removed. The composer is back
at its 14.5px measure. The scope lesson still holds from that build: **when the
owner says "only there, the rest is perfect", prove the scope with computed
styles, not by eye.** See [[fleet-text-size-belongs-to-the-reading-surface]] for
what replaced it and why a multiplier, not a pixel step.

**The placement lesson.** The first draft put the pair in the composer footer
beside Send and it wrapped the actions onto a second row — measured in a real
resumed conversation the footer is 794px of 545px fields + 225px actions with
the Shift+Enter hint already squeezed to 8px. **A footer row that is full at
the composer's fixed 794px width has no room for anything new**; the pair moved
into the field's own top-right corner with 54px of reserved right padding so a
long line never runs under the glyphs. Measure with and without the addition
before choosing a home for a control.

**Two gate failures, both fixed rather than excused.** KI-287 (nomenclature):
the vocabulary's `truncated` was still being set by the trailing
`paths.next().is_some()` after a clock break — the tree-fact/machine-fact split
had only touched the loop — and the test's equality compared a flag read at the
last loop check against an `elapsed` read after the fold and index, false by
construction. Now a clock stop rides `budget_exhausted` alone and the test
asserts the implication plus the separation
([[a-flag-that-conflates-a-tree-fact-with-a-machine-fact]]). KI-308:
`no_payload_carries_a_credential` forbids the literal `sk-` in any served
asset, so a CSS class named `task-wrap` reddened the suite with a 43KB payload
as the "evidence"; the class was renamed `write-wrap` rather than weakening a
security guard, and the row is open naming the two real fixes (match a key's
shape, or print the matched word and offset).

Related: [[fleet-work-and-the-conversation-revamp]] (the surface these fix),
[[gui-client-verified-by-a-vm-harness]] (the harness case and the Playwright
pass), [[a-declared-grid-row-re-orders-the-footer]] (the sibling layout bug),
[[gui-composer-width-is-not-the-windows]] (the composer is 794px, not the
viewport).
