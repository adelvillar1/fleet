---
name: measuring-a-mockup-by-reading-the-png
description: "feedback 2026-11-04 — with images readable, the mockup pass is BOTH a screenshot read at size and a DOM measurement over CDP; each caught a real defect the other could not, plus the Chrome traps that make a good page render wrong"
type: feedback
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

Huddle UX Phase 0 ([[huddles-ux-redesign-2026-11-04]]). The two methods are
complements, not substitutes — and this session proved each one's blind spot:

**The read found what measurement cannot.** The first render shipped a floor
of filled black squares down the rail (the icons' SVG paths had no
`fill:none; stroke`), member names jammed against their voices
(`designergrounds every claim`), and three elements bunched inside what
should have been one spaced row. Zero overflow, zero clipping, all sizes
legal — the layout measured perfectly and read as broken.

**The measurement found what the read could not.** One mark (`clerk`, the
lane-0 tint) sat at **4.3:1** in the `contrast` theme; every other mark was
5.4+. A single 30px disc, invisible by eye, and it is exactly what
[[huddle-mockups-measured-not-read]] had to approximate when the model could
not see images.

**The measurement pass, concretely:** headless Chrome with
`--remote-debugging-port`, then over CDP `Page.navigate` +
`Runtime.evaluate` reading `documentElement.scrollWidth` vs the viewport
(overflow), every element's `rect.right > vw` (past the edge), text nodes
whose `scrollWidth > clientWidth` without `text-overflow: ellipsis`
(clipping), then `getBoundingClientRect().height` for rows / primary
buttons / nav rows. **Contrast needs a canvas, not a regex**: computed
`color` arrives as `color(srgb 0.18 0.37 0.82)`, so `[\\d.]+` parses the
wrong numbers (every mark read 1:1). Paint the colour into a 1×1 canvas and
read `getImageData` instead. **2026-11-04, the canvas is not enough**: a
1×1 canvas filled with a `color-mix()` computed value also returned black
(ink measured 2.5:1 and 1.6:1 where every mark was visibly fine), so the
robust parse is explicit — split `color(srgb r g b)` and multiply the
first three numbers by 255 (after skipping the colour-space token),
else take the first three channel numbers; walk the ancestors for the
first non-transparent `backgroundColor` (the surface, not the page).
Details [[every-rail-icon-audited-2026-11-04]].

**One page, one script.** The measurement code lives in a page-context JS
file and the driver `fs.readFileSync`s it — do NOT embed it in a Node
template string: one unescaped backtick or `\\d` inside it becomes a syntax
error, `Runtime.evaluate` returns `undefined`, and the whole run dies on
`JSON.parse`. And a multi-page sheet renders every page's sections on each
page unless the renderers guard on their own mount point
(`if (!document.getElementById('grid')) return;`).

**Chrome traps that make a good page render wrong (all hit this session):**

- `--screenshot` writes relative to Chrome's own `--user-data-dir`, not the
  shell's cwd — and a profile that has run before can serve a **cached
  page**, so a re-render "succeeds" and the PNG is byte-identical to the old
  one. Use a fresh `--user-data-dir` per run and an ABSOLUTE `--screenshot`
  path.
- A class used twice killed the whole page: `.shell.pane` (the four-column
  layout) and `.pane` (the aside) made the shell itself render as a flex
  pane. Rename one (`.pane-open`).
- A max-width on the screen does not constrain a child whose own child
  overflows: a head block inherits its widest row, so the text sits far
  left of the transcript below. Wrap the head in an inner element carrying
  the same measure as `.stream`.
- Spacing sets negative margins: `.member + .member { padding-left: 18px }`
  pulled the following item LEFT. When a gap looks wrong, read the margin.
- **A stylesheet without `[hidden] { display: none !important }` renders
  every "hidden" row.** The icon sheet's folded rail drew all fifteen rows
  under each group label until the rule went into `mockup.css` (the face
  has it at `app.css:272`; a copied sheet must copy that line too). A
  folded-state screen is the one that proves it.
- `sips --cropToHeightWidth` silently writes a copy when the source and
  destination differ — crop into `/tmp` and read that, or you review the
  uncropped image and call it cropped.
- Measurement must filter to VISIBLE elements before comparing sizes: a
  hidden row still carries a 0×0 svg, so a "sizes all 17px" check fails on
  a correct folded rail until `getBoundingClientRect().width > 0` guards it.

**2026-11-04, the desk mockups — three more traps, two of them in the
measurement harness itself:**

- **A shared mockup.css owns class names.** The desk screens imported
  the huddles sheet verbatim, whose `.field` (the huddles composer's
  flex field) collided with the desk composer's stacked fields — the
  fields rendered inline and BROKEN, and the DOM measurement called it
  clean (overflow 0, clipping 0). The read caught it. When a screen
  imports a shared sheet, prefix the screen's own classes
  (`.dfield`, `.dpane`, `.drail`, `.dmain`) so nothing collides.
- **The measurement copies went unstyled three ways, all silent.**
  /tmp copies of the screens loaded `mockup.css` by relative href
  (absent in /tmp → unstyled), an absolute `file://` href to another
  tree (blocked), and a stylesheet-loading race (the measure script ran
  before the sheet applied). The robust recipe: **inline the CSS into
  the measure copy** and **defer the measurement to
  `window.addEventListener('load')`** — and put the stylesheet copy
  beside the copies as a fallback. Unstyled measurement reads plausible
  garbage (every contrast 1.0, "sizes" from UA defaults), so one
  styled sanity anchor (a known chip's computed background) belongs in
  the harness output.
- **`--dump-dom` reads the runtime title.** The measurement wrote its
  result into `document.title` and grep'd `<title>MEASURE …</title>`
  from the dump — 20 screen-theme combos per pass, no CDP port needed
  (the kernel-side driver failed: no `fetch` global in that Node).

**2026-11-05, the goal-task-pane mockups — two more harness facts:**

- **`String(canvas.fillStyle)` normalizes OPAQUE colours to hex** (`#181a1d`),
  not `rgb()` — a contrast parser that only accepts strings starting with
  `rgb` returns null for every colour and the whole contrast block reads
  empty (the cell loop still reported ok, so nothing failed loudly). Parse
  `#rgb`/`#rrggbb`/`#rrggbbaa` AND `rgb()/rgba()`. Also: `pairs` keyed per
  selector recorded only the FIRST matched element's variant — per-status
  contrast needs per-element keys, and the worst-case pair (lightest ink on
  lightest ground) is the one to reason from when variants collapse.
- **Node 24 has a global `WebSocket`** — a zero-dependency CDP driver
  (`/json/new` via PUT, `Emulation.setDeviceMetricsOverride`,
  `Page.captureScreenshot`) needs no ws library; the goal-pane measure script
  (`tmp/goal-pane-mockup/measure.mjs`) drove all 48 cells over one Chrome
  launch. And one eye-read per state in a light AND a dark theme (not every
  cell) is the workable read budget — it is what caught the done state's
  process card showing `act` as current, a lie invisible to all 48
  measurements.

**Why:** "measured green" and "looks right" are separate claims and each
hides the other's class of defect. **How to apply:** for any mockup or
state-dependent screen, do BOTH — read every PNG at full size in the theme
you will ship (and one dark), and run the DOM measurement across all themes
— then re-read after each fix, because a CSS change moves what you already
approved. Related: [[a-visual-coherence-check-is-reading-the-png]],
[[huddle-mockups-measured-not-read]], [[chrome-screenshot-fires-at-load]].

**The goal-task-pane mockup's read budget (2026-11-05):** 48 cells
(4 states × 3 pane widths × 4 themes) measured clean, then NINE cells
eye-read — every state in a light AND a dark theme, both width extremes —
rather than all 48. That budget is what caught the done state's process
card still showing `act` as current (a lie invisible to all 48
measurements), and it is the workable read shape: cover the state matrix
at the extremes, not every cell.

**2026-11-07, the component-catalog sheet — the same shape, and two more
harness facts.** The sheet (16 catalog kinds + one sample spec rendered end to
end) was measured in all four themes at 1440×900: no overflow, no clipped
text, ink ≥ 4.5:1, and the PNGs read in a light and a dark theme. The
measurement caught one real defect the eye could not (the primary button's
white-on-accent failing AA in the dark themes) and one false flood (132
phantom low-contrast failures from a parser that dropped the alpha —
[[a-contrast-measurement-needs-the-alpha-channel]]). Two harness facts worth
carrying:

- **A scroll container is not a defect.** A `<pre>` with `overflow: auto` and a
  long JSON spec reports `scrollWidth > clientWidth` on every theme; the
  overflow/clipping pass must skip elements whose computed `overflow` is
  `auto`/`hidden`/`scroll`, or it reports the spec block as broken four times.
- **The mockup's own renderer is a stand-in, and the sheet is the contract.**
  The sheet's JS renders the sample spec with its own `cat-*` classes so the
  loop is visible before any code; the Rust renderer emits the same classes.
  The sheet is therefore the render contract, not a preview of a different
  thing — and the `on-accent` token the measurement forced (white on light,
  dark on the bright accents, the face's `.dk-btn.primary` pattern) belongs in
  the renderer's stylesheet too.
