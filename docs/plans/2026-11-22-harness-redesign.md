---
status: active
created: 2026-11-22
plan: harness-redesign
---

# The harness redesign — one design system, every surface, rail + drawer everywhere

The owner's ruling (2026-11-22), quoted:

> "unfortunately we need to go back to the UI, first I need you to gain deep
> understanding of the surfaces in fleet, there should be around 20 of them.
> then let's do a clean redesign of the harness ux/ui. Rail and contextual
> drawer must be prevalent on all experiences. fleet's design features
> borrowed from my design canvas project I want to see mockups for a new
> design system and all the new surfaces. you have free reign to provide the
> best possible user experience possible, rearrange surfaces as needed.
> Don't break the app and make sure that the pwa for mobile and ipad are
> working. The bot experience must be exceptional, base on oh-my-hermes as
> needed. also run a hygiene pass after every delivery/cut/pass whichever way
> you want to call it"

This is one request with two halves: **understand** (the deep pass over every
surface, done in the same turn this plan was accepted), then **redesign**
(mockups first, owner's gate, then implementation per surface).

## The surfaces inventory (measured against the tree, 2026-11-22)

Nineteen destinations in the browser face, plus two standalone faces and the
bundle mounts — "around 20" confirmed:

| # | address | plain name | group | surface grammar today |
|---|---------|-----------|-------|----------------------|
| 1 | `sessions` | Conversations | workspace | landing + transcript + work list |
| 2 | `board` | Board | workspace | kanban columns |
| 3 | `desk` | Desk | workspace | shelf + deliverable + stages |
| 4 | `memory` | Intelligence | workspace | section rail + corpus sections |
| 5 | `learning` | Learning | workspace | section rail + lesson/graph/practice/history |
| 6 | `calibration` | Reviews | workspace | gate first, then op sections |
| 7 | `inbox` | Inbox | workspace | rows + thread |
| 8 | `second-brain` | Health | workspace | atlas + lanes + doctor |
| 9 | `armada` | Fleet | workspace (armada on) | roster + deployment map |
| 10 | `settings` | Settings | advanced | eight sections, section rail |
| 11 | `router` | Routing | advanced | live decision + chains |
| 12 | `delegation` | Helpers | advanced | tree + graph |
| 13 | `bots` | Assistants | advanced | personas + channels + pairing |
| 14 | `workspace-history` | Versions | advanced | spine tree + store + files |
| 15 | `vault` | Secrets | advanced | names sealed, values never |
| 16 | `plugins` | Plugins | advanced | tiers + bundles + drafts |
| 17 | `design` | Design | advanced | the canvas (boards, frames, pins) |
| 18 | `huddles` | Huddles (folded) | conversations | rooms + transcript |
| 19 | `skills` | Skills (folded) | intelligence | proposals + library |
| + | `#/<bundle>` | bundle mounts | — | sandboxed srcdoc frames |
| + | `bot.html` | the bot PWA | — | phone shell: inbox, conversation, seat, goals |
| + | `console.html` | the mini console | — | one column + command line + panels |
| + | the TUI | terminal face | — | ratatui: sessions, delegation graph, boards |

## The design source: design-canvas

The owner's own product, read at
`~/Projects/design-canvas/docs/architecture/design-system.md`. What transfers,
named:

- **The aesthetic — "Indigo Studio".** Cool blue-tinted neutrals, near-black
  ink, ONE indigo-violet accent (`#645FDF`, OKLCH `0.56 0.19 280`), crisp
  hairline borders, soft restrained shadows (three elevation tokens), calm
  density, generous whitespace. Explicitly NOT generic SaaS, no glassmorphism,
  no neon.
- **OKLCH tokens** with hex references and **measured, published contrast
  pairs** (the AA table is part of the spec, not an afterthought).
- **The type system**: Space Grotesk (display, `-0.02em`), Inter (body/UI),
  mono (measured values, tabular-nums). Display accents one word of a
  headline, never italic.
- **The elevation law**: `--shadow-1/2/3` — panel, frame-on-canvas, modal.
  Selection is a ring with an offset, never a shadow change.
- **The canvas-first philosophy**: the artifact is sacred; chrome serves it
  and never competes.
- **Mobile shell law**: full-screen canvas below `lg`, panels as slide-over
  drawers, `lg:contents` wrappers so the breakpoint changes the shell only,
  never panel internals.

What transfers **as grammar, not as color**: Fleet keeps its own themes and
its persona seats; the eight-signal law (U4) re-hues into the indigo system
rather than gaining a tenth hue.

## The design contract (defaults decided on the owner's behalf, revisable at any row)

The owner granted free reign; the recorded defaults, each revisable when they
say so:

- **D1 — One design system, "Indigo Studio" adapted to fleet.** OKLCH token
  chain (reference → alias → component, the existing three tiers kept), cool
  graphite night as the default theme, cool paper day, console and contrast
  retained. One accent (indigo-violet) + the signal law (ok/warn/down),
  persona seats stay (six tints re-derived on the new surfaces, AA-guarded).
  Hairlines everywhere; **shadows return** at three fixed elevations — this
  supersedes the card law's "no shadow" rule BY NAME in the ledger, because
  design-canvas proves a soft elevation reads calm where a shadowless card
  reads flat against a dark page.
- **D2 — The shell is rail | stage | drawer, on every experience.** The rail
  (left, persistent, collapsible, state-grouped — needs you / working / the
  rest, the relevance pass's order carried whole). The **contextual drawer**
  (right) is the ONE reading surface — what opens for a card, a row, a
  detail — replacing the pane's mixed duties: the drawer follows the focused
  object (U3, U13 — owner's press only), and every surface that showed a
  right pane now shows the drawer. Below 1180 the drawer is the sheet; below
  900 the rail is a drawer too; the phone bot gets the same grammar at touch
  scale. No experience ships without both rails' grammar present.
- **D3 — The lens set holds; Advanced consolidates.** The nine lenses stay
  (U44 + its amendment). Huddles lives inside Conversations as its rooms
  column (already the folded home); Skills is Intelligence's second section
  (U40 already said so); Routing is Reviews' first section (the U44
  amendment's own words). Advanced keeps: Settings, Assistants, Versions,
  Secrets, Plugins, Design. Nothing is deleted; every address keeps
  resolving (U47's grammar).
- **D4 — The relevance contract carries whole.** Needs-you leads every
  surface, changed-since second, inventory collapsed below (the relevance
  pass is the floor, not the casualty). The redesign re-skins the order; it
  does not re-litigate it.
- **D5 — The bot experience is exceptional, with OMH's three borrowables.**
  The phone PWA is the primary bot companion: needs-you first (approvals on
  the go), the seat's answers with **evidence labels** (OMH's two-axis
  evidence idea), **done-marks that link their evidence** (OMH finding 4: a
  done is a claim with a receipt), and **suggestion lines ranked by meaning**
  (OMH's shortlist idea — fleet suggests, never fleet decides) in the
  composer. iPad gets a split view: rail + conversation + drawer side by
  side. The bot speaks the same design system as the face.
- **D6 — Don't break the app.** The gate is green at every landing; the face
  harness never falls (278 cases; a rebuilt surface re-pins its cases the
  same turn); PWA verified at phone (390×844) and iPad (820×1180) viewports;
  `bot-sw.js` scope/cache behavior untouched.
- **D7 — Hygiene after every delivery.** After every pass/cut/delivery:
  `scripts/repo-hygiene.sh --report`, `plan_check.py`, `ki_ledger.py`,
  `git status --short` read against the session's edit list, and the recap.
  This is a standing AC of every row below, not a phase of its own.

## What the redesign keeps unchanged

- The needs-you document and its per-surface consumption (`ab867bea`).
- U44's lens set, U45's two registers, U46's type floor, U47's address
  grammar, U48's four themes + press-always-lands, U50's per-class windows,
  U51's spine law, U13's drawer-opens-by-press, U42's plain register.
- The harness's sentence discipline (empty/loading/refused/selected, the
  `SENTENCES` table).
- Everything OMH says fleet is already ahead on (memory, router,
  calibration, plan graph, evals, guards).

## Sequence

**M — mockups (this plan's first gate; nothing below M1 starts before the
owner reads the set).** `docs/plans/mockups/2026-11-22-harness-redesign/`,
per house-mockups-first: draw first in the app's own tokens, measure at real
size in both themes, read the PNGs, **the owner gates before code**.

- M1 — the design system: tokens, type ladder, components, states, motion.
- M2 — the shell: rail anatomy, drawer anatomy, palette, responsive.
- M3 — the nine lens surfaces, one screen each with real-looking data.
- M4 — the six advanced surfaces.
- M5 — the bot PWA (phone) + iPad split view + the mini console.
- M6 — measurement + reads over the whole set; then the owner's gate.

**I — implementation, per surface after the gate.** Each row: one surface,
its builder + CSS + harness pins + PWA viewport check, gate green, commit.
Order: shell + tokens first (everything re-conforms), then the lenses in
relevance order, then Advanced, then the bot. The ledger amendments (D1's
shadow law, the drawer as the reading surface, the OMH borrowables) append
`U`/`R` ids in the same commits as the rows that implement them.

## Acceptance criteria

- [x] M1 design-system mockup drawn, measured, read (both themes) — `screens/00-design-system.html`.
- [x] M2 shell mockup (rail + drawer + palette + responsive frames) — `screens/01-shell-navigation.html`.
- [x] M3 the nine lens surfaces, each one screen — `screens/10–18`.
- [x] M4 the six advanced surfaces, each one screen — `screens/20–27` (plus Rooms inside Conversations, `28`).
- [x] M5 bot phone + iPad + console mockups — `screens/30` (390×844), `31` (1180×820 landscape split), `32`.
- [x] M6 measurement + PNG reads: obscura CDP capture at real viewports in night and day (44 PNGs), in-page audit (overflow, type floor, contrast), and the reads. Defects the passes caught, all fixed the turn they were found: unconstrained icon SVGs (the rail painted as a giant blob); the desktop shell leaking into the phone screen (323 px overflow); `1fr` grid blowout against the drawer on eight screens (`minmax(0,1fr)`); the iPad drawer hijacked by the ≤1180 media rule and the split view starved at portrait (moved to 1180×820 landscape); sub-AA label tones (`--faint` → `--muted`); emoji glyph discs replaced with drawn keys. One capture-tool lesson recorded below.
- [ ] The owner's gate on the set before any implementation row starts.
- [ ] I-rows: each surface rebuilt with harness pins, gate green by exit code,
      PWA verified at 390 and 820 widths, committed the turn it lands.
- [ ] D7's hygiene pass run after every delivery in this plan, recorded in
      the row's note.

### The capture-tool lesson (the measurement needed the read, and the read needed a trustworthy capture)

obscura's headless Chromium (Chrome 145) renders `text-overflow: ellipsis`
wrongly when an atomic inline (an inline-flex chip) follows the text on the
same line: it truncates the WORD and paints an ellipsis even when the content
fits the box by 500 px ("fleet" painted as "fle …", while longer words
painted whole). Proven with a minimal repro in `tmp/redesign-mockup-tools/`;
the DOM measurements agreed the box was wide while the paint disagreed. The
mockup CSS drops `text-overflow: ellipsis` (hidden overflow, no dots) so the
captures are truthful; the production implementation keeps the real engine's
correct behavior. Three further capture findings: obscura's CDP needs
`--allow-file-access` for `file://`, exposes one lazily-created page target
(`Target.createTarget`, not `/json/new`), and the first paint lays out at the
pre-override viewport — a forced re-raster before `captureScreenshot`
prevents stale tiles.

### Amendment, 2026-11-22 (the owner's read: the iPad mockup was mangled; both orientations are the requirement)

Reworked `screens/31-bot-ipad.html` as one app surface that adapts:

- **Landscape (≥1024)** — three panes: sidebar 250 | conversation | receipts
  320, single hairline seams, no internal topbar borders (the first version's
  column seams and the switcher overlapping the drawer's action row were the
  mangling).
- **Portrait (<1024)** — two panes: sidebar 240 | conversation, and the
  receipts pane becomes a slide-over (absolute within the pad, 220 ms,
  scrim + backdrop, opened by the header's **receipts** press, closed by the
  ✕ or the backdrop). Below 640 the sidebar folds away entirely.
- The theme switcher seats itself in the sidebar's foot (`data-theme-switch=
  "inline-sidebar"`), ending every overlap.
- Captured in BOTH orientations per theme (`31-bot-ipad-{night,day}-{1180,820}.png`),
  zero overflow measured in all four.

**Second capture-tool finding, recorded with its evidence:** obscura's
headless Chromium composites an overlay pane's layer at the scrim's alpha —
the open slide-over paints as a ~55% blend of pane and conversation even
though the DOM is verified correct (computed background `rgb(17,20,26)`
opaque, rect `[420,0,400,1180]`, z 50 above the scrim's z 45; probed live).
Tried transition-off, forced re-raster, compositor warm-up, absolute instead
of fixed positioning — the blend persists, so it is the build's compositing
bug, not the design's. The gallery's portrait capture shows the honest closed
default; the interactive page demonstrates the slide-over live, and the DOM
probe is the open state's evidence. `position: absolute` (scoped to the pad,
not the viewport) is kept as the slide-over's implementation — the better
PWA scoping regardless.

## Out of scope

- No release cut without the owner's explicit go-ahead (standing rule).
- No vector/shape editing in Design (the surface's own ruling).
- No new backend seams invented for mockups; where a screen needs a read the
  face does not make, the mockup says so and the implementation row adds it
  through the existing bridge.
