---
status: active
created: 2026-11-01
updated: 2026-11-01
slug: design-parity
---

# Plan: design-canvas parity — CRUD, the centered composer, focus, page settings, and a local host

## Context

The owner's assessment, 2026-11-01, after the whole-shebang canvas landed:
five evident gaps against the source design-canvas — *"1) we need CRUD
operations on boards and created designs/pages, 2) the composer should be
in the center frame like in sessions and conversations, 3) focus of the
composer is if no page is selected on the canvas then the request is
global in nature and if a page is selected on the canvas the focus is on
that page, 4) we are missing a lot of page settings and changes that could
be made for a page that are present on design-canvas were not ported over,
5) save, export, host on local environment should be available as well."*
The instruction: a deep dive on the source, then a plan for as much
functionality parity as we can. A sixth direction the same day widened
the scope: multi-model fan-out joins the plan, user-selectable —
*"different models have different strengths and that's why it's part of
design-canvas."*

The deep dive compared the source (`~/Projects/design-canvas`, a Next.js
app: projects/frames, a four-column workspace) against the native canvas
(`src/canvas.rs`, the face's design stage). The kernel is whole — the
document, the tools, the conversation, directions, sizes, 62 presets,
assets, brands, reference DNA, page sets, the review loop, versions, TSX
codegen, packets (the `2026-10-31-design-canvas-complete` plan, T0–T6
complete). What is missing is exactly the owner's five: the CRUD layer
the kernel never surfaced, the composer's place and focus, the page
settings the source's inspector carries, and any save/export/host
affordance. This plan closes those five against the source's own surface.

Two facts from the survey that shape the work: the Rust kernel already
holds most of the CRUD (`create`, `update_frame` with `name`,
`delete_frame`, `duplicate_frame`, `set_frame_set`) — the gaps are a
missing `delete_project` and face affordances, not new engines; and the
source's publish is SaaS-shaped (frozen snapshots, tokens, moderation,
$5/mo hosting) while the owner asked for **host on local environment** —
so hosting is re-opened as a local, live-document route, and the SaaS
shell stays dropped (the 2026-10-31 *"disregard all commercial issues"*
ruling stands).

## Decisions

1. **2026-11-01 — local hosting is in scope; SaaS hosting is not.**
   *"host on local environment should be available as well"* reopens the
   whole-shebang out-of-scope item **for the local variant only**: the
   face serves the board's pages on loopback, and a site ZIP downloads.
   Publishing, share tokens, client review, analytics, moderation and
   expiry remain out.
2. **2026-11-01 — the composer's place is the conversation's, not the
   source's.** The source docks its composer at the bottom of the left
   chat panel; the owner ruled *"the center frame like in sessions and
   conversations."* The design stage's composer becomes a centered
   bottom-docked stack over the canvas, in the conversation's own idiom —
   a native placement, not a copy.
3. **2026-11-01 — focus is first-class.** No page selected → the request
   is board-global; a page selected → the request is page-scoped. The
   task already carries `frame_id` when a frame is selected; this plan
   makes the focus *visible* (chip, placeholder, verb filter) and
   complete (background click clears it).
4. **2026-11-01 — the model fan-out is the user's, and it is in
   scope.** *"multi-model fan-out should be in-scope, but user
   selectable. different models have different strengths and that's why
   it's part of design-canvas."* The composer's model control becomes a
   multi-select over the design catalog: one selected (the default) is
   today's single pinned run; K selected start K pinned runs, one frame
   set per model — the source's "Compare N models" affordance, ported
   with the owner's reason attached.

## Approach

**Close the five gaps where the deep dive located them: the CRUD layer
in Rust + the face, the composer's place and focus in the stage layout,
the page settings on the frame's chrome and cards, and the save/export/
host affordances around the existing write path.** No sys1 fit — this is
a fixed-checklist port against the source's own surface with no per-item
classification, scoring or routing at runtime; the one selection-shaped
read (preset by intent) already rides sem1's cosine path from the
kernel. Every item is either
a port (the source's own behaviour), a native replacement (fleet's shape
for the same job), or a native addition (beyond the source) — the bucket
list below is the contract for which is which. The phases land whole —
code, checks, docs — each green on its own, and the interaction hinge
(composer + focus) goes first because every later affordance is reached
through it.

**The bucket list — ported / native-replaced / deliberately dropped:**

| The source has | This plan | Bucket |
|---|---|---|
| Board create (home + rail "+") | already landed (`new board`) | ported (kernel) |
| Board rename (inline top bar) | inline-editable design-head title → the existing `rename` op | ported |
| Board delete (confirm, cascade) | `canvas::delete_project` + head trash + confirm naming frame count | **new Rust + ported** |
| Frame create | generation/import only — **the source has no blank-frame create either** | not a gap |
| Frame rename | API supports it, **source has no UI**; fleet adds double-click on the title bar → `frame-update {name}` | native addition |
| Frame delete / fork | already landed (frame bar) | ported (kernel) |
| Composer placement | source: bottom of left chat panel → **center, the conversation's idiom** (owner ruling) | native replacement |
| Focus semantics | source: implicit `selectedFrameIds` note to the model → **first-class focus chip + verb filter + task scope** | native replacement (beyond source) |
| Multi-model generation ("Compare N models", one frame per model) | the composer's model control becomes a multi-select over the design catalog; K selected → K pinned runs, one variant card each | ported (user-selectable) |
| Viewport presets on the frame | three buttons on the frame's title bar (`frame-update {w,h}`) | ported (moved) |
| Inspector provenance (model, duration/tokens) + description | the frame line in the tweak card: name · size · model · description · versions, `gen_meta` when present | ported |
| Review: critic-model choice, score deltas, "fix drift" | review card gains the critic picker, history deltas, and a fix-drift press composing a run over the coherence report | ported |
| Set membership (export checkbox) | packet **picks** already native-replace it; add a set picker on the frame → `set_frame_set` | native addition |
| Saved / Saving… indicator | save chip in the design head, driven by `designWrite`'s lifecycle | ported (native shape) |
| Download HTML / TSX / PNG / tokens / packet | every export button downloads its file, not just a disk-path notice | ported |
| Publish (frozen snapshot, /p/<token>, $5/mo, analytics) | **local host**: `GET /site/<slug>/[page]` serves the *live* document + a site ZIP export | native replacement (local) |
| Board duplicate, move-between-boards, blank frame | — | deliberately dropped (the source has none either) |
| Share tokens, client review links, moderation, expiry, billing | — | deliberately dropped (multi-user/commercial; "disregard all commercial" stands) |

- **P1 — the composer in the center, focus first-class** (issues 2+3,
  the hinge). The design stage's composer leaves the right rail and
  becomes a floating bottom-center stack over the canvas — the board's
  log (collapsible, bounded height), the approval seat, then the
  `.composer` form — mirroring the conversation column's arrangement at
  a bounded width. The focus row rides the composer: `board · <name>`
  with a board placeholder and the global verbs (generate, page_set,
  ask, redesign-when-reference) when nothing is selected; `page · <name>`
  with a page placeholder and the page verbs (refine, regenerate, fixes
  when review fixes exist) when a frame is selected. The mode choice
  follows the focus (selecting a page defaults to refine; clearing
  returns to the global default). Clicking the canvas background clears
  the selection — the focus returns to the board. The task composition
  already carries `frame_id`; the pin lands on the visible contract:
  *no selection → global task, selection → page-scoped task*.
- **P2 — boards and pages, CRUD complete** (issue 1). Rust gains
  `canvas::delete_project(home, slug)` — removes `canvas/<slug>/` whole
  (frames, assets, exports, reviews, versions ride along) and refuses a
  missing slug. The face gains: the design head's inline-editable title
  (the existing `rename` op, debounced) and a trash press with a confirm
  naming the board's frame count (the source's own confirm shape); the
  frame title bar's double-click rename (`frame-update {name}`).
- **P3 — page settings parity** (issue 4). The frame title bar gains the
  three viewport preset buttons (the source's own placement); the tweak
  card's frame line gains description and the `gen_meta` provenance the
  frame carries; the review card gains the critic-model choice, history
  score deltas, and a **fix drift** press whose run names the coherence
  report's findings; the frame bar gains a set picker that writes
  `set_frame_set` (an op the kernel already has, unwired) and the
  frame's set badge updates without a reload — the membership is
  assignable from the UI, one picker press, the badge and the board's
  set list following the write.
- **P4 — save status, export downloads, the local host** (issue 5). The
  design head carries a save chip whose lifecycle is the write path's:
  every gesture that goes through `designWrite` flips the chip to
  `saving…` while the request is in flight, `saved` on its ack, and
  `save failed: <reason>` on a refusal — one chip, one wrapped
  function, no gesture left silent. Every export
  button gains a download of its file (html, tokens.css, tokens.json,
  zip, tsx, png — bytes through a validated download route beside the
  exports, never a path the face builds). The **local host**: `GET
  /site/<slug>/[page]` serves the board's pages as live documents —
  tweaks injected, links rewritten through the kernel's own
  `page_set_slugs` + `rewrite_page_links`, a nav over the set's pages,
  `tokens.css` linked — under a CSP that allows the page's own inline
  script and style and **refuses all network** (`connect-src 'none'`), so
  a generated page can run but cannot reach the face's API. The export
  card gains a `site` kind: a ZIP shaped like the source's siteZip —
  `index.html`, one `<slug>.html` per page with FILE-mode relative nav,
  `tokens.css`, `manifest.json`. The host is live, not frozen: no
  snapshot, no republish, no expiry — the board file is the truth, as
  everywhere else.
- **P5 — the model fan-out, user-selected** (the owner's sixth
  direction). The composer's model control becomes a **multi-select over
  `canvas::DESIGN_MODELS`** — the designated catalog the read already
  carries with availability. One selected (the default) behaves exactly
  as today: one run, one pin. K selected compose the task **once** (the
  `task` op's own composition — brief, focus, directions, preset, brand,
  reference — unchanged) and the face starts **K runs, one per model**,
  each out `pin: provider/model` (the page's route, the owner's standing
  ruling), each landing its own frame(s) with a variant card and its own
  progress line — T1's machinery, one card per model. Directions
  compound honestly: N directions × K models is N×K frames, and each
  run's task names its own N. An unavailable model is offered, disabled
  and named, as today. Face-only — no new Rust: the task composes once,
  the pin rides each run.

## Acceptance criteria

- [ ] **C1 — the composer sits in the center frame.** The design stage
      renders the board's log, the approval seat and the `.composer` form
      as one bottom-center stack over the canvas, at the conversation's
      idiom; at 1440×900 the stack is visible, unclipped and the canvas
      is not overlapped beyond its bottom band (render + harness).
- [ ] **C2 — the focus is visible and complete.** With no page selected
      the composer reads `board · <name>` with a board placeholder and
      offers only the global verbs; with a page selected it reads
      `page · <name>` with a page placeholder, defaults the mode to
      refine and offers the page verbs; clicking the canvas background
      clears the selection and the focus returns to the board (harness).
- [ ] **C3 — the focused request is the page's.** A send with a page
      selected composes a task carrying that `frame_id` and the frame's
      facts; a send with none composes a board-global task — pinned on
      the wire in the harness (the composition path already exists; the
      pin names it).
- [ ] **C4 — the board renames from its head.** The design head's title
      is inline-editable and the rename persists across a reload (the
      existing `rename` op, surfaced).
- [ ] **C5 — the board deletes with a confirm.** The head's trash press
      confirms with the board's frame count, removes `canvas/<slug>/`
      whole, and the rail no longer lists it (`canvas::delete_project`,
      pinned with a fixture board).
- [ ] **C6 — a frame renames in place.** Double-clicking a frame's title
      bar name opens an input; the entered name persists across a reload
      via `frame-update {name}`.
- [ ] **C7 — the viewport presets sit on the frame.** The three preset
      buttons render on the frame's title bar and resize the page
      (`frame-update {w,h}`), the tweak card's viewport section keeping
      its own copy.
- [ ] **C8 — the frame line carries the page's settings.** The tweak
      card's frame line reads name · dimensions · model · description ·
      version count — description and `gen_meta` provenance when the
      frame carries them.
- [ ] **C9 — the review card closes the source's gaps.** It offers the
      critic-model choice, shows score deltas in the history, and a fix
      drift press composes a run whose task names the coherence
      report's findings (harness asserts the task text).
- [ ] **C10 — set membership is assignable.** A set picker on the frame
      writes `set_frame_set` and the frame's set badge updates without a
      reload.
- [ ] **C11 — every gesture shows its save.** The design head's chip
      reads `saving…` while a write is in flight, `saved` on its ack and
      `save failed: <reason>` on a refusal, for every `designWrite`
      gesture (harness drives one of each).
- [ ] **C12 — every export downloads.** Each export button (html,
      tokens.css, tokens.json, zip, tsx, png) offers a download of its
      file, not only a disk-path notice; a refused or missing file
      names itself in the notice.
- [ ] **C13 — the board hosts locally.** The site route serves a board's
      page as a live document with rewritten links and a network-dead
      CSP (a fetch from the served page to the face's API is refused);
      the export card's site kind produces the ZIP with an index page,
      one page per frame, the token sheet and the manifest, and the zip
      opens.
- [ ] **C14 — close-out.** The full gate green by exit code, the ledger
      checker green over the real file, and the contract docs carrying
      the new surface (design.md, the verbs table, the interface
      contracts).
- [ ] **C15 — the model control is a multi-select.** The composer's
      model control offers the design catalog as a multi-select with
      availability per option; with the default single selection the run
      body carries exactly one pin, as today (harness).
- [ ] **C16 — K models, K runs, K cards.** With K models selected, one
      composed task goes out as K runs carrying K distinct pins, and each
      run's frames land under its own variant card with its own progress
      line (harness asserts the K pins and the K cards).

## Files to be touched

`src/canvas.rs` (delete_project, the site build reusing
page_set_slugs/rewrite_page_links/coherence, pins),
`src/gui/bridge.rs` (board-delete + frame-set ops, the site export kind,
the download reply),
`src/gui/server.rs` (the `/site/…` route and its CSP, the download
route),
`src/gui/assets/index.html` (the composer stack, the focus row, the
head's rename/trash/save chip, the frame bar's presets and set picker),
`src/gui/assets/app.css` (the floating stack, the focus chip, the save
chip, the frame bar additions),
  `src/gui/assets/app.js` (focus state and verb filtering, background
  deselect, the head's rename/delete wiring, the frame rename, the
  downloads, the save chip over `designWrite`, the model multi-select
  and its fan-out loop),
`docs/features/design.md` (placement, focus, CRUD, settings, save/
export/host),
`docs/known-issues.md` (anything the gate finds, per hard rule 10).

## Out of scope

- **SaaS publishing**: share tokens, client review links, moderation,
  expiry, analytics, billing, the frozen-snapshot model — multi-user and
  commercial machinery the owner's *"disregard all commercial issues"*
  ruling drops (the local host is the re-opened half).
- **Board duplicate, move-between-boards, blank-frame create** — the
  source has none either; not parity gaps.
- **Apply-fixes as a successor frame** — the source's fix lands a new
  frame then re-reviews; fleet's refine keeps the page's identity with a
  version (the landed design). Recorded as a deliberate difference, not
  a gap.
- **Realtime collaboration, CRDTs, vector editing** — the
  whole-shebang out-of-scope stands (multi-model fan-out left this list
  for P5 on the owner's 2026-11-01 amendment).

## Verification

The gate's four commands by exit code at each phase's close; each
criterion's own pin — unit pins in `src/canvas.rs` (delete_project over a
fixture board, the site build's rewritten links and CSP headers), harness
checks for C1–C3 and C11–C12 (the composer's geometry by measurement,
the focus contract on the wire, one save cycle, one download, the
  fan-out's K pins and K cards for C15–C16), and the
live leg for C13 (`curl` the site route, then fetch `/api/design` from
the served page and read the refusal); a 1440×900 render of the design
stage read after every layout change (the screenshot-is-the-visual-review
rule); the ledger checker over the real file as the last act before each
commit.

## Linked artifacts

- `docs/features/design.md` — the composer's place, the focus rule, the
  CRUD surface, the page settings, the save chip, the exports' downloads,
  the local host and the site ZIP
- `TECHNICAL-DOCUMENTATION.md` § Interface contracts — the board-delete
  and frame-set ops, the `/site/…` route and its CSP contract, the
  download route
- `FUNCTIONAL-SPECIFICATIONS.md` § The surfaces (the design page) and §
  The commands as verbs (the new ops)
- `docs/architecture/surfaces.md` — the design destination's line for
  the host route

## Risks

- **The stage layout is load-bearing.** The composer's move rewrites the
  design body's grid; the harness's design checks and every drawer's
  mount point depend on it. Measure with `getComputedStyle` at the
  failing width, and update the harness in the same commit as the
  layout.
- **The site route serves scriptable HTML from the face's origin.** The
  CSP must allow the page's own inline script and style and refuse every
  network hop (`connect-src 'none'`), or a generated page can reach
  `/api/`. The refusal is pinned with a hostile fixture before the route
  ships.
- **`delete_project` is destructive.** It removes a directory whole; the
  confirm names the frame count, the op refuses a missing slug, and the
  pin runs against a fixture board — never the owner's.
- **The download route is a path surface.** File names are validated
  against the board's own exports (no traversal, no absolute paths),
  pinned with a hostile name.
- **Prompt drift.** The focus changes what the composer *sends*, not how
  the task is composed — `canvas::designer_task` stays the one
  composer; the pin asserts the wire carries `frame_id` iff a page is
  selected.
- **K runs are K seats and K× spend.** Each pinned fan-out run can raise
  its own approval seat (the KI-380 class) and spends its own tokens;
  the fan-out is bounded by the catalog (three models), the seat panel
  stacks every seat without losing one, and the composer shows the run
  count before send.

## Dependencies

- The landed kernel (`2026-10-31-design-canvas-native` +
  `2026-10-31-design-canvas-complete`, T0–T6 complete): the document,
  the tools, the transforms, the stage.
- The conversation stage's own composer idiom (`.composer`, the seat
  above it) — the placement to mirror.
- `page_set_slugs`, `rewrite_page_links`, `coherence`, `set_packet`,
  `deck::zip_parts` — the site build's parts.
- The face's `openDrawer`/pane idiom — where the settings cards already
  mount.
