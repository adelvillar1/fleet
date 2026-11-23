# Document viewers for the drawer — the embeddable landscape, measured

The owner's direction (2026-11-23):

> "The drawer needs to include preview of the object itself rendered as a human
> would see it, a preview of an image, a properly rendered md, docx, xlsx
> (table), pptx, etc. Do some deep research on viewers that can be embedded as
> plugins in fleet"

This note records what fleet renders today, the exact constraints any embedded
viewer must fit, the candidates measured against those constraints, and the
recommendation per format. Every version, licence and size below was read from
the package registry or the upstream repository, and the leading candidate was
rendered inside fleet's own sandbox shape (opaque-origin frame, inlined bundle,
no network) before being recommended.

## The constraints, measured from the code

- **The frame.** Every bundle (and every document preview) mounts as an
  `iframe sandbox="allow-scripts"` fed by `srcdoc`: an opaque origin, no
  same-origin, **no network at all**. Assets that must render are inlined into
  the document (the DOCX preview already does exactly this: JSZip and
  docx-preview are injected as source and the bytes ride in as base64).
- **The plugin data path.** `/api/plugins/query` enforces a manifest's declared
  set against **twelve names** — `state, router, delegation, memory, skills,
  sessions, huddles, usage, cache, related, tree, workspace-history` — with **no
  parameters** and **no file bytes** among them. A plugin cannot ask for a
  document today; the query takes only a name.
- **The core file path.** `/api/preview` is the one route that serves document
  bytes: containment to the launch folder, the credential refusal first, a
  32 MB cap, and a content type derived from an extension allowlist, never from
  the bytes. It is core, not reachable from a sandboxed plugin.
- **What is vendored already** (`src/gui/assets/vendor/VENDOR.json`): PDF.js
  6.4.299 (Apache-2.0, worker), docx-preview 0.4.1 (Apache-2.0), JSZip 3.10.1
  (MIT/GPLv3 dual), DOMPurify 3.4.1 (Apache-2.0 OR MPL-2.0).

## What the drawer renders today

| Format | Renderer | Fidelity |
|---|---|---|
| images | `<img>` over the preview route | full; SVG is inert inside `<img>` |
| PDF | PDF.js, pages to canvas | real, 50-page cap |
| DOCX | docx-preview + JSZip inside the sandboxed frame | real — styles, tables, images |
| XLSX | fleet's own zip reader + `DOMParser`: `sharedStrings` and each sheet's cells | **values only** — no number formats, widths, merges, dates; "formulas and charts are not part of this preview" is printed |
| PPTX | the same zip reader: each slide's `<a:t>` text + notes | **outline only** — the pane says "nothing renders a deck client-side" |
| markdown | a hand-rolled block parser (headings, fences, unordered lists, tables, inline) + DOMPurify round-trip | missing ordered lists, blockquotes, images, nesting |
| CSV/TSV | delimited table | full |
| drawio | XML + the `.svg` companion when present | honest partial |

So the real gaps are **PPTX (no slide rendering), XLSX (values, no form) and
markdown (a subset)**. The "nothing renders a deck client-side" line is stale —
it has been investigated and is false.

## The candidates, measured

Registry facts (unpacked size, dependencies) read 2026-11-23:

| Package | Version | Licence | Unpacked | Deps | Notes |
|---|---|---|---|---|---|
| `@meshesha/pptxjs` | **2.0.0** | **MIT** | 6.2 MB | **fflate only** (inlined in the UMD) | v2 TypeScript rewrite; HTML/SVG output; `PptxViewer`, `renderToHtml`; browser/worker/Node; "XSS-safe by construction", zip-bomb limits |
| `pptx-preview` | 1.0.7 | ISC | 1.8 MB | echarts, jszip, lodash, tslib, uuid | renders from ArrayBuffer into a container, but heavy, no repository link, last published a year ago |
| `pptxjs` (npm name) | 0.0.0 | — | 358 B | — | placeholder; the real package is scoped above |
| `xlsx` (SheetJS CE) | 0.18.5 | Apache-2.0 | 7.5 MB | 7 | npm build stale (upstream moved to git.sheetjs.com); styling is SheetJS Pro |
| `exceljs` | 4.4.0 | MIT | 21.8 MB | 9 | read/write oriented, too heavy for a preview |
| `marked` | 18.2.1 | MIT | 592 KB (UMD dist **53 KB**) | **0** | CommonMark + GFM tables; output sanitised by the DOMPurify already vendored |
| `markdown-it` | 15.0.3 | MIT | 2.1 MB | 6 | complete alternative; heavier than marked for this job |
| `mammoth` | 1.13.0 | BSD-2 | 1.7 MB | 8 | docx→semantic HTML, lower fidelity than docx-preview (already vendored) |
| `fflate` | 0.8.3 | MIT | 797 KB (unzipped source) | 0 | the one dep PPTXjs needs; already inlined in its UMD |

### The PPTX candidate, tested in the sandbox shape

`@meshesha/pptxjs@2.0.0`'s UMD bundle (366 KB) was inlined into an
`iframe sandbox="allow-scripts"` + `srcdoc` — opaque origin, no network — with a
generated two-slide 16:9 deck (python-pptx): a 44 pt bold title, a 24 pt body
run, a rounded rectangle with a solid fill, an embedded PNG, both slide
backgrounds. Result: 2 slides loaded; the DOM carried 20 SVGs and the image; the
screenshot shows the title/body hierarchy, the shape's exact fill, the image and
the slide bounds. The API is minimal:

```html
<script src="pptxjs.umd.js"></script>
<script>
  const viewer = new PPTXjs.PptxViewer('#slides', { mode: 'list', scale: 'fit' });
  await viewer.load(uint8array);       // ArrayBuffer/Uint8Array, no URL needed
</script>
```

The upstream's stated support: text styling and bullets, RTL, ~180 preset
geometries and custom paths, gradients and patterns, pictures (incl. SVG),
tables with merges, charts (SVG), SmartArt from the cached drawing, media,
slide backgrounds/master inheritance, hidden slides, notes. Risks, stated
plainly: **2.0.0 is young** (the README still calls it alpha), it also ships an
editor/export surface we do not need, and EMF/WMF pictures are unsupported by
browsers generally. It is the only candidate that renders slides faithfully
offline and passed the sandbox test.

## The recommendation, per format

1. **Images** — keep `<img>` over `/api/preview`. Nothing to change.
2. **PDF** — keep PDF.js. Nothing to change.
3. **DOCX** — keep docx-preview. Nothing to change.
4. **Markdown** — replace the hand-rolled block parser with **marked 18 (MIT,
   zero deps, 53 KB UMD)** rendered inside a sandboxed frame, sanitised with the
   vendored **DOMPurify** before insertion. This closes ordered lists,
   blockquotes, images, nested lists and emphasis — "as a human would see it" —
   with a small, single-purpose dependency.
5. **XLSX** — keep fleet's own zip reader; extend it rather than adding a
   dependency: parse `styles.xml` (number formats, bold), `<cols>` widths,
   `<mergeCells>`, and Excel's date serials; right-align numerics. The existing
   message already says formulas are not evaluated — the cached `<v>` values are
   shown, labelled. (SheetJS CE would do this too, at ~900 KB minified plus a
   stale npm pin and a Pro-gated styling story.)
6. **PPTX** — vendor **`@meshesha/pptxjs@2.0.0`** (MIT, 366 KB UMD, fflate
   inlined) and render it in the sandboxed frame the way docx already renders:
   build the frame, inline the bundle, hand over the bytes. Keep today's
   text-outline renderer as the fallback when the viewer fails, and label the
   fallback. Pin the version in `VENDOR.json` with the MIT text and the reason.
7. **drawio** — unchanged.

## The plugin question, answered honestly

**A document viewer cannot be embedded as a fleet plugin today**, for two
structural reasons measured above: the plugin bridge exposes twelve named
queries with no parameters and no bytes, and a sandboxed plugin has no network,
so it cannot reach `/api/preview` for a document either. A viewer plugin would
need the host to hand the bytes in — the docx renderer's own pattern, which is
core today.

Two honest paths if third-party viewers are wanted:

- **(A) Viewers stay core** — the recommendation now. Each format has one
  vendored engine, one sandboxed frame, one fetch through `/api/preview`, which
  already enforces containment, the credential refusal and the size cap.
- **(B) Add the seam** — one declared, **parameterised** read-only plugin query
  (a document path), enforced with `/api/preview`'s containment and cap, plus a
  mount-time byte handoff (base64 into the sandboxed document, as docx does).
  That is a plugin-contract change with its own plan and its own gate; it is not
  needed for the drawer preview and should not be smuggled in with it.

## What this changes in the study

- The drawer's **preview body becomes the default for documents and images**:
  image, pf→PDF pages, docx pages, xlsx sheet, pptx slides, markdown rendered,
  and the code/tree bodies for source. The mode switch stays the override.
- The map study's office specimens (S7) are drawn from this: docx, xlsx, pptx,
  image, PDF — each labelled with the engine that renders it.

## Costs, in order of adoption

| Item | Size | Licence | Risk |
|---|---|---|---|
| marked 18 (md) | 53 KB UMD | MIT | low; zero deps |
| PPTXjs 2.0.0 (pptx) | 366 KB UMD | MIT | medium — young major, keep the outline fallback |
| XLSX fidelity (core, no dep) | ~200 lines | — | low; the parser and cap already exist |

All three ship through the existing sandbox and `/api/preview`; none needs a
network, a server, or a widened plugin contract.
