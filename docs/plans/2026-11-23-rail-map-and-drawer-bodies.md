---
status: active
created: 2026-11-23
plan: rail-map-and-drawer-bodies
---

# The rail map and the drawer's object bodies — implementation

The owner's direction, quoted:

> "I like suggestion 2 because it groups objects logically into the type of
> work … suggestion 3, the drawer needs to be context aware of the object
> selected, if it's code then preview it, if its a file in a tree structure
> show the tree, if it is a md file or a document then a preview, if it is a web
> page or similar object then a preview of the object"
>
> "I'm leaning towards B … i prefer 'Home' with a home icon. The drawer needs
> to include preview of the object itself rendered as a human would see it …
> let's create a plan for both the s1-s6 suggestions and the drawer based viewers."

The study is the evidence: `docs/plans/2026-11-22-ux-map-and-drawer-study.md`
(seven screens, 14 WebKit captures, in-page audits clean) and the viewer
research is `docs/architecture/2026-11-23-document-viewers.md`. This plan turns
them into product work. **B is planned as the leading placement** per the
owner's read; the placement is one lookup table, so a final switch to A is a
one-row change, not a rework.

## What ships, in one sentence per piece

- **The rail becomes a map by work kind** — three bracketed regions
  (`software · design · knowledge`) with **Home** (the inbox, held count) at
  the head and a **system** strip at the foot; state is a dot or a count, never
  a group.
- **Attention consolidates into the inbox and the board** — their orders
  already carry it; the rail stops duplicating it.
- **The drawer opens the body the object is** — code file → code with the
  tree; directory → tree; md/document → rendered preview; image/PDF/office →
  the engine that renders it; a URL object → the web body — with the mode
  switch as the per-object override.
- **The drawer answers "what now"** — one line of why the object matters
  (age/state/seat, read from documents that exist) and the one press it
  suggests, at the top of the read body.
- **The viewers that were gaps become real** — markdown via marked+DOMPurify,
  xlsx fidelity in the core reader, PPTX via `@meshesha/pptxjs` 2.0.0 with the
  text outline as fallback. docx and PDF keep their vendored engines.

## The constraints (unchanged, enforced)

- No new tokens; the theme AA guard measures the file, and the study's own
  audit showed the four themes' words already clear AA where they must.
- Every address keeps resolving (U47): the rail's rows change shape, not the
  routes. The palette and the expert census keep working.
- U13 holds: no arrival opens the drawer; the rail's Home row is a press.
- The face plugin sandbox is not widened. A document viewer **cannot** be a
  plugin today (twelve unparameterised read-only queries, no bytes, no
  network) — that seam is out of scope and recorded in the research note.
- Vendored assets follow the established discipline: `VENDOR.json` entry with
  upstream, version, sha256, licence and reason; `build.rs` ASSETS entries;
  `server.rs` gz routes with an RAWS list for tiny modules; the twin test.

## Rows

### R1 — the rail's model data (one table, no behaviour yet)
- Replace the rail's sources (`GROUPS`, `stateGroups()` order in `buildRail`)
  with one `RAIL_MAP` constant describing region membership for placement B:
  `software: [sessions, board, armada] · design: [design] · knowledge:
  [memory, learning, desk, calibration]`, plus `system: [second-brain,
  workspace-history, vault, plugins, bots, settings]` and `home: inbox`.
  Addresses and labels stay as today (`conversations`, `board`, `desk`,
  `intelligence`, `learning`, `reviews`, `health`, `fleet`).
- `BUILT`, `ROUTES`, `FOLDED` and the palette keep consuming the same
  addresses — the table is the rail's view, not a second route registry.
- Pin: a harness check that every `RAIL_MAP` address is a `ROUTES` address
  and every `BUILT` surface is reachable from the map or the system strip
  (the U47 pin in a new shape).

### R2 — `buildRail` paints the map
- Home first (icon, label, held count when the needs-you document has holds),
  then three region sections with a header and the region's rows, then the
  system strip. Region folding keeps `setRailGroupFolded` keyed by region id;
  the expert census joins the system strip; enabled plugins join the system
  strip (a plugin declaring a retired group still mounts, per U44's rule).
- State: each row carries a dot when its surface has held or working work
  (from the needs-you/working documents the rail already reads) and a count
  only where a count means something (Board's holds, the inbox's door count).
  The needs-you/working GROUPS disappear.
- Pins (replacing the RP7 pins at `harness.mjs:5572–5574`): the rail paints
  Home first with its count; the three region headers in order; each lane in
  its region; no `needs you`/`working` group; a held lane keeps its dot; a
  quiet lane carries none; the current lane's region carries the accent
  bracket; folding a region persists.

### R3 — attention on the surfaces (verification row)
- The inbox's order (age of the hold) and the board's needs-you lead already
  exist; this row pins that they did not regress while the rail shed its
  groups, and that the rail's Home count equals the inbox's held count.
- Pin: one harness case booting the same fixture twice — the rail's Home count
  and the inbox's first rows name the same holds.

### R4 — the drawer's kind table
- One `KIND_MODE` table in the face:
  `{ code: 'code', directory: 'code', markdown: 'preview', image/office/pdf/
  svg: 'preview', url: 'web' }` by the `language` the file read already
  returns (`viewer_language`) and by the object kind for non-file objects
  (fact/deliverable/conversation keep `document`'s context body).
- `openFile` (and every object open that lands on the read tab) chooses the
  mode from the table; the mode switch always wins for the current object and
  is not persisted across objects (the kind is the default; the override is
  per object). The stored `fleet-pane-read` keeps serving the context/
  document body only.
- Pins: one fixture boot per kind — a code file opens `code`; a markdown file
  opens `preview`; an image and a docx open `preview`; a URL object opens
  `web`; switching the mode on one object does not change the next object's
  default.

### R5 — the relevance line and the suggested press
- The read body opens with one line of state read from documents that exist:
  the object's age/state word from the carrying read (a run: the seat or the
  running word; a deliverable: the stage that stops it; a fact: the recall
  that surfaced it; a file: the receipt that named it) and the one press the
  drawer already knows how to make (answer the seat, add the source, see where
  it came from, jump to the line).
- No new backend: each line reads a document the face already holds.
- Pin: one case per kind asserts the line's text source and the press's
  action; a kind with nothing to say draws no line (honest absence).

### R6 — markdown, properly rendered
- Vendor `marked@18.2.1` (MIT, zero deps, 53 KB UMD) into `assets/vendor/`,
  `VENDOR.json`, build assets and the gz route. Render markdown through
  marked → DOMPurify (already vendored) → the preview body. The hand-rolled
  `markdown()` block parser retires; `inline()` stays only if other surfaces
  use it (checked in-row).
- Pins: ordered lists, blockquotes, images (sanitised), nested lists, tables
  and code fences all render; a hostile script/iframe/`onerror` payload is
  stripped; the upstream license text lands beside the bundle.

### R7 — xlsx fidelity in the core reader
- Extend the existing zip+`DOMParser` reader (no new dependency):
  `xl/styles.xml` number formats and bold, `<cols>` widths, `<mergeCells>`,
  Excel date serials, right-aligned numerics; keep the 500-row page and the
  "formulas are not evaluated" line (cached values are shown).
- Pins: a small fixture workbook (generated once, committed as bytes with its
  generator noted) renders a dated cell as a date, a bold header, a merged
  cell, a right-aligned number and a percentage cell; the row cap still says
  so.

### R8 — PPTX, rendered
- Vendor `@meshesha/pptxjs@2.0.0` (MIT, 366 KB UMD, fflate inlined) with the
  full vendor discipline. `renderPptx` builds the sandboxed document the way
  `renderDocx` does: bundle + bytes inlined, `PptxViewer` over the container,
  `mode: 'list'`. On any error, fall back to today's text-outline renderer —
  named as the fallback, never silent.
- Pins: source pins for the vendored bundle and the sandboxed mount; the
  fallback path (a fixture that fails to parse) lands the outline and says
  so; the engine's own no-network posture is recorded in `VENDOR.json`'s
  reason.

### R9 — the map below 900px
- The rail drawer renders the same map (Home, regions, system) — no second
  layout path. The map's rows keep their dots/counts inside the drawer.
- Evidence: WKWebView captures at 390 and 820 for night and day (the study's
  S6 composition, captured from the implementation), plus harness pins that
  the rail's drawer carries the region headers.

### R10 — the gate and the hygiene, after every row
- Four legs by exit code: `cargo fmt --check`, `cargo clippy -D warnings`,
  `cargo build --release`, `cargo test`; `node tests/face/harness.mjs` green.
- Hygiene after every delivery: `repo-hygiene.sh --report`, `plan_check.py`,
  `ki_ledger.py`, `git status` read against the edit list; the tower-log line
  with the commit.
- This plan's gap ledger: `2026-11-23-rail-map-and-drawer-bodies-gap-ledger.md`
  — a gap is researched, replanned, matched to a row, and closed by commit.

## Acceptance criteria

- [ ] R1–R2: the rail is the map (Home, three regions, system strip, dots and
      counts, folding, expert census, plugins) and no attention group remains;
      the model table is pinned against `ROUTES`.
- [ ] R3: the inbox and the board still carry attention; Home's count equals
      the inbox's held count.
- [ ] R4: every kind opens the body it is (code/tree/rendered preview/web),
      the switch overrides per object, and defaults do not leak across opens.
- [ ] R5: each object kind's read body opens with a relevance line and one
      suggested press, both read from existing documents; absences are named.
- [ ] R6: markdown renders ordered lists, blockquotes, images, nesting, tables
      and fences through marked+DOMPurify, hostile payloads stripped.
- [ ] R7: xlsx shows number formats, dates, bold, merges, widths and
      right-aligned numerics; the row cap and the formula line still hold.
- [ ] R8: pptx renders slides in the sandboxed frame with the outline fallback
      on failure, both pinned; the vendor entry carries MIT and the reason.
- [ ] R9: the 390/820 map-drawer evidence is captured from the implementation
      and read; both themes.
- [ ] Harness green and the gate built by exit code after every row; hygiene
      run and recorded per delivery.
- [ ] The study's S1–S6 model is what shipped: map (B), Home, consolidated
      attention, type-aware drawer, phone/pad map.

## Out of scope

- The plugin byte seam (a parameterised document query) — recorded in
  `docs/architecture/2026-11-23-document-viewers.md`, gated separately.
- Placement A is not built; it remains a one-row table change if the owner
  reverses.
- No release cut or version bump without the owner's explicit word (standing
  rule).
- No theme or token changes; the audited AA state stands.
- No change to `/api/preview`'s containment, credential refusal or 32 MB cap.

## Risks, named

- **PPTXjs 2.0.0 is young.** Mitigation: the outline fallback, the version
  pin, the vendor entry, and a render smoke in the row itself. A regression in
  the engine degrades to the outline — it never blanks the drawer.
- **The rail's re-pin is broad** (fold state, expert census, plugins). The
  state-group harness pins retire with their behaviour; each new pin names the
  replacement, so a silent loss is a red check, not a quiet gap.
- **XLSX styles scope creep.** The row lists exactly which style features ship
  (formats, dates, bold, merges, widths, alignment); anything else stays
  values-as-table, said in the pane.
