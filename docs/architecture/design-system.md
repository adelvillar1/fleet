# The design system — read this before touching the face

The browser face (`src/gui/assets/`) is one product with one grammar. This
packet is the authoritative summary: the tokens, the type, the components,
the compositions a surface is built from, and the anti-patterns. It is
derived from the shipped `app.css` — if a value here and the stylesheet
disagree, the stylesheet is right and this file is a bug.

## The philosophy

**A console that reads like a desk.** One accent, a display face for names
and headlines, the system sans for prose, the mono for anything measured
(ids, counts, times, sizes, paths). Density is comfortable, not an ops
console. The surface answers the owner's question first and keeps the
machinery behind a door that names itself. A card is **a room you enter,
not a console you audit**.

## The colour chain (three tiers, nothing skips one)

1. **Reference** — `--dsw-static-*`: where a colour came from.
2. **Alias** — `--dsw-alias-*`: what it means. **Themes re-point this tier
   only**; no component rule moves.
3. **Component** — `--card-radius`, `--room-bg`, `--title-card` …: where
   it lands.

| alias | night (default) | day | console | contrast |
|---|---|---|---|---|
| `--dsw-alias-page` | `rgb(8,19,26)` | `rgb(246,244,239)` | `rgb(14,15,18)` | `rgb(255,255,255)` |
| `--dsw-alias-surface` | `rgb(15,37,48)` | `rgb(252,251,248)` | `rgb(20,22,26)` | `rgb(255,255,255)` |
| `--dsw-alias-surface-raised` | `rgb(18,48,60)` | `rgb(239,236,230)` | `rgb(28,31,36)` | `rgb(238,238,238)` |
| `--dsw-alias-ink` | `rgb(233,244,245)` | `rgb(24,26,29)` | `rgb(235,231,228)` | `rgb(0,0,0)` |
| `--dsw-alias-ink-secondary` | `rgb(164,190,198)` | `rgb(60,72,82)` | `rgb(166,175,188)` | `rgb(38,38,38)` |
| `--dsw-alias-ink-tertiary` | `rgb(150,178,188)` | `rgb(63,75,85)` | `rgb(138,148,163)` | `rgb(70,70,70)` |
| `--dsw-alias-accent` | `rgb(58,212,242)` | `rgb(10,147,184)` | `rgb(96,143,255)` | `rgb(24,66,160)` |
| `--dsw-alias-ok` | `rgb(55,226,164)` | `rgb(22,110,46)` | `rgb(74,190,110)` | `rgb(0,104,32)` |
| `--dsw-alias-warn` | `rgb(255,178,36)` | `rgb(122,82,8)` | `rgb(220,166,66)` | `rgb(124,82,6)` |
| `--dsw-alias-down` | `rgb(255,120,113)` | `rgb(176,36,32)` | `rgb(240,106,99)` | `rgb(176,20,20)` |
| `--dsw-alias-hairline` | `rgb(86 180 200 / 16%)` | `rgb(24 26 29 / 10%)` | `rgb(255 255 255 / 8%)` | `rgb(0 0 0 / 22%)` |

Themes: `night` (default, the `:root` block), `day`, `console`,
`contrast`; `data-theme` on `<html>`, persisted in
`localStorage['fleet-theme-2']`. `src/gui/themes.rs` checks every text
token against every surface at AA (≥4.5:1) **and** the six persona tints
against the theme's own ink, in the report the picker reads.

**P7 tokens.** `--on-tone` (the label on a tone-filled bead), `--on-flat`
(the label on a flat accent fill — a different background from the accent
gradient, measured per theme), `--scrim`/`--scrim-soft` (door and pane
dims), `--room-*` (the card vocabulary), `--title-surface/-room/-card`,
`--eyebrow-*`, `--measure-*`, and `--tint-1…6` (the six persona seats).

**Rules.** A component reads aliases, never a hex. A state is a word **and**
a tone, never colour alone. Semantic colours are the three aliases plus the
accent; nothing else is coloured for decoration. `testcensus::ASSETS`
carries the shipped stylesheet and a guard refuses any hex literal outside
the token block.

## Type

| role | stack | use |
|---|---|---|
| display | `"Space Grotesk", "Avenir Next", "SF Pro Display", system-ui, sans-serif` | page titles, room names, card heads, the brand, markdown headings |
| text | `-apple-system, BlinkMacSystemFont, "SF Pro Text", "Segoe UI", Roboto` | everything read |
| mono | `ui-monospace, "SF Mono", JetBrains Mono, Menlo, Consolas` | ids, counts, times, sizes, code, paths, every measured value |

Base `13.5px/1.55`; the transcript `15px/1.72`.

**The display ladder (P7).** One voice per job, and nothing else may look
like a title: surface title `clamp(30px, 3.2vw, 40px)/600` at `-0.035em`;
room name `24px/600`; card head `17px/600` in the display face; column
name `15px/600`. Eyebrows are the quiet label voice — `10.5px/600`,
uppercase, `letter-spacing .16em`, tertiary ink — never a 12px accent
shout. Measured values (`.tile-value`, `.home-stat-value`, stamps) are
mono `500` with `font-variant-numeric: tabular-nums`. Numbers carry
`tabular-nums` wherever they align in a column.

## Geometry

`--topbar-height: 58px`, `--rail-width: 236px` (`90px` collapsed),
`--work-width: 320px`, `--pane-width: 440px` (drag 300–720),
`--room-radius: 12px`, `--room-pad: 18px 20px 20px`,
`--composer-radius: 16px`, spacing `4/8/12/16/20/24/32/40/48`, radii
`7/10/14/20/999`, motion `120ms/180ms` on `cubic-bezier(.2,0,0,1)`.
Breakpoints `1180` (the work column is the sheet), `900` (the rail is the
drawer), `720`, `460`.

The shell is one grid: rail | stage | (pane). Rows: topbar, 2px meter,
work. Every surface is a `<main class="stage">` in column 2.

**The card vocabulary (P7).** One ground (`--room-bg`, the surface mixed
6% toward the page), one hairline, one radius, one padding rhythm, and
**no shadow**. A card separates itself once; a border **and** a shadow
**and** its own ground is three separations and reads as a widget. Applied
to `section.board-card`, `.suggest`, `.dk-owed`, `.stats-strip`, `.tiles`,
`.surface-tool`; the dense kanban `.board-card` keeps its compact padding
and also carries no shadow.

## The components a surface is built from

| component | class | notes |
|---|---|---|
| rail | `.surfaces`, `.surfaces-section`, `.surfaces-group`, `.surfaces-link` | 34px rows, 8px radius, 17px icon; a current row is raised with a 2px accent bar; the group label folds its rows |
| section rail | `.section-rail`, `.section-rail-row`, `.section-rail-mark` | the second rail idiom (memory, skills, calibration, settings, bots, boards); folds to 46px |
| work column | `.work-col`, `.work-head`, `.work-list`, `.work-group`, `.work-row` | absolute between the rail and the stage; 48px strip; the sheet below 1180 |
| rooms column | `.room-row`, `.room-bar`, `.rooms-group` | the work column's other tenant; 3-line rows |
| conversation | `.column`, `.log`, `.stream`, `.owner`, `.assistant`, `.turn` | 820px measure; a turn carries a 2px persona spine |
| room head | `.room-head`, `.room-title`, `.room-state`, `.member`, `.member-state`, `.room-shelf`, `.artifact` | one header per room |
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
`scale(.98)` on press; transitions 120–180ms; **at most one transition per
state change** (a card's hover warms its border, it does not lift);
nothing animates on load. A refused read keeps the last good value visible
and names the refusal. Empty, loading, refused and selected are four
different sentences.

**The four sentences live in one table.** `SENTENCES` in `app.js` holds
each surface's empty, loading, refused and selected wording — keyed by
the surface's own address, with a function where the sentence carries a
count or a name — and the builders read it rather than holding a second
copy. The selected state is a `derivation` line carrying
`data-state="selected"`; `tests/face/harness.mjs` pins all four per
surface by content, so a deleted sentence fails by name.

`@media (prefers-reduced-motion: reduce)` is a real rule: 1ms durations,
one iteration, zero delay — a reduced-motion page paints its final state
once and stops. `themes.rs` pins that rule's declarations.

## Accessibility

Focus is visible everywhere (`:focus-visible` rings); every control has an
`aria-label` or a visible label; the seat's decisions carry digit keys; the
rail's toggle keeps its words in `aria-label`/`title`; colour is never the
only signal (state words accompany every tone); every text token clears AA
in all four themes, the persona marks clear 4.5:1 on their own discs, and
the flat-accent label (`--on-flat`) clears AA on every theme.

## Anti-patterns (don't)

- A new hex, a gradient, a glow, a glass panel, an emoji icon.
- A card with a border **and** a shadow **and** its own background as three
  separations.
- A title set in the body face, or a measured value set in prose.
- An eyebrow louder than the title it introduces.
- A generic "dot" for a missing icon (it reads as a state).
- A label beside a control that every other control states with an icon.
- A modal for something the console does inline (the approval seat).
- A skeleton loader where a state word is true.
- A second list of surface names that can drift from `GROUPS`.

## Files

`src/gui/assets/index.html` (the shell and the panels), `app.css` (tokens,
components, themes), `app.js` (the surface builders; `ICONS`, `GROUPS`,
`paintWork`, `paintRooms`, `paintRoomHead`, `paintConvHead`), `themes.rs`
(the AA guard), `src/testcensus.rs` (the asset census and the no-hex
guard), `tests/face/harness.mjs` (the behavioural pins).
