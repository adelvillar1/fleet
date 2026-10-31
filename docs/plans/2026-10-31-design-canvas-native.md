---
status: completed
created: 2026-10-31
updated: 2026-10-31
slug: design-canvas-native
---

# Plan: the design canvas, native — a board of frames fleet generates itself

## Context

The owner's ask, 2026-10-31: *"design-canvas is a project that could be
very useful for fleet, disregard all commercial issues of that app,
let's figure out how to integrate it as a fully native part of the app,
it should use the same fleet design language and look and feel like part
of the app, not a mere bolt-on."*

**What design-canvas actually is** (surveyed at HEAD `fff86bb`): an
AI-assisted web-design canvas. The atomic object is **an opaque,
self-contained HTML page in a sandboxed iframe**, positioned on an
infinite pan/zoom board — *not* a vector editor, no shapes, no layers,
no resize handles (`components/canvas/CanvasStage.tsx:368-404`,
`components/canvas/FrameView.tsx:165-182`; `lib/types.ts:72-96`). The
board is a DOM `div` under `transform: translate(x,y) scale(zoom)` with
a CSS dot-grid; frames are `srcDoc` iframes with `pointerEvents: none`
and a title bar as the drag handle. Its value is the loop around that
board: brief → LLM generates complete HTML pages → compare/annotate/
tweak/review/export. Everything else in that repo — **76 API route
files, 27 Prisma models, 5 pure-function test files** — is the SaaS
platform: auth, ACLs, Stripe/credits/usage, publishing/hosting,
collaboration, admin, an MCP server.

**What is genuinely portable**, in the surveyor's own accounting: the
~925-line canvas surface (`CanvasStage` 425 + `FrameView` 404 +
`CanvasToolbar` 56 + `ModelBadge`), the `Frame`/`TweakSpec`/
`CommentPin` contract (`lib/types.ts`), the one sanctioned HTML
mutation path `lib/tweaks.ts` (108 lines, `injectTweaks` — pure), the
deterministic a11y scan `lib/a11yscan.ts` (182, pure), the hand-off
packet builder `lib/handoff.ts` (224), and the proven-valid frame
fixtures (`fixtures/sample-project/hero.json`: 2,446 bytes of real
HTML; `pricing.json`: 2,921). The canvas is **React-coupled but
Next-free** — no `next/*` import anywhere in `components/canvas/**` or
`lib/**` — and its persistence is a separate API surface, so nothing
about the board forces the platform.

**Fleet's face is the hard constraint that shapes this plan.** One Rust
binary serving hand-written HTML/CSS/JS embedded with `include_str!`
(`src/gui/server.rs:71-98`), content-addressed by `build.rs:1-56`, no
Node at runtime and no bundler — hard rule 6 (`CLAUDE.md:34`), and the
face is a **client of the CLI's writer, never a second writer**
(`src/gui/mod.rs:24-28`). So "native" cannot mean shipping the Next
app. It means: **port the canvas surface to vanilla JS, move its
document to a fleet home file behind the bridge, and swap every SaaS
leg for a leg fleet already owns.**

**The native swap, leg for leg:**

| design-canvas leg | fleet's leg |
|---|---|
| `lib/llm.ts` (1,299) + `lib/models.ts` (477) + `lib/sse.ts` — its own model registry, transport and stream | **A fleet run**: the bridge spawns the agent with a `designer` role; the model calls a new **`frame` tool**; the run streams on the existing `/events` SSE and lands in the session log (hard rule 5) |
| `lib/screenshot.ts` — Playwright Chromium singleton for PNG | **The `browse` tool's `screenshot` action** (`src/capabilities.rs:77-85`), the browser lane fleet already runs |
| `lib/zip.ts` + `lib/handoff.ts` — hand-rolled STORE zip | **`deck.rs`'s own `zip()`** (`src/deck.rs:1864`), already proven by `the_export_is_a_zip_a_reader_can_open` |
| Prisma/Postgres — 27 models, 25 migrations, all 72 routes | **One home document**: `<fleet-home>/canvas/project.json`, frames as an ordered array (array order *is* z-order, exactly as design-canvas renders), written through the bridge |
| Auth/sessions/ACLs/invites | The home is the boundary: loopback-only, one owner — the same ruling every fleet surface runs on |
| Stripe/credits/usage/entitlements | Deleted. No gates, no 402s — the commercial surface is out of scope by the owner's word |
| Publishing/hosting/analytics, client review links | Out of scope. A **local preview route** serves the tweaked frame with a CSP, which is what "share" needs on one machine |
| `lib/tweaks.ts`, `lib/tokens` | **Rust, one implementation** — the mutation path and the token exports must be byte-identical wherever used (live srcdoc, preview route, ZIP, PNG source) |
| `lib/a11yscan.ts` | **Rust, as a scanner** — the source is regex/string-based already (color parsers + `match`/`replace` over the HTML; `a11yscan.ts:1-40`), so no HTML parser is needed and none is added. Fidelity is a flag, not a certification |
| `lib/codegen.ts` | **Only if the TSX export is wanted**; if so, Rust (a pure string transform, server-side) |
| `lib/stylePresetLibrary` (148 curated + 673 generated) | Embedded as data, read by the composer's style chips |
| The 2,207-line workspace orchestrator | Rewritten: the bridge owns the document, the face owns the viewport |

**The Rust/JS boundary, as a rule.** Rust owns everything that must be
the same bytes wherever it is used, and everything that touches the
home: the document store, the tweak injection and token extraction,
the a11y scan, the frame validation and caps, the ZIP, the preview
route's CSP, the preset data, the design events. **JS owns everything
that needs a DOM, a pointer, or pixels**: the viewport transform and
pan/zoom/fit, the frame cards and their iframes, selection and
drag-move, the pins' placement and threads, the tweak panel's controls,
the composer, and the version list's rendering. A component that both
sides could compute goes to Rust when it must agree with an export and
to JS when it must agree with the screen — and the only component that
genuinely wanted both is the tweak injection, which is why it gets a
stateless endpoint (below) instead of a second implementation.

**The live-tweak round trip, named.** A slider drag cannot wait for a
document write, and the face must not reimplement `injectTweaks` (two
injections drift — the source's own docs warn it). So the transform is
one Rust function with two entry points: `POST /api/design/inject`
`{html, tweakSpec, tweakValues}` → the injected HTML, for live drags
(loopback, ~1 ms, debounced by the face's frame paint); and the stored
document for the preview route, the ZIP and the PNG source. The
response is bytes, never a home write.

**What the canvas keeps, because it is the valuable part:** infinite
pan/zoom with zoom-to-cursor, space/middle-drag pan, fit-to-bounds,
frame cards with viewport presets (1280×900, 768×1024, 390×844), drag
by the title bar with integer snap, keyboard nudge and delete, single
selection, comment pins as 0–1 fractions with threads and
pin-to-fix, **live token tweaks** (`injectTweaks` writes one
`:root{…}` style block — no LLM call), version snapshots with
restore, and per-frame copy/download HTML.

**The design-language boundary, named honestly.** Fleet's language
governs the **chrome**: the stage, the toolbar, the rails, the cards,
the pane, buttons, badges, the type scale and the alias tokens — all
fleet's own (`app.css:98-168`, `boardCard` `app.js:4371`, `sectionRail`
`app.js:15514`, `openDrawer` `app.js:7683`). The **frame** is the
artifact: an arbitrary HTML page the model wrote. That boundary already
exists in the face — the preview pane renders documents, PDFs and live
web pages in sandboxed frames today — so the canvas inherits it rather
than inventing it.

**The interaction infrastructure already in the face** (nothing new to
invent): the workspace-history DAG's zoom/pan/minimap — a pointer-driven
SVG plane with a scroll container, `historyZoom` clamped 0.25–2.0, and
a minimap whose pointerdown seeks (`app.js:11808-12285`); the pane
grip's pointer-capture + CSS-var + localStorage pattern
(`app.js:19193-19244`); the queue's drag/drop with `moveQueued`
(`app.js:2944-3028`); the SVG helpers `svg`/`svgPlot`/`dayChart`/
`donut` (`app.js:4348,5204,4503,4560`). The canvas copies those idioms.

## Decisions (the owner's, dated and quoted)

1. **2026-10-31 — the intent.** *"design-canvas is a project that could
   be very useful for fleet, disregard all commercial issues of that
   app, let's figure out how to integrate it as a fully native part of
   the app, it should use the same fleet design language and look and
   feel like part of the app, not a mere bolt-on."*
2. **The reading of "disregard commercial" this plan commits to** —
   named so it can be corrected in one sentence: **no Stripe, credits,
   subscriptions, usage metering, plan gates or 402 handling; no
   publishing/hosting/analytics; no multi-tenant auth, orgs, ACLs,
   invites or the admin console; no MCP server.** The single owner, the
   local home, and the loopback face are the whole security boundary.
3. **The reading of "fully native"** — the canvas is *fleet's own
   surface*: a stage in the face, a document in the home, generation by
   fleet's own models through fleet's own runs, files handed out by
   fleet's own exporters, and every chrome pixel on fleet's tokens.

## Approach

**Placement.** A new top-level stage `design` in the rail's `work`
group after chat/sessions (`ROUTES` `app.js:238-251`, `GROUPS`
`app.js:262-283`), rendering `<main class="stage stage-board"
id="panel-design">` with its own board (`index.html:672-675` is the
template). The stage body is a **canvas viewport** filling the board:
`.canvas-viewport` (the clipping container, `position:relative;
overflow:hidden`) holding `.canvas-world` under
`transform: translate(x,y) scale(z)`. Its head uses `surfaceHead` +
`surfaceStrip` (`app.js:5740,5751`) for the project name, frame count
and the zoom instrument — fleet chrome over a spatial surface, not a
card grid, because a card grid cannot pan.

**The document.** `<fleet-home>/canvas/<slug>/project.json`:

```json
{
  "version": 1,
  "name": "…",
  "created": "…",
  "frames": [
    { "id": "f-…", "name": "hero", "description": "…",
      "html": "<!doctype html>…", "x": 0, "y": 0, "w": 1280, "h": 900,
      "tweakSpec": [ { "key": "accent", "label": "accent", "type": "color",
                       "default": "#3b6cf6", "cssVar": "accent" } ],
      "tweakValues": { "accent": "#3b6cf6" },
      "model": "…", "genMeta": { "model": "…", "durationMs": 0 },
      "versions": [ { "id": "v-…", "html": "…", "tweakValues": {},
                      "source": "generate", "created": "…" } ],
      "pins": [ { "id": "p-…", "x": 0.5, "y": 0.25, "text": "…",
                  "resolvedFrameId": null, "replies": [] } ] }
  ]
}
```

Caps, named: **20 versions per frame**, **512 KiB of HTML per frame**
(refused by name past it), **10 reviews per frame**. The bridge reads and
writes it — `Bridge::design()`/`design_write()` (`bridge.rs:2016-2059`
is the template), routes `GET /api/design`, `POST /api/design` — and the
face never writes a file. The sample project ships from the fixtures
(`fixtures/sample-project/hero.json` + `pricing.json`, copied into
`src/gui/assets/design-sample.json` and adopted on first run), so a
fresh home has a board with something real on it.

**The generation leg — the native win, and the plan's spine.** The
composer's brief posts `POST /api/design/generate`; the bridge starts a
**fleet run** with the `designer` role (`roles/designer.md`, the
`presenter.md` frontmatter shape: name/description/tools/skills/shapes +
prose doctrine) whose instruction carries the brief, the style preset
and the frame's target viewport. The model calls **`frame`**, a new
tool in the base set (`tools.rs`): arguments `{name, html, tweakSpec?}`;
the handler validates (self-contained doctype, no external URLs, size
cap), appends the frame to the document, records the `artifact`-family
event, and returns the frame id — so the canvas repaints live over
`/events` while the run is still streaming, and the run itself lands in
the session archive like every other run. **Any fleet run can emit a
frame** — from the CLI, chat, or a cron job — and the canvas is where it
lands: the canvas is not a separate app, it is the destination of
fleet's design artifacts. The design doctrine (the
"complete, self-contained, token-driven HTML page" prompt rules) is
ported from design-canvas's `lib/llm.ts` into `roles/designer.md` — one
persona file, not a 1,299-line module.

**Tweaks and pins.** `injectTweaks` ports to Rust
(`src/canvas.rs::inject_tweaks`): insert/replace
`<style id="fleet-tweaks">:root{--accent:…}</style>`; `extract_root_tokens`
parses `:root` variables for the handoff's `tokens.css`; the tweak panel
renders sliders/selects/toggles/colors from `tweakSpec`, writing
`tweakValues` through the same document write. Pins are normalized 0–1
coordinates absolutely positioned over the frame card; a pin's thread
opens in the pane's object card (`openDrawer`), where replies and
"fix this pin" (a run whose instruction carries the pin's text and
frame) live.

**The visual proof is a real render, not a claim.** The canvas is a
visual deliverable, so acceptance runs the visual gate: render the
stage at 1440×900 via the browser lane and judge the screenshot (the
`docs/features/gui.md` idiom; the deck and face work already do this).
The harness covers structure and writes; the screenshot covers the
pixels.

## Slices

Each lands whole — code, checks, docs — and each is green on its own.

### S0 — the stage, the document, the sample board

`design` in `ROUTES`/`GROUPS`/`PANEL_EL`/`arrive` (`app.js:238-251,
262-283, 17372-17385, 17443-17480`); the stage markup beside the
calibration stage (`index.html:672-675`); `src/canvas.rs` with the
`Frame`/`TweakSpec`/`CommentPin` types and the document
read/write/validate; `Bridge::design`/`design_write` (`bridge.rs:2016`
template) + the two routes (`server.rs:1247` template); the sample
project adopted on first run; the face paints one read-only frame card
(sandboxed iframe, title bar, viewport size) from the document; the
harness case boots `#/design` over a stubbed document and asserts the
frame card, its iframe `srcdoc` carrying the HTML, and the head's
counts.

*Checks:* D1, D2.

### S1 — the canvas: pan, zoom, select, move

The viewport/world transform; wheel zoom-to-cursor (0.05–2.5, the
design-canvas bounds), space/middle-drag pan (pointer capture, the grip
pattern `app.js:19193`), fit-to-bounds with a 0.05 floor; single
selection by clicking a frame's title bar; drag-move by the title bar
with delta/zoom and integer rounding; keyboard nudge (16/1 with Shift),
delete (**persisted** — design-canvas's delete is client-only, a known
bug we do not port), duplicate (`x + w + 96`); the three viewport
presets; array order as z-order; a `design_geometry` write that
debounces like `settings_write` (the face writes the whole document on
settle). The zoom instrument in the head's strip.

*Checks:* D3, D4.

### S2 — tweaks, the preview route, pins

`inject_tweaks`/`extract_root_tokens` in `src/canvas.rs` with their
tests; the tweak panel (sliders/selects/toggles/colors over
`tweakSpec`) writing `tweakValues`; `GET /api/design/frame/<id>` serving
the tweaked HTML with a strict CSP and `nosniff` (the design-canvas
preview-route rule); pins: click-to-place in pin mode, the thread in the
pane's object card with replies and resolve, stored in the document;
"open in a tab" through the preview route.

*Checks:* D5, D6.

### S3 — generation through fleet's own run

The `frame` tool (`tools.rs`) with validation and its refusal texts;
`roles/designer.md` with the doctrine ported from `lib/llm.ts`'s
system prompts; `POST /api/design/generate` starting the run (the
`/api/run` path's own machinery, the role as the overlay); the
composer (brief textarea, style chips from the embedded preset data,
model select from the roster's catalog — the provider-selection surface
the providers plan already built); live repaint as `frame` events
arrive; the run visible in `#/sessions` with its log (hard rule 5). The
pin-to-fix and "refine this frame" flows ride the same seam.

*Checks:* D7, D8.

### S4 — review, versions, export

The a11y scan ported (`src/canvas.rs::a11y_scan`, the deterministic
checks: contrast pairs, alt text, focus order, viewport meta) shown as
facts beside the review; the AI review as a run using the `vision` tool
(the router's vision workload) writing a structured `review` into the
document, rendered in the pane as scores + strengths + fixes;
version snapshots on each generate/refine/edit (last 20), the history
panel with preview/restore and the token diff; export: copy/download
HTML, `tokens.css` + `tokens.json` from `extract_root_tokens`, the
hand-off ZIP via `deck.rs`'s `zip()`, and PNG via the `browse`
screenshot action; `send_file` for handing a frame to the owner.

*Checks:* D9, D10.

## Use cases

- [x] A: The owner opens `#/design`, sees the sample board, pans and
      zooms, drags a frame, and the position survives a reload.
- [x] B: The owner types a brief, picks a style and a model, and frames
      appear one by one on the board while the run streams — the run
      visible in `#/sessions`.
- [x] C: The owner opens a frame's tweaks, moves the accent slider, and
      the frame repaints live with no model call.
- [x] D: The owner drops a pin on a frame, writes "fix the spacing",
      and the fix run revises that frame as a new version.
- [x] E: The owner exports a frame and gets HTML, tokens and a PNG —
      the PNG rendered by the browser lane.

## Acceptance criteria

- [x] **D1 — the stage is a stage.** `#/design` renders one
      `panel-design` stage with the canvas viewport; the route appears
      in the rail under `work`; bare `#/design` resumes the last
      project. Pinned by the harness boot and the rail row.
- [x] **D2 — the document is the home's, behind the writer.** A frame
      written through `POST /api/design` lands in
      `<home>/canvas/<slug>/project.json`; the face never writes a
      file; an oversized frame (>512 KiB) is refused by name; the
      sample project adopts on first run. Pinned by a bridge test and a
      harness write.
- [x] **D3 — the canvas moves like a canvas.** Wheel zoom keeps the
      point under the cursor fixed; space/middle drag pans; fit frames
      the bounds; drag-move rounds to integers and divides by zoom;
      nudge steps 16/1. Pinned by harness checks over geometry writes.
- [x] **D4 — selection, delete and duplicate persist.** Delete removes
      the frame from the document (the design-canvas bug fixed, not
      ported); duplicate offsets by `x + w + 96`; a reload shows the
      document's truth. Pinned by harness checks reading the writes.
- [x] **D5 — tweaks are live and local.** `inject_tweaks` is the one
      mutation path: the panel's slider changes `:root` in the frame's
      `srcdoc` with no model call, and the same injection feeds the
      preview route and the exporters. Pinned by `inject_tweaks` unit
      tests (insert, replace, escape) and a harness check.
- [x] **D6 — pins are the owner's marks.** A pin lands at its
      normalized position, its thread opens in the pane's object card,
      replies persist, and resolve marks it. Pinned by a bridge
      round-trip and the harness.
- [x] **D7 — generation is fleet's own.** A brief starts a fleet run
      with the `designer` role; the `frame` tool appends the frame and
      the canvas repaints live; the run's log carries the brief and the
      frame call. Pinned by an integration test with the mock API and
      by the harness consuming a `design_frame` event.
- [x] **D8 — the run is the record.** A design run appears in
      `#/sessions` with its events; killing it mid-generate leaves the
      frames it already emitted. Pinned by the replay read and an
      interrupted-run case.
- [x] **D9 — versions and export are honest.** Each generate/refine
      snapshots a version (20 cap); restore is itself a version (the
      design-canvas rule); the ZIP opens as a reader's archive
      (`deck.rs`'s own pin), tokens round-trip, and the PNG is the
      browser lane's file. Pinned by unit tests and one live export.
- [x] **D10 — the visual gate and the docs.** fmt, clippy, build, test
      green by exit code; the harness green with the counts quoted; a
      1440×900 render of the canvas judged by the visual house bar
      (chrome on fleet's tokens, frames legible, nothing clipped); the
      feature doc and `TECHNICAL-DOCUMENTATION.md` updated.

## Files to be touched

**Backend (fleet):**
- `src/canvas.rs` — new: the `Frame`/`TweakSpec`/`CommentPin`/`Review`
  types, the project document read/write/validate with the caps,
  `inject_tweaks`, `extract_root_tokens`, `a11y_scan`
- `src/gui/bridge.rs` — `design`/`design_write`/`design_generate`, the
  frame preview route's document, the `design_frame` event
- `src/gui/server.rs` — `GET/POST /api/design`,
  `GET /api/design/frame/<id>` (CSP), `POST /api/design/generate`
- `src/tools.rs` — the `frame` tool declaration and handler
- `roles/designer.md` — new: the persona and the doctrine (ported from
  `lib/llm.ts`'s prompts)
- `src/gui/assets/design-sample.json` — the two fixtures, adopted on
  first run
- `src/gui/assets/design-presets.json` — the style presets, embedded

**Frontend:**
- `src/gui/assets/index.html` — the `panel-design` stage
- `src/gui/assets/app.js` — `ROUTES`/`GROUPS`/`PANEL_EL`/`arrive`; the
  canvas viewport and every interaction; the tweak panel; the composer;
  `readDesign`/`paintDesign`; the `design` reducer cases
- `src/gui/assets/app.css` — the canvas region, frame cards, pins, the
  toolbar (fleet tokens only)

**Tests:**
- `src/canvas.rs` module tests — the document, injection, tokens, scan,
  caps
- `tests/integration.rs` — the generate seam with the mock API
- `tests/face/harness.mjs` — the design case(s)

**Documentation:**
- `docs/features/design.md` — new: the surface, the document, the run
- `TECHNICAL-DOCUMENTATION.md` — the `canvas.rs` entry
- `docs/features/gui.md` — the stage list gains `design`

**Source (read-only):** `/Users/alejandrodelvillar/Projects/design-canvas`
at HEAD `fff86bb` — the canvas components, `lib/types.ts`,
`lib/tweaks.ts`, `lib/a11yscan.ts`, `lib/handoff.ts`, the fixtures, the
preset data, and the doctrine in `lib/llm.ts`. **Read HEAD, not the
working tree** (the dirty tree carries `/tmp` debug instrumentation).

## Out of scope

- **The commercial surface**: Stripe, credits, subscriptions, usage
  metering, entitlement gates, the upgrade UI, 402 handling.
- **Publishing, hosting, analytics, client review links, share
  tokens**: a one-machine app serves a frame from its preview route.
- **Multi-tenant auth, orgs, ACLs, invites, collaborators, the admin
  console, the MCP server, TOTP, API keys.**
- **Vector/figma-style editing**: design-canvas has no shape model, and
  this plan does not invent one. No layers, groups, resize handles,
  alignment guides, marquee or rotation.
- **Undo/redo** for geometry: design-canvas has none either. Version
  snapshots cover frame *content*; a geometry undo is its own later
  decision (the event log makes it tractable).
- **Realtime collaboration, CRDTs, websockets.**
- **The redesign/Firecrawl pipeline and brand kits**: named as later
  candidates, not built.
- **The landing page, onboarding flow and sample-project marketing
  copy** of the commercial app.

## Verification

**D1/D5** — the document and the injection:
```bash
cargo test --lib canvas
```
Expected: green — the round-trip, the cap refusal, insert/replace/escape,
the token extraction, the scan's facts.

**D7/D8** — the generation seam:
```bash
cargo test --test integration a_design_run_emits_a_frame
```
Expected: green over the mock API — the run carries the `designer` role,
the frame call appends, the run's log holds the call.

**D3/D4/D6** — the canvas:
```bash
node tests/face/harness.mjs
```
Expected: every design check green; the counts quoted in this plan.

**D10** — the gate and the pixels:
```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo build --release && cargo test
```
Expected: exit 0 each; then a 1440×900 render of `#/design` (the browser
lane's screenshot), read as the visual review — chrome on fleet's
tokens, a frame legible on the dot grid, nothing clipped at the edges.

## Linked artifacts

- `docs/features/gui.md` — the face's contract (no framework, embedded
  assets, hash stages, the SSE halves of one log); the `design` stage
  joins its stage list
- `docs/features/tools.md` — the `frame` tool sits in the built-in
  family beside `deck` and `imagine`
- `docs/plans/2026-10-31-provider-models-settings.md` — the catalog the
  composer's model select reads (landed)
- `src/deck.rs:1864` — the ZIP the handoff reuses
- `/Users/alejandrodelvillar/Projects/design-canvas/docs/architecture/design-system.md`
  — the canvas spec this plan ports (viewport math, the sanctioned
  mutation path)

## Risks

- **Model-generated HTML in an iframe is the attack surface.** The
  frame renders `sandbox="allow-scripts"` **without**
  `allow-same-origin` (design-canvas's own rule), the preview route
  sends a strict CSP and `nosniff`, the `frame` tool validates
  self-containment and a size cap, and the canvas never injects model
  HTML into the face's own document — only into an iframe's `srcdoc`.
- **Frame documents grow.** Caps (512 KiB HTML, 20 versions, 10 reviews)
  and one project document; a document past a size the bridge refuses
  names it.
- **The port drifts from the source.** The plan ports the *behaviors*
  the survey recorded with citations, not the files; the fixtures give
  a fixed reference frame to compare renders against.
- **The harness cannot paint an iframe.** The fake DOM asserts
  structure, `srcdoc` bytes and writes; the *pixels* are the visual
  gate's job (D10), which is why the plan names both gates.
- **A long generation stalls the canvas.** The run streams on the
  existing SSE with fleet's heartbeats and lag handling; the canvas
  paints frames as they land, so a killed run leaves its frames.

## Dependencies

- The face's stage machinery and primitives (landed): `ROUTES`,
  `sectionRail`, `boardCard`, `openDrawer`, the DAG's zoom/pan, the
  grip's pointer pattern.
- The browser lane (`browser_screenshot`, landed) for PNG.
- `deck.rs`'s `zip()` (landed) for the handoff.
- The `designer` role's skills: `slide-deck`'s tokens discipline is the
  nearest precedent; the doctrine port is the new content.

## Notes

- **What design-canvas gets right that this plan keeps**: the frame is
  an opaque page (so any design tool's output — a site, a slide, a
  component sheet — is one object), `injectTweaks` is the single
  mutation path (so preview, share and export can never disagree), and
  pins are normalized fractions (so a viewport change never moves
  them).
- **What the survey found that must not be copied**: the unpinned
  delete (`app/projects/[id]/page.tsx:1213-1225` — client-only; this
  plan's delete writes), the working tree's `/tmp` debug instrumentation
  (read `fff86bb`), the single-process in-memory rate limiting (no
  equivalent here — the home is one owner), and the `SiteAnalysis`
  table with no owner (not ported at all).
- **No HTML parser is added, and that is a decision.** The a11y scan is
  scanner-shaped in the source (regex color parsing over the extracted
  `:root` tokens plus `<img alt>`, label and viewport-meta checks) and
  ports as one — the codebase's own idiom for this class: a flag, not a
  certification. If a *rendered* scan is ever wanted (real
  `getComputedStyle` contrast, real focus order), the browser lane
  already exposes `Runtime.evaluate` internally
  (`services/browser/src/browser/__init__.py:294,307-314`), so the
  upgrade path is a lane tool, not a crate.
- **The one thing worth reconsidering later**: brand kits. A brand kit
  is a token set + doctrine snippet, which is exactly what a fleet
  `frame` tool's second argument could carry — but it is a data
  decision, not part of this build.

## Landed 2026-10-31

S0: `src/canvas.rs` (the `Frame`/`TweakSpec`/`Pin`/`Version` types, the
project document with atomic writes, the caps — 512 KiB HTML, 20
versions, 10 reviews, refused by name), the sample board from the
source's two fixtures (adopted on a home with no canvas), the
`Bridge::design`/`design_write`/`design_inject`/`design_frame_html`/
`design_export` methods, and the five routes (read, write, inject,
preview under CSP, export). The face: the `design` stage, `ROUTES`/
`GROUPS`/`PANEL_EL`/arrive entries, the read/paint, and the frame cards
rendering each page in a sandboxed `srcdoc` iframe.

S1: the viewport under one transform — wheel zoom toward the cursor
(0.05–2.5), space/middle drag pan, fit-to-bounds, title-bar drag with
delta/zoom and integer rounding, click-to-select, arrow nudge (16/1),
**persisted** delete and duplicate (the source's client-only delete, not
ported), the three viewport presets, array order as z-order — and each
gesture writing through the design writer. The zoom and pan are
remembered per board.

S2: `inject_tweaks` as the one mutation (insert/replace, escaped values —
the unit rule fixed when a color took a `rem` suffix), the stateless
`/api/design/inject` the live tweak panel rides, the CSP preview route,
pins (normalized marks, threads in the pane, reply and resolve), and the
token extraction feeding `tokens.css`/`tokens.json`.

S3: the `frame` tool (validation, the refine snapshot, the board the
model names) and the `review_frame` tool; `roles/designer.md` with the
doctrine ported from the source's prompts; the brief composer starting
an ordinary fleet run whose task carries the board and asks for `frame`;
the canvas repainting as frame results land. Proven by
`a_design_run_emits_a_frame` (the frame lands, the log carries the call)
and `the_frame_tool_refuses_a_board_it_cannot_find`.

S4: the a11y scan (a scanner: the page's own token contrast plus
alt/label/focus/viewport checks), shipped per frame in the read and
rendered beside the review; the review card with the scan facts, the
stored review, and the ask that runs browse → screenshot → vision →
`review_frame`; versions with restore-is-itself-a-version; the exports
(HTML, tokens, ZIP through `deck.rs`'s zip, and the PNG through the
browser lane, which the bridge spawns against the preview route on the
face's own validated loopback origin).

**Two bounds moved, and the moves are recorded because they are
consequences, not tuning:** the design tools took the native surface to
**9,575 definition tokens** against 9,000, so `MAX_DEFINITION_TOKENS`
moved to **10,000** (its own doc's rule — the ceiling moves with the
count bound) and to **30 declarations**; and 30 native + a joining lane's
four crossed **MAX_VISIBLE_TOOLS = 32**, which meant a lane's own tools
were deferred the moment it joined — so the count bound moved to **36**
(30 native, room for one six-tool lane). Both crossings were caught by
the pins the surface-curation plan left behind
(`the_native_surface_fits_its_own_bounds`,
`a_full_run_calls_the_real_sem1_lane_through_the_mock_api`), which is
exactly what they are for.

New KI: **KI-385** — the relay suite is load-sensitive (two loopback
tests fail intermittently under a full parallel run; green alone and on a
clean full run), found while gating this landing. Not caused by it.

Gate, read from the runs: fmt 0, clippy `-D warnings` 0, build 0,
**suite 1,866 passed / 0 failed**, the face harness **543 checks across
55 cases**.

**The live proofs (against the reloaded face, new binary):**

- **The sample board adopted** — `GET /api/design` answered
  `sample-board`, two frames, each with four tweaks and its scan facts
  (a11y 3/4 on both, the honest failure being **no `:focus` style** in
  the fixtures' pages — a real finding about the sample content, not the
  scanner).
- **The PNG export through the browser lane** —
  `POST /api/design/export {kind: "png"}` answered
  **64,379 bytes** at
  `~/.fleet/canvas/sample-board/exports/sample-landing-hero-png-20261031T171436Z.png`:
  the lane opened the preview route under its CSP and captured it.
- **The visual gate** — Chromium at 1440×900 on `#/design`: the stage
  renders in fleet's chrome (the rail with `design` under `work`, the
  head with the board picker and `new board`, the instrument strip, the
  dotted world, the composer), both fixture pages framed and legible,
  **zero page errors**. The render exposed one first-run roughness —
  the board opened at a default transform with the second frame half
  off-viewport — so a board seen for the first time now **fits its
  frames** (`designFit` on the first paint of an unremembered view);
  the re-render shows both frames fitted at 40%.

**A defect the live read caught that the tests had not**: the sample
fixtures carry the source's camelCase spellings (`type`, `cssVar`,
`tweakSpec`), which the first reader refused — the surface would have
shown an empty board on first run while every unit test passed. Fixed
with serde aliases (both dialects land) and pinned by
`the_bundled_sample_board_parses`, which parses the bundled file and
adopts it on a scratch home.
