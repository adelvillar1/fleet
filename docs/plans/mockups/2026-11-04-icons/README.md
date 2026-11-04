# Icons — the sheet

Drawn before the code (the mockups-first ruling). Plan:
[`docs/plans/2026-11-04-icons.md`](../../2026-11-04-icons.md).

The palette is `app.css`'s, hand-copied: the alias tier keeps its names and
values, so a mark read here is the mark the face draws. One data file
(`icons.js`) holds every key — its shapes, its meaning, where it is used
and whether it is kept, redrawn, new or deleted — and the three pages are
renders of it. The implementation transcribes the same table into `ICONS`
in `src/gui/assets/app.js`.

| page | what it shows |
|---|---|
| `sheet.html` | Every key at its four real sizes — 13 in a transcript card, 14 in the palette, 17 in a rail, 18 in a section rail — with its meaning and use. A redrawn key shows **was** above and its replacement below. |
| `rails.html` | The main rail (fifteen destinations), the four section rails (memory, skills, calibration, settings) and the bots rail, read at 236px with 34px rows — the context that matters. |
| `toggle.html` | The rail head with the collapse control as a chevron **alone**, in both modes. |
| `zoom.html` | The drawings most at risk at 17px, at 68px (17 × 4): the eye judges pixels here, not intent. |

## What the audit found

- **Thirteen keys are declared and never drawn.** `icon()` falls back to
  `ICONS.dot` silently, so on the main rail **huddles, vault and plugins —
  three built destinations — are the same hollow ring**; in the section
  rails, six of memory's ten rows and three of settings' seven are
  indistinguishable from a failure.
- **One drawing for many concepts.** The clock was sessions, memory's
  archive, skills' media and calibration's turn; the folder was memory's
  project, skills' documents, calibration's consolidate and every
  workspace project; the route was the router, skills' web, settings'
  models and calibration's routes.
- **Drawings that cannot read at 17px**: the seven-element workspace
  graph, the six-path delegation, the antenna-and-eyes robot, the
  micro-clipboard plan, the four-peak pulse, the filled thinking diamond.
- **No mark at all**: the huddles rail (declared, never populated), the
  plugins sections, the bots personas (one shared stack), the pane tabs.
- **The owner's divergence**: the rail's collapse control is a text
  chevron plus the words "collapse the rail", while every section rail's
  fold is a chevron alone.

The sheet itself caught one more: `repo` was used by memory's "project"
row and by every workspace project but had no drawing — it rendered the
unknown state in the first render of `rails.html`.

## The principles the drawings follow

- **One concept, one drawing.** A surface and its own section share a mark
  (memory's book, design's artboard, a project's folder, the needs-you
  bell); two different concepts never do.
- **Legible at 13px.** Two to five elements, ≥2-unit detail on the 24
  grid, 1.6 stroke. Anything with micro-clips, r1.3 dots or six sub-paths
  was redrawn.
- **The unknown state is visible.** A key with no drawing renders a dashed
  circle with a question mark — a defect that looks like a defect, never
  like the intentional "everything else" mark (three dots).
- **No new alphabet.** The same 24 grid, the same stroke, the same round
  caps; the drawings stay an inline SVG table, not a font or a sprite.

## How it was verified

Rendered in Chromium at 1440×968 and measured over the DevTools protocol
in all four themes (paper, console, graphite, contrast):

| fact | target | measured |
|---|---|---|
| horizontal overflow | 0 | 0 on all four pages, all themes |
| elements past the right edge | 0 | 0 |
| clipped text (non-ellipsis) | 0 | 0 |
| marks measured | — | 66 in the rails, all four themes |
| mark ink on its surface | ≥4.5:1 | **5.5:1** paper, 6.1 console, 5.9 graphite, 9.4 contrast |
| icon box | 17px | 17px |
| rail row | 34px (the app's) | 34px |

Commands:

```sh
"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --headless=new \
  --user-data-dir=/tmp/ic --screenshot=rails.png --window-size=1440,1500 \
  "file://$PWD/rails.html"
```
