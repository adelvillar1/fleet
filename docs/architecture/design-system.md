# The design system — read this before touching the face

The browser face (`src/gui/assets/`) is one product with one grammar. This
packet is the authoritative summary: the tokens, the type, the components,
the compositions a surface is built from, and the anti-patterns. It is
derived from the shipped `app.css` — if a value here and the stylesheet
disagree, the stylesheet is right and this file is a bug.

## The philosophy

**A console that reads like a desk.** Warm paper, ink, one accent; the
display serif for names and headlines, the system sans for prose, the mono
for anything measured (ids, counts, times, sizes, paths). Density is
comfortable, not an ops console. The surface answers the owner's question
first and keeps the machinery behind a door that names itself.

## The colour chain (three tiers, nothing skips one)

1. **Reference** — `--dsw-static-*`: where a colour came from.
2. **Alias** — `--dsw-alias-*`: what it means. **Themes re-point this tier
   only**; no component rule moves.
3. **Component** — `--card-radius`, `--pane-width`, `--work-width` …:
   where it lands.

| alias | paper | console (dark) | graphite | contrast |
|---|---|---|---|---|
| `--dsw-alias-page` | `rgb(246,244,239)` | `rgb(14,15,18)` | `rgb(10,12,16)` | `#fff` |
| `--dsw-alias-surface` | `rgb(252,251,248)` | `rgb(20,22,26)` | `rgb(15,18,23)` | `#fff` |
| `--dsw-alias-surface-raised` | `rgb(239,236,230)` | `rgb(28,31,36)` | `rgb(21,25,31)` | `rgb(238,238,238)` |
| `--dsw-alias-ink` | `rgb(24,26,29)` | `rgb(235,231,228)` | `rgb(232,238,244)` | `#000` |
| `--dsw-alias-ink-secondary` | `rgb(82,88,96)` | `rgb(166,175,188)` | `rgb(160,170,184)` | `rgb(38,38,38)` |
| `--dsw-alias-ink-tertiary` | `rgb(94,102,112)` | `rgb(138,148,163)` | `rgb(132,144,160)` | `rgb(70,70,70)` |
| `--dsw-alias-accent` | `rgb(47,95,208)` | `rgb(96,143,255)` | `rgb(94,148,255)` | `rgb(24,66,160)` |
| `--dsw-alias-ok` | `rgb(22,110,46)` | `rgb(74,190,110)` | same | `rgb(0,104,32)` |
| `--dsw-alias-warn` | `rgb(122,82,8)` | `rgb(220,166,66)` | same | `rgb(124,82,6)` |
| `--dsw-alias-down` | `rgb(176,36,32)` | `rgb(240,106,99)` | same | `rgb(176,20,20)` |
| `--dsw-alias-hairline` | `rgb(24 26 29 / 10%)` | `rgb(255 255 255 / 8%)` | same | `rgb(0 0 0 / 22%)` |

Themes: `paper` (default, `:root`), `console`, `graphite`, `contrast`;
`data-theme` on `<html>`, persisted in `localStorage['fleet-theme-2']`.
`src/gui/themes.rs` checks every text token against every surface at AA
(≥4.5:1) at build time.

**Rules.** A component reads aliases, never a hex. A state is a word **and**
a tone, never colour alone. Semantic colours are the three aliases plus the
accent; nothing else is coloured for decoration.

## Type

| role | stack | use |
|---|---|---|
| display | `"New York", ui-serif, Georgia, "Iowan Old Style", serif` | page titles, room names, the brand, markdown headings |
| text | `-apple-system, BlinkMacSystemFont, "SF Pro Text", "Segoe UI", Roboto` | everything read |
| mono | `ui-monospace, "SF Mono", JetBrains Mono, Menlo, Consolas` | ids, counts, times, sizes, code, paths |

Base `13.5px/1.55`; the transcript `15px/1.72`; the display ladder
`clamp(28–36px)` for a page title, `19–21px` for a rail head or a room
name. Labels are `9.5–10.5px`, `600`, uppercase, `letter-spacing .11em`.
Numbers carry `font-variant-numeric: tabular-nums`.

## Geometry

`--topbar-height: 58px`, `--rail-width: 236px` (`66px` collapsed),
`--work-width: 320px`, `--pane-width: 440px` (drag 300–720),
`--card-radius: 14px` (inner 10), `--composer-radius: 16px`,
spacing `4/8/12/16/20/24/32/40/48`, radii `7/10/14/20/999`, motion
`120ms/180ms` on `cubic-bezier(.2,0,0,1)`. Breakpoints `1180` (the work
column is the sheet), `900` (the rail is the drawer), `720`, `460`.

The shell is one grid: rail | stage | (pane). Rows: topbar, 2px meter,
work. Every surface is a `<main class="stage">` in column 2.

## The components a surface is built from

| component | class | notes |
|---|---|---|
| rail | `.surfaces`, `.surfaces-section`, `.surfaces-group`, `.surfaces-link` | 34px rows, 8px radius, 17px icon; a current row is raised with a 2px accent bar; the group label folds its rows |
| section rail | `.section-rail`, `.section-rail-row`, `.section-rail-mark` | the second rail idiom (memory, skills, calibration, settings, bots, boards); folds to 46px |
| work column | `.work-col`, `.work-head`, `.work-list`, `.work-group`, `.work-row` | absolute between the rail and the stage; 48px strip; the sheet below 1180 |
| rooms column | `.room-row`, `.room-bar`, `.rooms-group` | the work column's other tenant; 3-line rows |
| conversation | `.column`, `.log`, `.stream`, `.owner`, `.assistant` | 820px measure |
| room head | `.room-head`, `.room-title`, `.room-state`, `.member`, `.room-shelf`, `.artifact` | one header per room |
| event card | `.card-row`, `.card-open`, `.glyph` | the transcript's one non-prose shape |
| composer | `.composer`, `.foot-fields`, `.runbar-field`, `.foot-btn` | one filled button (send) |
| pane | `.pane`, `.pane-tabs`, `.pane-body`, `.pane-preview` | the right reading surface |
| buttons | `.replay-button`, `.room-act`, `.press` (base/primary/quiet), `.ghost` | `.press` is the shared door button |
| empty states | `.empty`, `.room-empty`, `.board-empty`, `.pane-empty` | the surface's own words, never a skeleton |
| doors | `.door`, `.field`, `.picks` | a modal task, one press to close |

## Icons

One table, `ICONS` in `app.js`: 24 viewBox, 1.6 stroke, round caps, one
drawing per concept, drawn to read at 17px. A key with no drawing renders
`unknown` (a dashed circle with a question mark) — a defect made visible.
Sizes: 17 rail/section, 14 palette, 13 card. The harness pins every key the
source asks for.

## States and motion

Loading is a state word and a spinner, never a skeleton. Progress is
`scale(.98)` on press; transitions 120–180ms; nothing animates on load.
A refused read keeps the last good value visible and names the refusal.
Empty, loading, refused and selected are four different sentences.

## Accessibility

Focus is visible everywhere (`:focus-visible` rings); every control has an
`aria-label` or a visible label; the seat's decisions carry digit keys; the
rail's toggle keeps its words in `aria-label`/`title`; colour is never the
only signal (state words accompany every tone); `prefers-reduced-motion`
turns transitions off; every text token clears AA in all four themes, and
the persona marks clear 4.5:1 on their own discs.

## Anti-patterns (don't)

- A new hex, a gradient, a glow, a glass panel, an emoji icon.
- A card with a border **and** a shadow **and** its own background as three
  separations.
- A generic "dot" for a missing icon (it reads as a state).
- A label beside a control that every other control states with an icon.
- A modal for something the console does inline (the approval seat).
- A skeleton loader where a state word is true.
- A second list of surface names that can drift from `GROUPS`.

## Files

`src/gui/assets/index.html` (the shell and the panels), `app.css` (tokens,
components, themes), `app.js` (the surface builders; `ICONS`, `GROUPS`,
`paintWork`, `paintRooms`, `paintRoomHead`, `paintConvHead`), `themes.rs`
(the AA guard), `tests/face/harness.mjs` (the behavioural pins).
