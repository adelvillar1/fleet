---
status: active
created: 2026-10-31
updated: 2026-10-31
slug: design-canvas-complete
---

# Plan: the whole shebang — the design canvas completed, and what ports to Rust

## Context

The owner's assessment, 2026-10-31, after using the landed canvas: *"it's
missing a lot from the original design-canvas. it looks as if you just
distilled the very basics of it. I want the whole shebang of
design-canvas. how do we get there?"* — followed by the steer that
scopes this plan: ***"make sure that we port to rust whatever merits
porting."***

The assessment is accurate, and the tally is worth stating plainly.
`2026-10-31-design-canvas-native` (landed `04abf41`) delivered the
**board kernel**: the document, pan/zoom/frames, live token tweaks,
pins, versions, the a11y scan, the review store, the exports and the
`frame`/`review_frame` tools. The source has roughly **twenty-five
product features**; the canvas has about **twelve**. What is missing is
not polish — it is the layer that makes design-canvas a *designer's
workspace*:

| The source has | The canvas today | The native path |
|---|---|---|
| **A board-scoped conversation** (`ChatPanel` + `Composer`): refine in words, with the selected frame, its pins and its review as context; "apply fixes"; regenerate | one-shot brief only; no conversation on the board | the board's own chat, whose runs carry the board and the selection — fleet already has the agent, the event stream and the frame tool |
| **Multi-direction generation**: several models picked at once, one frame each | one frame per brief | N directions + a model multi-select over the roster; each `frame` call lands one |
| **Frame size from the brief** (`mobile` → 390×844; `llm.ts:24-32`) | always desktop 1280×900 | **port to Rust**: the size is a document write |
| **62 style presets** (52 library + 10 curated) with specs and *avoid* lists | 10 curated, no avoid lists | embed all 62 as data; the avoid list rides the task |
| **Image assets**: generated (`imagine`, Wan), uploaded, SVG import; `{{img:NAME}}` substitution into the page | none | **port to Rust**: inlining + MIME + caps; `imagine` is a fleet tool already; SVG sanitizer is a security boundary |
| **Brand kits**: token sets + doctrine, hard constraints | none | a brand store in the home, merged into the task and the frame's tweak defaults |
| **Reference-site DNA**: fetch a URL (SSRF-hardened), extract style facts, generate in that spirit | none | **port to Rust**: the hardened fetch and the style-fact scanner |
| **Redesign**: discover a site's pages, scrape content, generate one coherent site | none | the fetch leg + sitemap scanner (port) + content-faithful generation |
| **Page sets**: a multi-page site — plan, per-page frames, shared nav, coherence audit | none | Rust: nav rewrite/slug (port), coherence checks (new) |
| **The review loop**: 8 named scores, desktop+mobile screenshots, review→fix, history | scan + generic scores; the ask never run live | Rust orchestrates the dual capture via the browser lane; the fix run is the conversation |
| **Version token diff** | list + restore only | **port to Rust**: the diff feeds the panel and the export |
| **React/TSX codegen** (deterministic, 382 lines) | none | **port to Rust**: an export transform |
| **Hand-off packet with PNGs, board-level export selection** | per-frame zip, PNG per frame | Rust: the packet with its PNGs; a selection |
| **Publish, share links, client review, collaboration, billing, admin, MCP** | — | **out of scope** — the owner's earlier "disregard all commercial" stands, and the rest needs multi-user |

## Decisions

1. **2026-10-31 — the scope.** *"I want the whole shebang of
   design-canvas."* The creative product, complete: the conversation,
   directions, assets, brands, reference DNA, page sets, the review
   loop, the exports. Not the SaaS shell (see Out of scope).
2. **2026-10-31 — the boundary.** *"make sure that we port to rust
   whatever merits porting."* Every candidate gets a verdict below; the
   rule is the one the first plan established: **Rust owns everything
   that must be the same bytes wherever it is used and everything that
   touches the home; JS owns what needs a DOM, a pointer, or pixels** —
   and a transform both sides need gets one Rust implementation behind
   an endpoint, never a second copy.

## The Rust/JS boundary, for every candidate

**Port to Rust** (each is byte-identical across consumers, a home
write, or a security boundary):

| Port | Source | Lines | Why Rust |
|---|---|---|---|
| Frame size from the brief | `lib/llm.ts:24-32` | 9 | a document write (w/h) that preview, PNG and export all read |
| Asset inlining (`{{img:NAME}}`, file→`data:`, MIME, caps) | `lib/llm.ts` substitution + `lib/assets.ts` | ~120 | must agree with the preview route, the ZIP and the PNG source |
| SVG sanitizer + intrinsic size + wrap-as-document | `lib/svg-import.ts` | 92 | strips `script`, `on*`, `foreignObject`, non-local refs — a security boundary |
| Version **token diff** | `lib/tweaks.ts` (`diffTokens`) + `VersionHistoryPanel` | ~60 | pure; feeds the panel and the export's changelog |
| HTML→**TSX codegen** | `lib/codegen.ts` | 382 | deterministic export transform, server-side |
| **Hand-off packet** with PNGs | `lib/handoff.ts` + `lib/zip.ts` | 224+~90 | server artifact; the zip is `deck.rs`'s already |
| **Style facts** from a reference site | `lib/fetchSite.ts` (style half) | ~150 | a scanner over style blocks/attributes — a flag, not a parser |
| **Hardened site fetch** (URL normalization, private-range refusal, redirect caps, pinned IP) | `lib/fetchSite.ts` (transport half) | ~280 | network + SSRF: the one place a wrong answer is a hole |
| **Sitemap/nav discovery** | `lib/sitemap.ts` | 185 | lite XML/text scan feeding redesign |
| **Page-set slug + nav rewrite** | `lib/publish-pages.ts` (minus expiry) | ~60 | needed for a coherent multi-page export |
| **Coherence checks** over a board (shared tokens, nav targets resolve, unique slugs) | — (new) | — | deterministic facts over the document |
| **Per-mode task builders** (generate, refine, review, page set, redesign DNA) | `lib/llm.ts` prompt assembly | ~200 | every surface must compose the identical task; the doctrine text lives in `roles/designer.md` |
| **Review record + dual-capture orchestration** | `app/api/review` shape | — | Rust shapes the 8 scores and drives the browser lane twice (desktop + mobile) |
| **Brand store + token merge** | `lib/*brand*` | — | a home document; merged into the task and the frame's tweak defaults in Rust |
| **Preset library shipping** | `lib/stylePresetLibrary.generated.ts` + `stylePresets.ts` | 62 entries | data, embedded once and served by the read — the *selection* by intent reuses deck's suggest path (sem1 cosine in Rust) |

**Stays JS** (DOM, pointer, pixels): the canvas viewport and gestures,
the boards rail, the board's chat panel and its frame chips, the
variant grid, the drawers, the tweak/asset/preset/brand pickers as
*controls*, the review card, the version panel's list, keyboard
handling. None of these may reimplement a transform above.

**Reuses fleet, no port**: generation is fleet runs + the `frame` tool;
the review's vision read is the `vision` tool; screenshots are the
browser lane; image generation is the `imagine` tool (gen1, Wan 2.7);
web browsing is the `web` tool; the chat itself is the agent and its
event stream.

## Phases

Each lands whole — code, checks, docs — and each is green on its own.

### T0 — the board's conversation

The biggest missing piece, and the one most others lean on. The design
stage gains a **chat panel** beside the canvas (the conversation's own
card: message list, composer, streaming) whose runs are
**board-scoped**: the task names the board, the selected frame(s), their
pins and the latest review. Verbs: **refine** (a run that calls `frame`
with the selected `frame_id`), **regenerate** (a new frame from the
selection's brief), **apply fixes** (a review's fixes as the
instruction), **ask** (a run with no frame write). The transcript shows
**frame chips** — a press selects and fits that frame on the board. The
per-mode task builders are Rust (`canvas::designer_task`), so the CLI
and a cron job compose the identical task.

*Checks:* W1, W2, W3.

### T1 — directions, sizes, models

The brief composer becomes dc's: a **directions** count (N frames per
brief), a **model multi-select** over the roster's enabled catalog (the
providers plan's own read), and the **size from the brief** — Rust's
`canvas::frame_size_for` decides desktop/tablet/mobile and the frame
lands at that size. Each landed `frame` shows as a variant card with
its own progress line while the run streams.

*Checks:* W4, W5.

### T2 — assets and brands

`frame` gains `assets: [{name, path}]`: each file is read, MIME-checked,
capped, and inlined as `data:` — and `{{img:NAME}}` placeholders in the
model's HTML are substituted (Rust, one implementation). The canvas
gains an **assets panel** per board (files under
`canvas/<slug>/assets/`, a manifest, click to copy a placeholder), an
**upload** path, and **SVG import** (sanitized in Rust, wrapped as a
document). **Brand kits**: `<home>/canvas/brands/<slug>.json` (tokens +
doctrine), a brand picker in the composer, and the merge into the task
and the frame's default tweak values. The full **62-preset library**
lands as data, with each preset's *avoid* list riding the task as the
negative prompt.

*Checks:* W6, W7, W8.

### T3 — reference DNA, then redesign

**Design from a URL**: `canvas::fetch_site` (Rust: URL normalization,
private-range refusal, redirect caps, pinned resolution, byte cap) and
`canvas::style_facts` (the scanner: palette, type, scale, radius,
layout signals from style blocks and attributes — a flag, not a
parser). The composer gains a reference field; a run receives the facts
as design DNA. **Redesign mode** then composes on top: the sitemap
scanner discovers pages, the owner picks, the run generates one
coherent site from the scraped content.

*Checks:* W9, W10.

### T4 — page sets and coherence

**Page set** mode: the run plans the pages (a shared token system and
nav first), then calls `frame` once per page; Rust rewrites each page's
internal links to the set's slugs (`canvas::page_set_slugs`, ported from
`publish-pages`) and runs `canvas::coherence`: shared tokens present
across the set, every nav target resolves to a frame in the set, no
duplicate slugs. The board groups a set's frames and exports them as
one packet.

*Checks:* W11, W12.

### T5 — the review loop, completed

The review card names dc's **8 scores** (hierarchy, spacing, contrast,
accessibility, polish, motion, aiUx, responsive) and the ask captures
**two** screenshots — desktop and mobile — through the browser lane,
hands them to the `vision` tool, and stores the review with
`review_frame` (Rust shapes the record and drives both captures). The
review's fixes become an **apply fixes** run from T0. A **review
history** strip reads the bounded history the store already keeps.

*Checks:* W13.

### T6 — versions and exports, completed

The version panel gains the **token diff** between any two versions
(Rust, ported) and a preview of the older page. Exports gain the
**React/TSX codegen** (Rust port), the **packet with its PNGs**
(desktop and mobile), and **board-level selection** — check frames, get
one ZIP with each page, its PNGs, the tokens, the manifest and the
changelog.

*Checks:* W14.

## Acceptance criteria

- [x] **W1 — the board has a conversation.** The design stage renders a
      chat panel whose run's task names the board; the transcript shows
      the run's cards and frame chips; a frame chip's press selects and
      fits that frame. Pinned by the harness and one integration run.
- [x] **W2 — refine is a run on the selection.** With a frame selected,
      refine calls `frame` with that `frame_id`; the board reloads
      and the frame keeps one identity with a new version. Pinned by the
      integration test and the harness.
- [x] **W3 — apply-fixes is a run.** A stored review's fixes compose a
      run whose instruction names them; the harness asserts the task
      text carries the fix lines.
- [ ] **W4 — directions.** A brief with N directions lands N frames in
      one run, each with its own progress line; the harness asserts the
      task asks for N.
- [ ] **W5 — the size follows the brief.** A mobile brief's frame lands
      390×844, a tablet brief 768×1024 — Rust's rule, pinned by unit
      test, and the harness reads the frame's w/h.
- [ ] **W6 — assets inline.** A frame with `assets` carries each as a
      `data:` URI; an oversized or non-image file is refused by name;
      `{{img:NAME}}` substitution is the one implementation, shared by
      preview and export. Pinned by unit tests over a real PNG and a
      temp file.
- [ ] **W7 — the SVG import is sanitized.** A script/handler/external-ref
      SVG loses those parts and still renders; the sanitizer is a Rust
      unit pin with hostile fixtures.
- [ ] **W8 — brands and presets.** A brand's tokens reach the task and
      the frame's defaults; all 62 presets ride the read; a preset's
      avoid list composes into the task.
- [ ] **W9 — the reference fetch is hardened.** `fetch_site` refuses a
      loopback/private target, a non-http scheme, a redirect past the
      cap, and an oversized body — each by name, pinned.
- [ ] **W10 — DNA reaches the frame.** Style facts extracted from a
      fixture page (palette, type, scale) ride the run's task; a scan of
      a page with no style block says so.
- [ ] **W11 — page sets are coherent.** A set's pages share tokens, its
      nav targets resolve, its slugs are unique — `coherence` returns
      facts either way, pinned.
- [ ] **W12 — a set exports whole.** One packet carries each page, its
      PNGs, the tokens, the manifest and the changelog; the zip opens.
- [ ] **W13 — the review loop closed.** Two captures (desktop, mobile)
      reach the vision read; the stored review names the 8 scores; the
      history holds bounded entries; the fix run starts from it.
- [ ] **W14 — versions and exports completed.** The token diff names the
      changed variables; the TSX export parses as TypeScript; the
      board-level ZIP holds only the selected frames.

## Out of scope

- **Publishing, hosting, analytics, share/view tokens, client review
  links** — they need a server the owner does not run, and the owner's
  2026-10-31 *"disregard all commercial issues"* stands.
- **Collaboration, invites, roles, ACLs, admin, MCP server, TOTP, API
  keys** — multi-tenant machinery a single-owner home does not carry.
- **Stripe, credits, subscriptions, usage metering, plan gates.**
- **Realtime multi-user editing and CRDTs**; **vector/shape editing**
  (the source has none either — a frame is a page); **undo for
  geometry** beyond version restore.
- **The marketing site and onboarding copy.**

## Risks

- **The conversation is the hinge.** Every later phase leans on T0's
  board-scoped runs; building it last would mean rebuilding the
  composer twice. It goes first.
- **The hardened fetch is the riskiest code here.** A wrong SSRF guard
  is a hole, not a bug: no redirects past the cap, no private ranges,
  no non-http schemes, a byte ceiling — each pinned with a hostile
  fixture before the feature ships.
- **Prompt drift.** The per-mode tasks are Rust for a reason: a mode
  whose assembly lives in JS becomes a second behaviour the moment a
  second surface (CLI, cron) composes it.
- **Asset inlining grows documents.** The per-frame cap stays 512 KiB;
  an asset that would cross it is refused by name with the room left.
- **Dual capture doubles the review's cost.** Bounded: two captures per
  request, one review stored, history bounded at 10.
- **The harness cannot paint.** Structure and writes are pinned there;
  the pixels are the visual gate's job, per phase.

## Dependencies

- The landed kernel (`2026-10-31-design-canvas-native`, completed):
  the document, the tools, the transforms, the stage.
- Fleet's tools: `imagine` (gen1), `vision`, `web`, `browse` (browser
  lane), `deck`'s zip.
- The providers plan's enabled-model catalog (the multi-select reads it).
- The surface-curation plan's measurement discipline for the per-mode
  tasks (a task change is measured, not assumed).

## Notes

- **What the kernel got right, and stays**: a frame is an opaque page
  (so the whole shebang adds *inputs* and *verbs*, never a second
  editor); `inject_tweaks` is the one mutation; the document is the
  truth and the view is a projection. The completion plan adds the
  designer's workspace around that kernel, not a rewrite.
- **The preset count is 62** (52 library + 10 curated), not the 148 a
  first pass counted — that number folded chips and presets together.
- **A measurement worth taking before T4**: whether multi-page
  generation is better as one run calling `frame` N times or N child
  runs (the `subagent` route) — the board shows both; T4 should pick on
  evidence.

## Landed: T0 — the board's conversation

The board has its chat: turns in the document (capped at 200), frame
chips, and the composer with a **verbs** select that follows the
selection. Every request's task is composed in Rust
(`canvas::designer_task`, `mode` ∈ ask / generate / refine / regenerate /
fixes / page_set / redesign) and the face starts the run with exactly
what the writer answered — the pin-fix and review-ask paths go through
the same call, so no surface hand-builds a task. `frame_size_for` is in
Rust too (a mobile brief sizes the frame 390×844). The task carries the
**board's inventory** — each frame's name, id, size, model, open pins
and review state — which the live run proved necessary: before it, an
"ask" about the board made the model explore the console with the
browse tool for minutes; after it, the same ask answered
*"2 pages. — Sample — Landing hero (f-6e0cc6ec, 1280×900) — Sample —
Pricing…"* and handed back.

The design page also **renders the approval seat**: a run raised from
the board shows its seat in the board's own panel with the same
decisions, because an approval behind the conversation stage is the
KI-380 class. The finish→turn flush (the designer's closing words and
the frames it landed, diffed off the board) is pinned in the harness.

**A defect the live gate caught, and its fix**: the face's `post()`
helper resolves `{status, body}`, not a fetch `Response`, so the design
code's `reply.json()` calls had made every follow-up (the composed task,
the run start, the export notices) silently dead while the harness —
which asserted the POST bodies — stayed green. The whole design surface
now reads `reply.body`, and the provider save's reply-driven notices
(the same misuse from the providers landing) were fixed with it.

Evidence: `canvas::` unit pins (task builders, the size rule, the chat
store and its cap, the inventory in an ask), the harness's conversation
checks (turns render, a chip selects and fits, a send records the turn,
composes the task through the writer, starts the run with the composed
text, and the close lands the designer turn) — **552 checks across 55
cases**; the suite **1,868 / 0**, fmt 0, clippy 0. Live: the send chain
over HTTP (chat → task → run accepted), the inventory answer above, and
a real seat on the design page.

**The live flush, seen**: a probe with the tab open sent *"name the
frames on this board"* from the board's composer and watched the board's
chat gain the owner turn, then the designer's closing turn
(*"done — nothing new on the board"*: an ask lands no frames) when
the run ended — the document's turns written off the re-read board.
Live runs stream tokens rather than emitting a `message` event, so the
flush reads the closing words from the rendered assistant row first; the
fallback text above is what the run without streamed content showed.

Two honest limits, both named in the feature doc: a tab closed mid-run
keeps the frames (the tool wrote them) but loses the closing turn, and
the runs on this machine are slow — an ask walked the orchestrator's
chain for several minutes while the inventory fix removed the *tool*
detour (before it, the same ask browsed the console). T1–T6 are
unstarted.
