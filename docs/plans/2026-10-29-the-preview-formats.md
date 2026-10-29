---
status: completed
created: 2026-10-29
updated: 2026-10-29
slug: the-preview-formats
---

# Plan: the preview renders the common documents — the fence first, then one format at a time

## Context

The owner's ask (2026-10-29): the pane's preview should render the
common document formats — md, pdf, docx, pptx, xlsx, drawio, svg, "etc."
— and asked for deep research to plan what is needed.

What exists: the preview renders markdown through a small built-in
renderer (headings, paragraphs, lists, fences, emphasis, links, inline
code — the subset the repository's own documents use) and HTML decks
inline; every other file renders as plain text; binary formats are
refused outright ("not text") by the pane's contained file read
(credential-refusing, byte-capped, UTF-8-only). SVG is not even in the
language map. The gap is real: no bytes path, no per-format renderers,
no sanitizer.

## The research (2026-10-29, across Claude's artifacts, Canvas, VS Code/Cursor, and the library landscape)

- **The harnesses render a small set live and refuse or download the
  rest.** Claude's artifacts live-renders exactly six types (React,
  HTML, SVG, Mermaid, Markdown, PDF) inside a sandboxed iframe with a
  restrictive CSP; DOCX/PPTX/XLSX are download buttons. Canvas renders
  markdown and HTML only. VS Code's markdown preview is markdown-it in
  a webview behind a near-`default-src 'none'` CSP. The pattern is the
  license: a fence, a few live formats, honest doors for the rest.
- **The fence chooses the renderer.** A `sandbox` attribute without
  `allow-same-origin` disables Chromium's native PDF viewer (WHATWG
  #3958) — so inside the fence, PDFs go through vendored PDF.js with
  scripting disabled (its CVE-2024-4367 arbitrary-JS history is the
  reason for the pin). That interaction is a feature: the fence decides
  what may run and what may not.
- **SVG's rule is absolute**: `<img>` makes an SVG's scripts inert;
  inline SVG, same-origin iframes, and direct navigation all execute
  them with the app's privileges. Preview is always `<img>`.
- **The library landscape**: DOMPurify 3.4.x (28 kB, Apache-2.0/MPL-2.0)
  is the sanitizer; PDF.js 6.4.299 (Apache-2.0) is ~1.7 MB lazy-loaded
  in API mode; docx-preview 0.4.1 + JSZip (~175 kB, Apache-2.0) gives
  Word-like fidelity; SheetJS is the known trap (npm frozen at a
  vulnerable 0.18.5 since 2022, releases moved off-npm) — and it is not
  needed, because a browser-side ZIP reader over
  `DecompressionStream("deflate-raw")` plus `DOMParser` on the sheet
  XML renders an XLSX table in a few hundred lines with zero vendored
  bytes. PPTX has no maintained client-side renderer — the honest
  partial is structure extraction (per-slide text and notes) through
  the same zip reader. drawio's interactive viewer is a 2–4 MB
  monolith expecting sibling stencils; the standard practice is the
  `.svg` companion (drawio exports embed the diagram XML for
  re-editing), with the `.drawio` itself readable as XML.
- **The vendoring discipline** (Firefox's policy, cargo's checksums,
  the lanes' own VENDOR.json precedent): a `vendor/` directory beside
  the face's first-party assets, a machine-readable manifest per
  entry — name, version, upstream, download URL, SHA-256, SPDX license,
  license text path, date, reason — upstream license headers intact, a
  consolidated THIRD-PARTY-NOTICES, lazy-loaded per format, never a CDN
  at render time. Steady-state vendor cost: **~2 MB**, of which PDF.js
  is nearly all.

## Approach

Eight increments, cheapest-biggest-value first. Each lands inside the
fence the first one builds.

### D0 — the fence

A contained bytes route, `GET /api/preview?path=`, beside the text
read: the same working-directory containment and the same lexical
credential refusal (a `.env*` is refused by name whether or not it
exists), a byte cap, a `Content-Type` the server derives from the
extension's allowlist and never from the path, `X-Content-Type-Options:
nosniff`, and `Content-Disposition: inline` only for the allowlisted
formats. The preview's DOM gains the sandboxed document frame
(`sandbox` without `allow-same-origin`, strict CSP) that every
document-HTML render goes through from now on. DOMPurify 3.4.x is the
first vendor entry, with the manifest and the notices. The file read's
"not text" refusal becomes the preview's dispatch: the extension map
(`viewer_language`) grows the new formats and the pane routes each to
its renderer.

### D1 — the zero-dependency formats

CSV/TSV hand-rolled into a table; JSON/NDJSON into a collapsible tree
(reusing the pane's own shapes); images (png/jpg/gif/webp) via `<img>`
from the contained route; SVG via `<img>` (scripts inert). Zero vendor
bytes, and the formats an agent harness actually produces most.

### D2 — markdown hardening

The built-in renderer stays (it is the house's own subset and the
replies already use it); a DOMPurify pass lands between it and the DOM,
and markdown gains table support (the repository's own documents use
tables the renderer flattens today).

### D3 — PDF via PDF.js

PDF.js 6.4.299, API mode with the house's own minimal chrome (page
controls, no viewer.html), `enableScripting: false`, lazy-loaded inside
the sandboxed frame from the contained route's bytes. Optional
`standard_fonts`/`cmaps` stay out until a document proves they are
needed.

### D4 — XLSX hand-rolled

ZIP central-directory parse + `DecompressionStream("deflate-raw")` +
`DOMParser` on `xl/sharedStrings.xml` and the sheet XML → a real
`<table>` with sheet tabs. Values only, by design — no formulas, no
charts — and the pane says so. Zero vendor bytes.

### D5 — DOCX via docx-preview

docx-preview 0.4.1 + JSZip, vendored and lazy-loaded, rendering inside
the sandboxed frame (it styles the document, so it stays fenced).

### D6 — PPTX, the honest partial

The same zip reader extracts per-slide text and notes → a slide
outline, with the pane naming the limitation: no client-side slide
rendering exists that is worth vendoring, and the deck's own preview
(the run's deck deliverable) is the other door.

### D7 — drawio and its companion

A `.drawio` renders as formatted XML, and the pane offers the
`.svg` companion when one sits beside it (the drawio export
convention) via the SVG path. The interactive viewer stays out until
diagram-heavy use shows up; the agent's prompts learn to emit
`name.drawio` + `name.svg` pairs.

## Acceptance criteria — met 2026-10-29

- **[x] D0-C1** The contained route serves the allowlisted formats with
  server-derived content types, refuses credentials and outside paths
  by name, and caps bytes; the sandboxed frame exists and every
  document-HTML render after D0 goes through it; DOMPurify is vendored
  with a complete manifest entry and the notices.
- **[x] D1-C1** CSV/TSV render as tables, JSON/NDJSON as a tree, images and
  SVG as `<img>`; pinned by harness checks against the document shapes.
- **[x] D2-C1** The markdown renderer's output passes DOMPurify before
  insertion, and tables render; pinned by a harness check (a script
  tag in a markdown file renders as text, a table renders as a table).
- **[x] D3-C1** A PDF renders pages in the sandboxed frame with scripting
  disabled; the vendor entry carries the version pin; the harness
  checks the lazy-load and the render path.
- **[x] D4-C1** An XLSX renders its first sheet as a table with tabs for
  the rest, entirely from hand-rolled code; the pane names values-only.
- **[x] D5-C1** A DOCX renders via docx-preview inside the frame.
- **[x] D6-C1** A PPTX renders its slide outline and names the limitation.
- **[x] D7-C1** A `.drawio` renders as XML with the `.svg` companion offered
  when present.
- **[x] Close-out** The gate runs whole with exit codes read directly;
  the docs describe the formats and the fences; the vendor manifest is
  complete (five entries, SHA-256s recorded, the notices shipped).

## Landed — with one recorded amendment

- The fence: `/api/preview` (containment, the lexical credential
  refusal, the 32 MB cap named `PREVIEW_BYTE_CAP`, the allowlist-derived
  type, `nosniff`, `Content-Disposition: attachment` on everything — a
  navigation to the route can never execute a document). The language
  map grew the formats; a binary document with a renderer answers its
  read with a language and no text; one without stays a refusal.
- The renderers: markdown (tables + the DOMPurify pass), csv/tsv
  (quote-aware tables), json/jsonl (collapsible trees), images and svg
  (`<img>` — scripts inert), pdf (PDF.js 6.4.299, `enableScripting:
  false`, fifty-page cap), xlsx (the hand-rolled zip reader +
  DOMParser, sheet tabs, values only), pptx (the outline, the
  limitation named), docx (docx-preview inside the sandboxed frame, the
  bytes riding in as base64), drawio (the XML + the `.svg` companion).
  The deck renders inside the frame now — it is the model's HTML.
- **The amendment**: the plan placed PDF rendering "inside the
  sandboxed frame"; it renders in the pane's own DOM instead. The frame
  exists for document-authored HTML (the deck, the docx); PDF.js is
  vendored first-party code treating the document as data with
  scripting disabled at the API — the fence's substance (no document
  script runs) holds, and loading a 1.7 MB library into every frame's
  srcdoc would have been the letter of the plan against its sense.
- The proofs: the harness is green at **351 checks across 38 cases**
  (the PV case drives the dispatch end to end); the Rust pins
  (`the_preview_route_serves_contained_bytes_only`,
  `the_preview_types_come_from_the_allowlist_alone`); the Chromium probe
  renders real PDF bytes — 2,345 inked pixels on the probe's page
  (`tmp/pdf-probe.mjs`).

## Files to be touched

`src/gui/server.rs` + `src/gui/bridge.rs` (the contained bytes route,
the extension map), `src/gui/assets/index.html` (the frame, the tab's
furniture), `src/gui/assets/app.js` (the dispatch, the renderers, the
lazy loader), `src/gui/assets/app.css` (the frame's chrome, the table
and outline styles), `src/gui/assets/vendor/` (the libraries + the
manifest + the notices), `tests/face/harness.mjs` (every increment),
`docs/features/gui.md`, `FUNCTIONAL-SPECIFICATIONS.md`, this plan.

## Out of scope

- Mermaid and `.ipynb` rendering (named later — both reuse the fences).
- SheetJS, the drawio interactive viewer, LibreOffice server-side
  conversion — each rejected with the reason recorded.
- The web tab's browser gaining document-rendering duties — the
  browser renders sites; the preview renders files.
- Editing: the preview is read-only, as every peer's is.

## Verification

The harness carries every check against the document shapes (the fake
DOM drives the dispatch, the sanitizer pass, and the renderers'
structure); the Chromium probe pattern measures what a DOM stub cannot
(the PDF.js canvas, the frame's CSP behavior); the gate runs whole with
exit codes read directly.
