---
name: design-canvas-native-plan-2026-10-31
description: "LANDED 2026-10-31 (04abf41 + 0630df5) — design-canvas ported natively to fleet's face: a canvas document in the home, a design stage with pan/zoom/frames/tweaks/pins, generation via a frame tool + designer role, exports incl. PNG through the browser lane"
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

**LANDED 2026-10-31** — `04abf41` (the landing) + `0630df5` (live-proof fixes), plan `docs/plans/2026-10-31-design-canvas-native.md` status completed, criteria D1–D10 all checked. Gate: fmt 0, clippy 0, build 0, **suite 1,866/0**, harness **543/55**. Live: sample board adopted, PNG export through the browser lane 64,379 bytes, 1440×900 render read as the visual review (fitted at 40%, zero page errors).

From the owner's ask: *"design-canvas is a project that could be very useful for fleet, disregard all commercial issues of that app, … integrate it as a fully native part of the app, same fleet design language, not a mere bolt-on."*

**What design-canvas actually is** (surveyed at HEAD `fff86bb`, `~/Projects/design-canvas`): an AI web-design canvas where the atomic object is an **opaque self-contained HTML page in a sandboxed iframe** on an infinite pan/zoom board — NOT a vector/figma editor (no shapes, layers, resize handles). Portable core: ~925 lines React canvas, `lib/types.ts`, `lib/tweaks.ts` (injectTweaks — the one sanctioned mutation), `lib/a11yscan.ts`, `lib/handoff.ts`, two real-HTML fixtures. The other 76 API route files + 27 Prisma models = the SaaS platform, out of scope.

**The native swap:** its llm.ts/models.ts/SSE → **a fleet run with a `designer` role** + a new **`frame` tool**, so ANY fleet run can emit a frame and the canvas is where fleet's design artifacts land. Its Playwright PNG → the **browser lane** (`browser_navigate` + `browser_screenshot` against the preview route). Its zip → `deck.rs`'s `zip_parts`. Prisma/PG → `<home>/canvas/<slug>/project.json` (frames array = z-order; caps 512 KiB HTML, 20 versions, 10 reviews) behind `Bridge::design`/`design_write`. Orchestrator → bridge document + face viewport.

**The Rust/JS boundary (amended bfa769a):** **Rust owns what must be the same bytes wherever it is used and everything that touches the home; JS owns what needs a DOM, a pointer, or pixels.** Rust: injectTweaks+extractRootTokens (one implementation — live srcdoc / preview / ZIP / PNG / tokens must agree), the document store, the frame tool's validation, the a11y scan **as a scanner** (the source is regex/string-based; **no HTML parser added**), the ZIP + tokens. JS, never ported: viewport transform, frame cards/iframes, selection, drag, nudges, pins, tweak panel, composer. The one component wanting both is the live tweak: `POST /api/design/inject` returns bytes, the stored document feeds preview/export.

**Design-language boundary:** fleet's tokens/boardCard/sectionRail govern the CHROME; the frame is the artifact (iframe `srcdoc`, sandbox `allow-scripts` without same-origin) — the boundary the preview pane already draws. The preview route carries a strict CSP (`default-src 'none'`, inline styles, no scripts, `frame-ancestors 'none'`).

**Rail placement (owner steer 2026-10-31, `5612619`):** the canvas sits **directly under `conversation` in the WORK group**, above `sessions` — *"we need a design menu item below conversations"*. The reason recorded in the comment: a board is work the owner OPENS the way they open a chat, not a listing to browse after one. Live-verified in Chromium (rail reads conversation → design → sessions, `design` highlighted on the canvas, zero page errors); harness unchanged at 543/55 because the rail derives from the one `GROUPS` declaration. `ROUTES` was deliberately NOT reordered — it is a lookup, not a display order, and the rail/palette both read `GROUPS`.

**Surface as built:** `#/design` stage (ROUTES/GROUPS work-group/PANEL_EL/arrive), instrument strip (zoom out/fit/zoom in/grid/pin mode/readout), the viewport (wheel zoom to cursor 0.05–2.5, space/middle drag pan, fit-to-bounds, title-bar drag with delta/zoom + integer snap, arrows 16/1, persisted delete — dc's client-only delete NOT ported, duplicate, three viewport presets), frame cards with per-frame acts (tweaks/review/versions/export/copy/duplicate/open/delete), pins as 0–1 fractions with threads in the pane, and the brief composer that starts an ordinary fleet run.

**The boards rail (owner steer, `e402ce9` + `163f5f6`):** *"I think the design page should have a rail like sessions does, where it should show all the designs the user has done"*, then the correction *"it should be on the same design language of this and the other pages that have the second rail."* My FIRST attempt was the trap — a bespoke `.design-rail` with `work-row` rows, new CSS and a head `<select>` picker; the owner's correction meant the **shared `sectionRail()`** (settings, calibration, memory, skills, bots, workspace-history). Final shape: `<nav class="section-rail" id="design-rail">` + `.design-body`; one row per board (name, frame count as the badge, age in the badge title), the open board as `current`/`aria-current`, fold remembered under `fleet-design-rail`, arrows between rows; **the head's picker and its `change` listener retired**, so one switcher exists. Layout extends the siblings' selectors rather than cloning: `.stage-canvas` shares the 18px gap and the fold-to-46px rule, with the column at **216px not 188** — the rows/34px/13px metrics are identical, but section names are one word while board names are data and clipped at 188 (the first Chromium render showed "Ember & Oak — Prici…"). Harness **546/55** after strengthening the pin (a one-board fixture made the switch check pass trivially; the stub now answers per requested `project=` slug). Docs updated in the same commits (design.md, surfaces.md, FS).

**Tools:** `frame` (validates, snapshots the refined page as a version, returns the id) and `review_frame` (scores/summary/strengths/fixes onto the frame). `roles/designer.md` carries the doctrine ported from the source's prompts. Both are in the built-in family beside `deck`, and plan mode refuses them.

**Exports:** HTML, `tokens.css`, `tokens.json`, ZIP (via `zip_parts`), and **PNG through the browser lane** — the bridge spawns the lane, opens the preview route on a **validated loopback origin** the face passes, captures at the frame's size, closes the lane. Files land in `canvas/<slug>/exports/`; the reply carries path + bytes.

**Do-not-port list:** unpinned delete, `/tmp` debug instrumentation in dc's dirty tree, in-memory rate limiting, ownerless SiteAnalysis. Out of scope: Stripe/credits/publishing/hosting/auth/orgs/admin/MCP, vector editing, undo (dc has none either), realtime collab, Firecrawl redesign, brand kits.

**Two bound moves the surface forced (both recorded in the constants' doc comments):** the design tools took the native surface to **9,575 definition tokens** vs 9,000 → `MAX_DEFINITION_TOKENS` **10,000** and 30 native declarations; 30 native + a joining lane's 4 crossed **MAX_VISIBLE_TOOLS 32**, which deferred the joining lane's OWN tools → the count bound moved to **36** (see [[a-declaration-budget-is-a-measurement-too]]). KI-385 filed (a load-sensitive relay test).

Known remainder: the review ask is wired as a run instruction (browse → screenshot → vision → `review_frame`) with every piece unit-pinned, but no live model run of it was executed; the fixtures' pages fail the scan's `:focus` check (a true finding about the sample content); ten of eleven presets extracted (the eleventh didn't match the extractor).

**Docs, after the landing (2026-10-31, `c9cd60e`):** the session recap is `docs/recaps/2026-10-31-the-design-canvas.md` (`347a2ec`), and the aggregate docs caught up in a separate pass — `FUNCTIONAL-SPECIFICATIONS.md` gained the **Work — design** passage in the owner's flow language (thirteen destinations now), `docs/architecture/surfaces.md` a **Design** section in its own shape, `README.md` a design-canvas bullet, `TECHNICAL-DOCUMENTATION.md` the four routes, and `docs/features/design.md` a **"What is proven, and what is not"** section so the feature doc itself names the unwalked review loop. Setting a board's first view **fits its frames** (`designFit` on an unremembered transform) after the 1440×900 render showed the second frame half off-viewport. See [[contract-docs-are-the-third-gate]].

**Scope correction, 2026-10-31 (same day):** this is the KERNEL, not the product. The owner used it and said *"it's missing a lot … just distilled the very basics … I want the whole shebang"*; ~12 of the source's ~25 features shipped. The remainder is carried by [[design-canvas-complete-plan-2026-10-31]] (phases T0–T6) — do not reopen this plan for scope. The lesson is filed as [[a-kernel-is-not-the-product]].

Related: [[provider-models-settings-plan-2026-10-31]] (the composer's model select reads that catalog), [[the-screenshot-is-the-visual-review]] (the visual gate that caught the first-run fit), [[a-second-rail-is-the-shared-section-rail-not-a-lookalike]] (the boards rail).
