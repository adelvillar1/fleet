# The design canvas: a board of frames fleet generates itself

**2026-10-31** · the day's fourth session. The owner's ask — *integrate
design-canvas as a fully native part of the app, same fleet design
language, not a mere bolt-on* — was surveyed, planned, and implemented in
one session: a design-canvas port that takes the source's canvas surface
and its contract, and swaps every SaaS leg for one fleet already owns.
The face gained a `design` stage (a pan/zoom board of frames), fleet
gained the `frame` and `review_frame` tools plus a `designer` persona,
and the canvas is now the destination of any run's design artifacts.
Gate green at **1,866 tests / 0 failed** and the face harness at **543
checks across 55 cases**; ledger clean at **385 rows, 37 open** (KI-385
filed the same day).

## The plan, and what the survey changed

`docs/plans/2026-10-31-design-canvas-native.md` — **completed**, D1–D10
walked against fresh evidence including the live proofs.

The survey at HEAD `fff86bb` had one finding that reframed everything:
**design-canvas is not a vector editor.** Its atomic object is an
opaque, self-contained HTML page in a sandboxed iframe on a pan/zoom
board — no shapes, layers or resize handles — so the portable core is
~925 lines of Next-free React canvas plus the `Frame`/`TweakSpec`
contract, `injectTweaks`, the a11y scan and two proven-valid fixtures.
The other **76 API route files and 27 Prisma models** are the SaaS
platform, and the owner's "disregard all commercial" put them out of
scope in one sentence.

The plan's own amendment (`bfa769a`) named the **Rust/JS boundary**
component by component after the owner asked which parts were worth
turning into Rust: Rust owns what must be the same bytes wherever used
and what touches the home (the document, the injection, the token
exports, the scan, the validation, the ZIP, the preview's CSP); JS owns
what needs a DOM, a pointer or pixels (the viewport, the frames, the
pins, the panels). Two facts settled it — `a11yscan.ts` is a regex
scanner, so no HTML parser crate was needed, and the browser lane's
`Runtime.evaluate` is the path to a rendered scan later.

## Commits

| Hash | What |
|---|---|
| `b788d39` | the plan: the board ported, the SaaS legs swapped |
| `bfa769a` | the assessment's amendment: the Rust/JS boundary, named |
| `04abf41` | the landing: S0–S4, 22 files, the canvas module and the stage |
| `0630df5` | the live-proof fixes: both dialects accepted, fresh boards fit |
| `5612619` | the owner's steer: design directly under conversation |

## What landed

**The document** — `src/canvas.rs`: frames in array order (which is the
z-order), each an opaque page with its token spec, pins and version
snapshots, at `<home>/canvas/<slug>/project.json` with atomic writes.
Caps enforced by name: 512 KiB of HTML, 20 versions, 10 reviews. A fresh
home adopts the bundled sample board (the source's two fixtures).

**The stage** — `#/design`: the head with the board picker and counts,
the instrument strip (zoom out / fit / zoom in / grid / pin mode / the
zoom readout), the viewport, the brief composer. The world under one
`translate · scale` transform; zoom toward the cursor (0.05–2.5),
space/middle-drag pan, fit, title-bar drag with integer snap, arrow
nudge (16/1), **persisted** delete (the source's client-only delete, not
ported), duplicate, the three viewport presets. A board seen for the
first time fits its frames. Zoom and pan are remembered per board.

**The transforms, one implementation each** — `inject_tweaks` is the one
sanctioned frame mutation: the live `srcdoc`, the CSP preview route, the
HTML export, the ZIP and the PNG source all run it, and the face reaches
it through the stateless `POST /api/design/inject` so no second
implementation exists. Values are escaped; a unit belongs to a length,
never to a colour (a bug the unit test caught — `#ff0000rem`).
`extract_root_tokens` feeds `tokens.css`/`tokens.json`; `a11y_scan` is a
scanner (the page's own token contrast plus alt/label/focus/viewport
checks), never a parser.

**The generation leg — the native win** — `frame` puts a validated page
on a board (a `frame_id` refines, snapshotting the previous page) and
`review_frame` stores a structured review. `roles/designer.md` carries
the doctrine ported from the source's prompts. The composer starts an
**ordinary fleet run** whose task names the board and asks for `frame` —
so the canvas repaints as results land and the run appears in
`#/sessions` with its log. Any run can emit a frame: the canvas is
fleet's destination for designed output, not a separate app.

**Review, versions, exports** — the scan facts ride every read and render
beside a stored review, with the ask that runs browse → screenshot →
vision → `review_frame`. A restore is itself a version. Exports write
HTML, tokens, the ZIP packet (`deck.rs`'s zip) and **PNG through the
browser lane** — live-proven at 64,379 bytes against the preview route
on a validated loopback origin.

## The live proofs (and the two defects they caught)

- The sample board adopted on first read: two frames, four tweaks each,
  scan **3/4** — the honest failure being *no `:focus` style* in the
  fixtures' pages.
- The PNG export through the browser lane: **64,379 bytes**, path
  returned.
- A Chromium render at 1440×900: the stage in fleet's chrome (rail,
  head, instrument, dotted world, composer), both pages framed and
  legible, **zero page errors**; the re-render fitted at 40% after the
  first-view fix.
- **Caught by the live read, invisible to the tests**: the fixtures
  carry the source's camelCase (`type`, `cssVar`, `tweakSpec`) and the
  first reader refused them — the first-run board would have opened
  *empty* with every unit test green. Serde aliases accept both
  dialects; `the_bundled_sample_board_parses` pins the bundled file.
- **Caught by the first render**: a board opened at a default transform
  with its second frame half off-viewport; a board seen for the first
  time now fits its frames.

## Two bounds moved (consequences, recorded)

The design tools took the native surface to **9,575 definition tokens**
against 9,000 and to **30 declarations**; 30 native plus a joining
lane's four then crossed `MAX_VISIBLE_TOOLS = 32`, which meant a lane's
own tools were deferred the moment it joined — the one thing the tier
must not do. So `MAX_DEFINITION_TOKENS` → **10,000** and
`MAX_VISIBLE_TOOLS` → **36** (30 native, room for one six-tool lane),
each with the reasoning in its doc comment. Both crossings were caught
by pins the surface-curation plan left behind — exactly what they are
for.

## Files changed

- **Backend**: `src/canvas.rs` (new — the document and the transforms),
  `src/gui/bridge.rs` (design read/write/inject/preview/export/PNG),
  `src/gui/server.rs` (five routes), `src/tools.rs` (`frame`,
  `review_frame`), `src/agent.rs` (declarations, plan-mode refusals, the
  handlers, the two bounds), `src/deck.rs` (the zip shared),
  `src/settings.rs` (the atomic writer shared), `src/lib.rs`.
- **Frontend**: `index.html` (the stage), `app.js` (~700 lines: the
  canvas module, registries, reducer hook, boot wiring), `app.css`
  (the canvas region).
- **Assets**: `design-sample.json` (the fixtures, adopted on first
  run), `design-presets.json` (ten of the source's eleven curated
  presets — the eleventh's shape didn't match the extractor).
- **Role**: `roles/designer.md`.
- **Tests**: `tests/face/harness.mjs` (the PM-D case, 18 checks),
  `canvas::` unit pins, two integration pins, two bridge pins.
- **Docs**: `docs/features/design.md` (new), `TECHNICAL-DOCUMENTATION.md`
  (`canvas.rs`), `docs/features/gui.md` (the stage list),
  `docs/features/tools.md` (the moved bounds), `docs/known-issues.md`
  (KI-385).

## Doc updates applied

- `docs/features/design.md` — new: the surface, the document, the tools,
  the transforms, the routes, the limits.
- `TECHNICAL-DOCUMENTATION.md` — the `canvas.rs` entry and the tools
  family.
- `docs/features/gui.md` — `#/design` in the stage list.
- `docs/features/tools.md` — 36 tools / 10,000 tokens, the surface
  measured at 30 declarations / 9,575 tokens.

## Doc updates proposed (pending the owner's approval)

Two now: last session's providers passage is still unapplied, and this
one joins it.

### `FUNCTIONAL-SPECIFICATIONS.md` — the design surface (new passage)

```markdown
Open design from the rail and you land on a board: your pages, side by
side on a canvas you can pan and zoom. Write a brief under the board,
press generate, and pages appear on it as the run works. Drag a page by
its title bar to move it; open its card to move its colors and type
scale with sliders that change the page as you drag. Drop pins where
something is wrong and ask for the fix; ask the designer to review a
page and read the scores beside the page's own measured checks. Export
a page as HTML, as design tokens, as a hand-off packet, or as a PNG.
```

## CLAUDE.local.md changes

None this session.

## Open questions / next steps

- **The router: no new workload label.** The owner asked; the assessment
  is in the transcript — `frontend`'s own label text *is* site design,
  and its chain (`deepseek-flash → GLM → ollama`) is already the design
  chain, so a seventh label would split the fuzziest boundary and need a
  vendored-vocabulary change for a duplicate. The real levers are a
  per-run pin (the composer) and the execution head for variants. What
  would settle it: replay ~10 real design briefs through sys1 and read
  the labels — not yet run.
- **The review ask was never run live.** Every piece is unit-pinned
  (browse screenshot, `review_frame`, the review card), but no model run
  has walked browse → screenshot → vision → `review_frame` end to end.
- **A design run cannot see its own output.** The review is a second
  run; letting a design run screenshot its frame mid-loop and iterate is
  the natural next increment.
- **The `:focus` scan failure on the fixtures** is a true finding about
  the sample content — a sample board should pass its own scan.
- **KI-385** (the relay suite is load-sensitive) stands open; found while
  gating this landing, not caused by it.
- The providers plan's `FUNCTIONAL-SPECIFICATIONS.md` passage from the
  previous recap is still pending.

## Notes

- **The live read is a gate, not a formality.** Two defects — the
  camelCase refusal and the unfitted first view — were invisible to
  every unit test and to the harness (which pins structure and writes,
  never pixels). The 1440×900 render and one real `curl` against the
  reloaded face caught both in minutes.
- **Serde aliases for a foreign dialect.** `type`/`cssVar`/`tweakSpec`
  ride beside `kind`/`css_var`/`tweak_spec` on the canvas types: a frame
  written by either dialect lands, which matters because the model-facing
  tool schema and the source's fixtures speak differently.
- **Sweeps**: process registry 0 entries; cache sweep **51.0 → 51.6 GiB**.
- The app was rebuilt (release) and restarted; the face serves the canvas.
