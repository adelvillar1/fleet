---
name: design-canvas-complete-plan-2026-10-31
description: "Plan 2026-10-31 at docs/plans/2026-10-31-design-canvas-complete.md — COMPLETE, pushed at b4cb09e (status complete, W1–W14 all ticked); T0 e8636f7, T1 38630cd, the page's routing b46383d, T2 cb48dc4, T3 7432498, T4 4c995fa, T5 6b9a939, T6 cebb35a; three runs unspent"
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

**COMPLETE 2026-11-01 (same session arc), pushed `b4cb09e`.** Plan
`docs/plans/2026-10-31-design-canvas-complete.md`, status `complete`, W1–W14
all ticked. Commits in order: `b46383d` (routing from the page — see
[[design-routing-comes-from-the-design-page]]), `cb48dc4` T2, `7432498` T3,
`4c995fa` T4, `6b9a939` T5, `cebb35a` T6, `b4cb09e` (the composer layout fix +
the session recap `docs/recaps/2026-10-31-the-whole-shebang.md`). Gate at the
close: suite **1,889/0**, harness **578 checks / 56 cases**, clippy/fmt clean;
face rebuilt and restarted.

**What each phase is, in the code:**
- **T2** — `design-presets.json` holds the source's 62 directions (10 curated
  + 52 mined from `lib/stylePresets.ts` + `lib/stylePresetLibrary.generated.ts`),
  each split at its own `AVOID…:` clause into `spec` (positive prompt) and
  `avoid` (`canvas::Preset`; `designer_task` appends `Avoid: …`). Assets:
  `put_asset` sniffs the MIME from magic bytes (png/jpeg/gif/webp/svg), 256 KiB
  cap, manifest under `canvas/<slug>/assets/`, `{{img:NAME}}` →
  `substitute_assets` one path; the frame tool's `assets:[{name,path}]` reads a
  path only inside the fleet home or the cwd. `svg_sanitize` STRIPS (not
  refuses) script/foreignObject/iframe/object/embed, `on…=` and every
  out-of-file href/src, quote-aware. Brands: `canvas/brands/<slug>.json`
  (tokens + doctrine), chosen per board, colour tweaks seeded from the
  brand's tokens on landing. Routes `/api/design/asset/<slug>/<name>`.
- **T3** — `fetch_site` (http(s) only, no credentials, loopback/private/
  link-local refused literal AND resolved, resolution pinned to the checked
  addrs, 3 redirects each re-validated, 2 MiB, page content type) with a
  `SiteTransport` seam so the caps are pinned by a scripted transport;
  `style_facts` scans palette/fonts/sizes/radii/layout/title/description/own
  pages; `POST /api/design/reference` caches a `BoardReference` on the project;
  composer field + scan + page picks + a `redesign` verb.
- **T4** — `Frame.set`; `page_set_slugs`, `rewrite_page_links` (`<slug>.html`),
  `coherence` (shared tokens, missing tokens **over the union**, broken links
  resolved against names and slugs with the leading slash trimmed,
  duplicates); `set_packet` / `selection_packet` → `frames_packet` ZIP
  (pages, tokens.css/json, manifest with the coherence report, changelog);
  `set-png` captures every page at its size and 390×844.
- **T5** — `canvas::REVIEW_SCORES` (the eight, closed); `review_task`
  composes the ask; `POST /api/design/review` spawns the browser lane once,
  captures desktop+phone into `canvas/<slug>/reviews/`; history = newest +
  `MAX_REVIEWS` (10).
- **T6** — `token_diff`; `to_tsx` (the ported codegen); `GET
  /api/design/diff/…` and `/api/design/version/…` (CSP-framed old page);
  per-frame pick → selection packet.

**Honest gaps (stated at the close):** no model run has walked a page set, a
reference redesign, or the review capture end to end — every piece is pinned,
the compositions are Rust's, the three runs are unspent. The source's Tailwind
class mapping (`syntax.ts`) was deliberately not ported (W14 is TSX, not
Tailwind). KI-385 (load-sensitive relay suite) fired three times across the
gate, a different test each time, green alone.

**Tooling note kept from T2:** node `--experimental-strip-types` cannot
resolve an extensionless TS import, so copy both `.ts` files to a scratch
dir and rewrite the import to `.ts` before importing.

Related: [[design-canvas-native-plan-2026-10-31]],
[[design-routing-comes-from-the-design-page]],
[[session-recap-2026-10-31-the-whole-shebang]],
[[design-parity-plan-2026-11-01]],
[[a-page-set-report-needs-the-union-not-the-intersection]],
[[a-fixed-width-row-pushes-the-pane-not-a-line]].
