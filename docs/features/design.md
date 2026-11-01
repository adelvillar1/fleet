# The design canvas

Fleet's own board of designed pages. A **frame** is one complete,
self-contained HTML page — the artifact a design run produces — and the
canvas is where frames land, get tweaked, pinned, reviewed and handed
off. It was ported from the design-canvas project; what came over is the
canvas surface and its contract, and what stayed behind is that app's
SaaS shell (billing, accounts, publishing, collaboration). The reasoning
lives in `docs/plans/2026-10-31-design-canvas-native.md`.

## The surface

The canvas sits directly under the conversation in the rail's work
group — it is work the owner opens the way they open a chat, not a
listing to browse after one.

`#/design` is a split stage like settings and calibration: the **boards
rail** on the left — the same `sectionRail` every split page renders, one
row per board with its name, its frame count and the open one marked,
the fold remembered — and the canvas filling the body: a head (the
board's name and count, **new board**), an instrument strip (zoom out /
fit / zoom in / grid / pin mode / the zoom readout), the **viewport**, and
**the conversation** — a chat panel beside the board. The rail's column is 216px rather
than the sections' 188 — the rows are the same rows, but their names are
the owner's board names, and a clipped board name is a board you cannot
tell apart. The viewport is a clipping box holding a world under one CSS
transform — `translate(x,y) scale(z)` — and the world holds the frame
cards. Fleet's tokens, cards and buttons are the chrome; the frame inside
its sandboxed iframe is the artifact, which is the same boundary the
preview pane already draws for documents and web pages.

Gestures, all vanilla JS on the face's own pointer idioms:

- **Wheel** zooms toward the cursor (0.05–2.5).
- **Drag the empty board, or space/middle-drag anywhere**, pans.
- **Fit** frames the board's bounds into the viewport.
- **Drag a title bar** moves its frame: the delta divides by zoom and
  rounds to whole world units, then the position is written.
- **Click a title bar** selects; arrows nudge 16 (1 with Shift).
- **Pin mode**, then click a frame body, and the pin lands at that
  normalized point.
- The zoom and pan are remembered per board in `localStorage`.

Every gesture that changes the document **writes through the design
writer**. The board file is the truth; the view is a projection of it.

## The board's conversation

The panel beside the canvas is the board's chat, and it is part of the
document: the turns live in `project.json`, so a reload, the CLI and a
cron job read the same conversation. Each turn carries the frames it
landed as **chips**; a chip's press selects its page on the board and
fits the view. The composer's **verbs** follow the selection — *design*
(one or more new frames), *refine* (a new version of the selected
frame), *another direction* (a fresh frame beside it), *apply the
review's fixes* (when the frame has a review), and *ask* (a question
about the board, which writes nothing).

The composer carries a **directions** count: a brief can ask for up to
four clearly different pages, one `frame` per direction, in one run.
The frame's **size** follows the brief — `canvas::frame_size_for` reads
a "mobile" brief as 390×844, a tablet one as 768×1024, and everything
else as the desktop 1280×900 — and the task tells the model which
viewport word to pass, so the page lands at the size the request meant.

**Every task is composed in Rust.** The face asks the writer for the
task (`task` op) and starts the run with exactly what came back, so the
composer, the CLI and a cron job ask for the same thing in the same
words. The composed task carries the **board's own inventory** — each
frame's name, id, size, model, open pins and review state — so a
question about the board is answered from the facts rather than by
exploring the console with tools, and a refine knows what it is
refining. A review's fix lines and a frame's open pins ride the task
too, and a *refine* names the `frame_id` so the page keeps its identity.

A run started from the board raises its **approval seat on this page**
as well as on the conversation's: the design panel carries the same seat
with the same decisions, because an approval the owner cannot see is the
KI-380 class of defect. When the run ends, the face records the
designer's turn — its closing words and the frames it landed — off the
board it re-reads. A tab closed mid-run therefore keeps the frames (the
`frame` tool wrote them) but loses that closing turn; the conversation
answers on the next visit.

## The document

`<fleet-home>/canvas/<slug>/project.json`:

```json
{
  "version": 1, "name": "Sample board",
  "created": "…", "updated": "…",
  "frames": [
    { "id": "f-…", "name": "hero", "description": "",
      "html": "<!doctype html>…", "x": 0, "y": 0, "w": 1280, "h": 900,
      "tweak_spec": [ { "key": "accent", "label": "accent", "kind": "color",
                        "default": "#3b6cf6", "css_var": "accent" } ],
      "tweak_values": { "accent": "#3b6cf6" },
      "model": "glm",
      "versions": [ { "id": "v-…", "name": "hero", "html": "…",
                      "tweak_values": {}, "source": "generate", "created": "…" } ],
      "pins": [ { "id": "p-…", "x": 0.5, "y": 0.25, "text": "…",
                  "source": "owner", "resolved_frame_id": null, "replies": [] } ],
      "review": { "kind": "vision", "scores": {}, "summary": "…",
                  "strengths": [], "fixes": [], "history": [] },
      "created": "…", "updated": "…" }
  ]
}
```

**Array order is z-order** — frames render in array order, as the source
did. Pins are normalized 0–1 of the frame's own size, so a viewport
switch never moves them. Caps, enforced by the writer: **512 KiB** of HTML
per frame, **20 versions** per frame, **10 reviews** kept in a frame's
history. A fresh home adopts the bundled sample board (the two
proven-valid fixture frames from the source project).

## The tools

Two model-facing tools carry the canvas, both available to any run:

- **`frame`** — `{project, name, html, frame_id?, tweak_spec?}`. The page
  is validated before it lands (a complete document, under the cap) and
  the frame id comes back. With `frame_id` the call **refines** that
  frame instead: the previous page is snapshotted as a version. This is
  how a design run fills the board — the canvas is the destination of
  fleet's design artifacts, not a separate app.
- **`review_frame`** — `{project, frame_id, scores?, summary?, strengths?,
  fixes?}`. Stores a structured review on the frame; the canvas shows it
  beside its own deterministic checks.

The **`designer` role** (`roles/designer.md`) is the persona that uses
them: a frame is one complete page opened by its conclusion, the token
system comes first, and refining means keeping the frame's identity.

## Transforms, one implementation each

- **Tweak injection** (`canvas::inject_tweaks`) is the single sanctioned
  frame mutation: it inserts (or replaces) `<style id="fleet-tweaks">
  :root{…}</style>` from the frame's spec and current values. The live
  `srcdoc`, the preview route, the HTML export, the ZIP and the PNG
  source all run it. The face gets live values from
  `POST /api/design/inject` — the same function, stateless — so it never
  reimplements the transform. Values are escaped: a hostile variant can
  write nothing into the sheet.
- **Token extraction** (`extract_root_tokens`) parses the page's `:root`
  block into the hand-off's `tokens.css` and W3C-draft `tokens.json`.
- **The a11y scan** (`a11y_scan`) is a **scanner**, not a parser: it
  measures the page's own token contrast pairs (WCAG maths on the
  extracted values) and checks `<img>` alt text, control labelling, a
  `:focus` style and the viewport meta. Its fidelity is a flag, not a
  certification — every check names what it read.

## Routes

- `GET /api/design[?project=<slug>]` — the boards, the active board with
  its frames (each frame's `html` already injected, `html_raw` before
  overrides, plus `a11y`), the presets, the caps.
- `POST /api/design` — the one writer: `create`, `rename`,
  `frame-update` (geometry, name, description, tweak values, z),
  `frame-delete`, `frame-duplicate`, `version-restore`, `pin-add`,
  `pin-reply`, `pin-resolve`, `review-put`.
- `POST /api/design/inject` — stateless injection: `{html, tweak_spec,
  tweak_values}` → `{html}`.
- `GET /api/design/frame/<slug>/<id>` — the preview: the stored page with
  its tweaks, served under a CSP (`default-src 'none'`, inline styles
  only, no scripts, no frame ancestors, no form posts) and `nosniff`.
- `POST /api/design/export` — `{project, id, kind}` where kind is `html`,
  `tokens.css`, `tokens.json`, `zip`, or `png`. Files land in
  `canvas/<slug>/exports/` and the reply carries the path and byte count.
  The **PNG is the browser lane's**: the bridge spawns the browser lane,
  opens the preview route, captures it at the frame's size, and closes
  the lane.

## The composer

The brief box at the foot of the stage starts an ordinary **fleet run**
against the current session: the task carries the brief, the chosen style
preset's spec, and the board's slug, and it asks for the page through
`frame`. The run streams like any other — the canvas repaints as each
`frame` result lands — and the conversation appears in `#/sessions` with
its full log. The style presets are the source project's curated set,
embedded once.

## The page's models

A design run's route comes from this page. A page asks a different
question of a model than code or prose does, so fleet designates three
models for design work (`canvas::DESIGN_MODELS`) and the composer's
picker names one per run:

```
deepseek-flash · zai GLM-5.3-Flash · ollama-cloud glm-5.3-flash
```

The picker opens on the first candidate that can serve *now* — the read
resolves each against the roster, so a model with no key reads
*unavailable* and is refused up front instead of failing mid-run — and
the owner's choice persists across repaints. The run then goes out pinned
to `<provider>/<model>`: a pair pin skips the router's judgment entirely,
because the fact that the request started here *is* what makes it design
work. No router chain selects among the designated three, and the
router's own workloads never name them as a design route. A board whose
models can none of them serve says so and leaves the route to the router.

## Design work started elsewhere

A design asked for in the conversation — or by a cron job, or the CLI —
lands on the board all the same: any run can call `frame`. When a frame
lands while the owner is not on the design page, the conversation raises
one **steer card** naming the board it landed on, and its press opens
`#/design/<slug>` there. A fan-out that lands several pages in one run
updates the one card rather than stacking them. Nothing is re-routed and
nothing is re-generated: the run is the same run, and the owner is
walked to where the work lives. The designer's doctrine says the same
thing from the model's side — a design asked for in a conversation
belongs on the page, never in prose.

## What is proven, and what is not

The gate pins the document and its caps, the injection (including a
hostile value and the unit rule), the scan's facts, the tools'
validation, delete/duplicate/restore, the exports (a real ZIP), the
preview's injected page, the composer's run and its model picker (the
pinned `provider/model`, the unavailable option refused, the default
being the first that can serve), the steer a frame landing off the board
raises, and the canvas's gestures and writes; a live Chromium render at
1440×900 is the visual review, and the PNG export ran against the
browser lane. **Not yet exercised end to
end:** the review ask — browse → screenshot → vision → `review_frame` —
whose pieces are each pinned but which no model run has walked. A design
run also cannot see its own output: the review is a second run.

## What it is not

No vector editing (no shapes, layers, resize handles, grouping or
alignment guides — the source had none either), no undo beyond version
restore, no realtime collaboration, no publishing or hosting, no
billing. The frame's own page is the artifact: what it does inside its
iframe is the page's business, and the canvas never injects model HTML
into the face's own document — only into an iframe's `srcdoc`.
