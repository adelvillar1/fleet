---
name: a-missing-icon-falls-back-to-dot-silently
description: "feedback 2026-10-31, class-wide 2026-11-04 — `icon()` falls back to `ICONS.dot` silently, so a missing drawing reads as a deliberate minimal mark; the 2026-11-04 audit found THIRTEEN such keys (three built main-rail destinations were the same ring), so the fallback must be a visible `unknown` plus a source pin, not a per-request fix"
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

**2026-10-31 (the design rail's glyph).** The owner asked for *"an icon
similar to something from this image of design icons"*, pointing at a
stroke sheet of artboards, palettes, rulers, layers. The finding before any
drawing: the `design` address had **no entry in `ICONS` at all**, so
`icon()` was rendering `ICONS.dot` — a plain circle that reads as a
deliberate minimal mark. The rail had been wrong since the stage landed and
nothing said so.

**Why it matters:** `icon(name, className)` resolves
`(ICONS[name] || ICONS.dot)` — a fallback that is *invisible as a failure*.
Every new surface inherits it silently, and the missing entry shows up as
"the owner asked for a better icon" rather than "the icon was never there".

**How to apply:** when a surface's glyph is asked for or questioned, grep
`ICONS` for the address FIRST. If it is absent, the fix is one entry plus a
harness check — not a redesign. Draw new glyphs in the set's own terms: a
24-unit viewBox, `stroke-width` 1.6, `stroke=currentColor`, three or four
`[tag, attrs]` parts, so it reads at the rail's 18px beside the existing
marks. The design glyph landed as an **artboard with a sun and a horizon**
(`rect` + `circle` + `path`), matching the sheet's frame-with-image motif,
and the boards rail's per-board rows took the same mark for free because
they already passed `icon: 'design'`.

**Pin the fallback's absence, not the drawing:** the harness check asserts
the design link carries a `rect` **and** a `circle` — a shape assertion that
fails if the entry disappears, where "an svg exists" would pass on the dot.
Same shape of lesson as [[a-green-pin-can-hide-a-broken-first-run]]: the
thing that was supposed to exist was never asserted. The drawings follow one
contract — 24 viewBox, `stroke-width` 1.6, round caps/joins, two to five
parts, ≥2-unit detail so it reads at 13px.

**2026-11-04, THE CLASS, not the instance** ([[every-rail-icon-audited-2026-11-04]]).
An audit of every icon on every rail found **thirteen keys declared and
never drawn**, all rendering the same hollow ring: on the main rail
**huddles, vault and plugins** (three built destinations); memory's
needs-you/user/feedback/episodic/retired/today (6 of 10 rows); skills'
needs-you/design/curation/retired; calibration's gate/eval_corpus/
embed_timing/lifecycle; settings' trust/schedule/interface (3 of 7); and
then `repo` — used by memory's "project" row and every workspace project —
a fourteenth, caught only when the proposed sheet was RENDERED in situ.
The same audit found one drawing for many concepts (the clock was
sessions, memory's archive, skills' media and calibration's turn; the
route was the router, skills' web, settings' models), drawings that cannot
read at 17px (the 7-element workspace graph, the r1.3 robot antenna), and
dead icon code (`GLYPHS`, `GROUP_MARKS`, the unused `tower` drawing).

**How to apply, generalized:** a silent fallback + a hand-kept table is a
class that WILL drift, so pin the CLASS — a harness case that reads
`app.js` as text, extracts every `icon('…')` call and every key in the
icon-carrying section maps, and fails on any key missing from `ICONS` — and
make the fallback a VISIBLE `unknown` (dashed circle + question mark,
never the intentional "everything else" mark) so a defect looks like a
defect. And draw every rail in situ on a sheet before landing: the
render, not the table, caught `repo`.
