---
name: doc-preview-research-2026-10-29
description: "project 2026-10-29 — the pane's preview renders md/pdf/docx/ pptx/xlsx/drawio/svg now: the plan (docs/plans/2026-10-29-the-preview-formats.md) was implemented in full as 2c59652 via \"/goal implement plan as designed\"; the fence, per-format paths, the five vendored libs with provenance, the recorded PDF-frame amendment, and the proofs live here"
type: project
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

2026-10-29, right after the right-pane slice ([[the-right-pane-2026-10-29]]):
the owner asked for the preview tab to render md, pdf, docx, pptx, xlsx,
drawio, svg "etc." via deep research, then "let's create a plan to build
this", then **"/goal implement plan as designed"** — the whole plan
(**docs/plans/2026-10-29-the-preview-formats.md**, status completed) landed
as **2c59652**, gate green, relaunched, harness **351 checks / 38 cases**.

**What landed:**
- **D0 the fence**: `/api/preview?path=` — the text read's own containment
  (working directory, the LEXICAL `.env*` refusal first, 32 MB
  `PREVIEW_BYTE_CAP`), content type from an extension allowlist and nothing
  else, `nosniff`, and **`Content-Disposition: attachment` on everything** —
  an `<img>` fetch ignores the disposition, but a navigation to the route
  can never execute an SVG on the face's origin (the SVG-XSS hole closed
  structurally). The sandboxed document frame exists; the deck renders
  inside it now (it is the model's HTML). `viewer_language` grew the
  formats; a binary document with a renderer answers its read with a
  language and NO text; one without stays a refusal.
- **D1**: CSV/TSV quote-aware tables; JSON/NDJSON collapsible `<details>`
  trees; images + SVG as `<img>` (scripts inert there and nowhere else).
- **D2**: markdown table support + the DOMPurify pass (the built renderer
  builds nodes with textContent — the serialize→sanitize→reparse round
  trip is the second layer that survives a renderer's mistake).
- **D3 PDF**: vendored PDF.js 6.4.299, API mode, `enableScripting: false`,
  worker from the same pin, pages to canvases, a fifty-page cap naming its
  cut. **The recorded amendment**: the plan placed the render "inside the
  sandboxed frame"; it renders in the pane's own DOM instead — PDF.js is
  first-party code treating the document as data, and 1.7 MB per frame's
  srcdoc would have been the letter against the sense. The frame is for
  document-authored HTML (deck, docx).
- **D4 XLSX hand-rolled**: zip central-directory walk +
  `DecompressionStream("deflate-raw")` + DOMParser on sharedStrings/sheet
  XML → real tables with sheet tabs, values only, named. **The same reader
  powers D6** (PPTX per-slide outline, the limitation named in the pane —
  no maintained client-side renderer exists).
- **D5 DOCX**: docx-preview + JSZip inside the sandboxed frame, bytes in
  as base64, lib sources inlined with `</script>` escaped — the render
  never touches the app's origin.
- **D7 drawio**: the XML + the `.svg` companion rendered when it sits
  beside it (drawio's own export convention; the interactive viewer stays
  out — 2–4 MB monolith).

**The vendored set** (`src/gui/assets/vendor/`, ~1.9 MB): purify.min.js
(DOMPurify 3.4.1, 28 kB, always loaded via a script tag), pdf.min.mjs +
pdf.worker.min.mjs (6.4.299, lazy `import('/vendor/pdf.min.mjs')`),
docx-preview.min.js 0.4.1 + jszip.min.js 3.10.1 (frame-inlined). Each
carries a VENDOR.json entry (version, upstream, download URL, SHA-256,
license, reason) and THIRD-PARTY-NOTICES.md consolidates attribution.
SheetJS is deliberately absent (npm frozen at a vulnerable 0.18.5) and the
drawio viewer deferred.

**Proofs**: Rust pins `the_preview_route_serves_contained_bytes_only` +
`the_preview_types_come_from_the_allowlist_alone`; the harness PV case
drives the dispatch end to end (csv table, md table + script-stays-text,
json tree, img from the route, and a binary doc failing GRACEFULLY in the
fake DOM — the failure card is the checkable surface where
DecompressionStream doesn't exist); the Chromium probe renders real PDF
bytes — 2,345 inked pixels (`tmp/pdf-probe.mjs`, blob-URL imports because
a setContent page can't cross-origin-import from the server). Harness
count is now **351 checks / 38 cases**.

**The e2e probe (551dcaf, closing the verifier's gap):** the completion
verifier refused "done" because D4 (XLSX) and D5 (DOCX) had only
fake-DOM evidence — the stub pins graceful failure precisely because it
cannot inflate a zip. The answer is the **scratch-gui e2e pattern**:
`FLEET_HOME=/tmp/… fleet gui --port 3199 --no-open` launched with its
CWD as the fixtures directory, driven by Playwright through the pane's
own tree rows and preview tab. Two facts made it easy: the loopback
face's auth is ONLY the connection guard (loopback Host + loopback
Origin — there is no token check and no 401 anywhere in server.rs, the
"launch token" lives in the remote/relay path), and a scratch home
avoids touching the owner's face. Fixtures built with Python's zipfile
(a real 2-sheet xlsx with shared strings; a minimal docx). Results: the
hand-rolled reader inflates the real archive in Chromium — both tabs,
shared strings resolved, every value — and docx-preview reports
**rendered** from inside the sandboxed frame. The docx bootstrap now
posts its state to the parent (boot → libraries → rendered/failed via
`parent.postMessage`), permanent face code — the pane's window into
what an opaque frame did. **The probe's first run was silent because it
verified a stale binary** (form eleven of
[[verify-the-binary-is-newer-than-the-source]]). Probes live:
`tmp/preview-probe.mjs`, `tmp/pdf-probe.mjs`, `tmp/pane-width-probe.mjs`.

Related: [[the-right-pane-2026-10-29]], [[gui-client-verified-by-a-vm-harness]], [[a-layout-claim-is-measured-not-asserted]].
