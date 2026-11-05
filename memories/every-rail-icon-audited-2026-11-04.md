---
name: every-rail-icon-audited-2026-11-04
description: "project 2026-11-04 — the owner ordered an audit of every icon on every rail (and the rail toggle reduced to its chevron), then the groups made collapsible; the plan is docs/plans/2026-11-04-icons.md (C1–C14), the sheet (57 keys, rails in situ, toggle, 4× read, the folded rail) is committed at 4c97c46 + f90874a, and the code awaits the owner's read"
type: project
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

After the huddles UX mockup ([[huddles-ux-redesign-2026-11-04]]) drew the
main rail by hand, the owner asked for **that process applied to every icon
on every rail**, plus one named divergence: *"on the main rail the
'collapse the rail' + chevron should be just an icon, there is no need for
the text as it diverges from the design system we have."*

Plan: `docs/plans/2026-11-04-icons.md` (C1–C14 after the owner's addition).
The audit's findings are
in [[a-missing-icon-falls-back-to-dot-silently]] — thirteen keys never
drawn, one drawing for many concepts, seven unreadable at 17px, `GLYPHS` /
`GROUP_MARKS` / `tower` dead, the huddles rail declared and never filled,
bots' persona rows on one shared stack, pane tabs words-only.

**Decisions locked in the plan:**

- **One drawing per concept**, and a surface may share a mark with its own
  section (memory's book, design's artboard, a project's folder, the
  needs-you bell).
- **`unknown` is a visible fallback** (dashed circle + ?); `other` (three
  dots) is the intentional "everything else / declined" mark.
- **A source pin kills the class**: a harness case reads `app.js` as text
  and fails on any `icon('…')` / section-map key `ICONS` does not draw,
  proven red by deleting a key.
- **Bots personas wear their `pmark` initials** (the mark huddles uses),
  not one shared `skills` stack; channels keeps the robot.
- **`#huddles-rail` is DELETED, not filled** — the huddles UX plan replaces
  that surface and its rail is the rooms column.
- **The toggle is the chevron alone**: an SVG from the set, 16px,
  rotating 180° in icons mode, `#rail-toggle-name` deleted,
  `aria-label`/`title` carry the words — matching the section rails' fold,
  which has always been chevron-only.
- **One stroke**: 24/1.6 everywhere; the rail menu (1.7) and send (1.8)
  inline drawings come in line.
- **Out**: pane tabs' words (a tab strip, not a rail), the transcript
  cards' text glyphs (`▦ ◇ ±`), a font/sprite.

**What was drawn** (57 keys): redraws of `sessions` (a list card, not a
clock), `workspace-history` (a git branch), `delegation` (parent over two
children), `bots` (a face with a stem), `plan` (a clipboard), `router` (a
fork, so it no longer reads as the calibration pulse), `calibration` (two
peaks), `thinking` (an open diamond), `dot` (a filled bullet); new
`huddles` (two figures), `vault` (a lock), `trust` (an open lock),
`plugins` (a puzzle piece), `pending` (a bell), `repo` (a folder — it had
NONE), `person`, `rules`, `events`, `retired` (a moon), `archive`,
`today`, `web` (a globe), `pen`, `media`, `documents`, `data`,
`platforms`, `curation`, `declined`, `other`, `gate`, `spark`, `eval`,
`recall`, `consolidate`, `screen`, `question`, `verify`, `timing`,
`cycle`, `models` (a 4-pin chip), `roles`, `schedule`, `interface`,
`environment` (a key), `theme`, `unknown`, `chevron`.

**Phase 0 committed (4c97c46)** at
`docs/plans/mockups/2026-11-04-icons/`: `sheet.html` (every key at its four
real sizes — 13 card / 14 palette / 17 rail / 18 section — with meaning and
use, `was` above each redraw), `rails.html` (main rail + memory + skills +
calibration + settings + bots initials, in situ at 236px), `toggle.html`
(both modes), `zoom.html` (12 at-risk drawings at 68px), all rendered from
one `icons.js` data table — the implementation transcribes that table.

**Measured, four themes:** 0 overflow / 0 past-edge / 0 clipping on all
four pages; 66 marks in the rails at ink **5.5 paper · 6.1 console · 5.9
graphite · 9.4 contrast**; icons 17px; rows 34px. The measurement script
had to parse `color(srgb 0.18 …)` — the huddles-era regex/canvas trick was
replaced with explicit srgb parsing after it misread the ink at 2.5:1
(see [[measuring-a-mockup-by-reading-the-png]]).

**The owner's addition (C13): the rail's groups FOLD.** After reading the
sheet — *"can we include in the plan to make the categories on the rails
collapsible? that way we improve screen real state usage"* — each group
label (work · intelligence · knowledge · system) became a control that
folds its rows to the label and back. The rulings the implementation
carries: a folded group **keeps its needs-you count on the label** so
nothing hides silently; the fold is **remembered per group** across
launches (the rail's own collapse idiom); a folded group **keeps its
divider** so the rail's rhythm holds; **icons mode shows every row** (the
labels are hidden there, so the fold is restored when the rail expands);
and the rule applies wherever a rail renders `group` headers (the design
boards by project), not one-off. Drawn at `rails.html` as the open rail
and the same rail with work + intelligence closed — **6 rows of 15**,
~240px returned. Committed f90874a.

**State at the close:** the sheet — now with the folded rail — is handed
back for the owner's read as the gate; the code phases (table, drawings,
rails, toggle, folding groups, harness, deploy) are NOT started. Next: on
"go", land P1–P5, then gate → live proof →
`cp target/release/fleet ~/.fleet/bin/fleet` + bootout/bootstrap + prove
`/app.js` → dmg → push.
