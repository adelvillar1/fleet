# The design canvas

Fleet's own board of designed pages. A **frame** is one complete,
self-contained HTML page — the artifact a design run produces — and the
canvas is where frames land, get tweaked, pinned, reviewed and handed
off. It was ported from the design-canvas project; what came over is the
canvas surface and its contract, and what stayed behind is that app's
SaaS shell (billing, accounts, publishing, collaboration). The reasoning
lives in `docs/plans/2026-10-31-design-canvas-native.md`.

## The surface

`#/design` is a stage of its own: a head with the board picker and the
frame count, an instrument strip (zoom out / fit / zoom in / grid / pin
mode / the zoom readout), the **viewport**, and the brief composer at the
foot. The viewport is a clipping box holding a world under one CSS
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

## What it is not

No vector editing (no shapes, layers, resize handles, grouping or
alignment guides — the source had none either), no undo beyond version
restore, no realtime collaboration, no publishing or hosting, no
billing. The frame's own page is the artifact: what it does inside its
iframe is the page's business, and the canvas never injects model HTML
into the face's own document — only into an iframe's `srcdoc`.
