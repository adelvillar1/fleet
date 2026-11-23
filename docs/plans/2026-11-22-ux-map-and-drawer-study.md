---
status: completed
completed: 2026-11-23
created: 2026-11-22
plan: ux-map-and-drawer-study
---

# The UX map study — the rail as a map, attention in the work, the drawer by object kind

The owner's direction (2026-11-22, quoted):

> "the rail was meant to organize things is a way that is not a dull menu, the
> contextual drawer is there because there are objects shown on the screen for
> which we were not providing any relevant information. is there a better way?"
>
> "I like suggestion 2 because it groups objects logically into the type of
> work, suggestion 1 is more like an inbox and/or kanban board, and maybe there
> is a consolidation opportunity there. suggestion 3, the drawer needs to be
> context aware of the object selected, if it's code then preview it, if its a
> file in a tree structure show the tree, if it is a md file or a document then
> a preview, if it is a web page or similar object then a preview of the object"
>
> "let's produce them, like I said before I don't mind ripping and replacing
> what we have in place so that we can have the right UX"

Three rulings carry into the study:

1. **The rail becomes a map by work kind.** Stable geography — making
   software, making design, making knowledge — with the lenses sitting inside
   their region. The picture is learnable; the region names the kind of work
   before the lane's name does.
2. **Attention consolidates into the work surfaces.** The inbox is already
   ordered by the age of the hold and the board already leads with its
   needs-you rows; the rail stops being a second attention surface. Its lanes
   keep a light signal (a dot, and a count only where a count means
   something), never attention grouping.
3. **The drawer is context-aware of the selected object.** The read body is
   chosen by what the object is, not by the last mode the owner used; the
   four-tab grammar (read · act · connections · made here) and the read
   body's mode switch stay. The drawer opens with a line of why the object
   matters now and the one press it suggests.

The owner's test for every screen: does the information carry value or foster
interaction; could it be grouped better; does the organisation earn its
cognitive load; does it survive the phone and the tablet.

## The models under test

**The map rail (one model, two placements).** Three work-kind regions plus a
system strip at the foot, and one utility row at the head: the door (the
inbox), carrying the held count — attention's entrance, not a region.

- **Placement A — by the work it serves.** making software: Conversations,
  Board, Reviews, Fleet · making design: Design, Desk · making knowledge:
  Intelligence, Learning. The argument: Reviews gates the software you make;
  Desk is a brief bench whose first user is design work.
- **Placement B — by the nature of the evidence.** making software:
  Conversations, Board, Fleet · making design: Design · making knowledge:
  Intelligence, Learning, Desk, Reviews. The argument: the desk's claims and
  the reviews' judgments are the same nature — evidence read against sources.

**The drawer by object kind.** One read body chosen by the selected object:

| object | the body it opens with |
|---|---|
| code file | code view, with its place in the tree |
| markdown / document | rendered preview |
| other file | the tree, positioned on the file |
| web page / URL capture | the page preview |
| design board | the board, rendered |
| conversation / run | the transcript with its goal and state |
| fact | its card grammar — basis, sentence, recalls |
| deliverable | its stages and claims |

**Attention where the work is.** Drawn once, honestly: the inbox ordered by
the age of the hold, the board's needs-you lead, the rail's map carrying dots
and the door's count.

## Screens

- S1 `00-rail-map-a.html` — full shell, placement A, the Board in the stage.
- S2 `01-rail-map-b.html` — placement B; the two screens differ only in
  placement, so the comparison is honest.
- S3 `02-drawer-matrix.html` — the eight object kinds as drawer specimens:
  the chosen body, the mode switch's chosen mode marked, the relevance line
  and the suggested press.
- S4 `03-drawer-in-place.html` — in-place frames: a conversation with the
  drawer on a code file (code + tree); the desk with the drawer on a claim's
  source (document, the sentence highlighted).
- S5 `04-attention-consolidated.html` — the inbox and the board carrying
  attention in the stage; the rail with the door count and light dots.
- S6 `05-map-phone.html` — the map at 390 and 820: the rail as a drawer,
  regions collapsible, Home at the top.
- S7 `06-office-previews.html` — the drawer's preview as a human sees it:
  docx, xlsx, pptx, image, PDF — each drawn as its renderer paints it, the
  engine labelled, preview as the default body for documents and images.

## Acceptance criteria

- [x] Each screen drawn with the study shell, no new tokens — the system
      sheet's variables only (`study.css` adds grammar, not colours; the one
      literal is the persona disc's dark label, which the sheet already
      carried).
- [x] Each screen captured at its viewport (1440 for S1–S5; 390 and 820 in
      S6's device frames) in night and day, measured, and READ — twelve
      captures in `png/`, twelve in-page audits, all clean (see the
      measurement pass below). Defects fixed the turn they were found.
- [x] Both placements argue their case in the screen's own copy; neither is
      straw-manned — S1 and S2 differ only in the rail and carry the same
      argument card, stated both ways.
- [x] The drawer matrix names every object kind the harness carries and shows
      the body, the override where a kind has siblings, and the relevance
      line + suggested press.
- [x] The phone frame proves the map survives below 900px — S6 draws 390
      (full-width drawer) and 820 (left drawer, work behind the scrim), both
      with the regions named.
- [x] The drawer previews the object as a human sees it — S7 draws the five
      office/media types with their engines (docx-preview, the core xlsx reader
      with its planned styles pass, PPTXjs 2.0.0, `<img>`, PDF.js), and the
      viewer research is recorded in `docs/architecture/2026-11-23-document-viewers.md`.
- [x] The owner's gate: B is the leading placement ("I'm leaning towards B"),
      Home replaces the door, and the drawer previews the object as a human
      sees it (S7). The implementation plan is
      `docs/plans/2026-11-23-rail-map-and-drawer-bodies.md`.
- [x] No implementation until that gate — this study touched only mockups
      and the mock sheet's tokens; no product code.

## The measurement pass — what it found

Captures: WebKit (the app's engine) at exact viewports, the theme seeded
before paint, the study's theme switch suppressed (`?switch=off`) so the
evidence is the design. Audit: an in-page pass per screen per theme —
`scrollWidth` vs `clientWidth` on the page and on every clipping box; the
type floor; and WCAG contrast on every text node against the ground actually
painted beneath it (canvas-resolved colours, AA 4.5:1, 3:1 for large text and
control glyphs, `aria-hidden` decorative glyphs excluded, painted-image
grounds skipped and counted). Result: **14/14 screens clean** — zero
overflow, no text under 10px, zero contrast failures — after the fixes below.

The audit found four real defects in the shared mock sheet, fixed at the
token rather than per-screen:

1. **`--dsw-static-muted` measured 4.21:1 on the night surface** — the label
   token the rail headers, chips, breadcrumbs and foot all carry. Raised to
   `oklch(0.61 …)`, clearing 4.5.
2. **Day's tone chips failed** (ok 3.96, warn 3.82, down 3.99): the label was
   the raw tone on a 15% wash. `.chip.ok/.warn/.down` now mix their tone 56–60%
   toward `--ink` for the label — the word stays the tone's, the text clears AA.
3. **Faint carried meaning**: line numbers, the "chosen by kind" note, board
   card stamps. Moved to `--ink-soft`/`--muted` and raised to 10px where they
   sat at 9–9.5px.
4. **Code colours**: night `--accent-strong` keywords measured 4.01 on the
   code ground and day's `--accent` 3.8 — the code body now uses `--accent-ink`
   (the themes' own readable-accent alias), and a highlighted line lifts its
   comment to `--ink-soft` instead of dimming it under the wash.

Two capture-time defects were also fixed: S3's code line overflowed its
drawer by 7px (split), and one split line had duplicated its number. S7's
slide-number chrome sat at 8px/3.71 and was raised to the floor and AA. The
shared sheet's fixes change the earlier gallery's harness where it shares
those tokens (its captures predate the fix); the product's own tokens are the
implementation themes, measured by the AA guard, and are untouched.

## Out of scope

- No code, no token changes, no surface removed — this study is evidence for
  a decision.
- The console and contrast themes are not re-drawn; the four themes' AA guard
  covers implementation.
- No backend seams; every object kind in the matrix already exists as a
  surface or a drawer body.

## The measurement protocol (carried from the redesign)

- Captures via WKWebView (the app's own engine) at exact viewports, the theme
  seeded at document start; obscura only for quick structural probes — its
  inline-flex intrinsic-width defect makes it untrustworthy for layout
  evidence.
- Per-screen in-page audit: `scrollWidth > clientWidth` on the page and each
  card; text under the floor; and the theme's own contrast arithmetic on
  every text colour read against its background.


## Amendment, 2026-11-23 — the owner's read

The owner's direction after the first presentation:

> "I'm leaning towards B, on thing though I don't like the name 'The Door' it
> just sounds weird, i prefer 'Home' with a home icon. The drawer needs to
> include preview of the object itself rendered as a human would see it …
> Do some deep research on viewers that can be embedded as plugins in fleet"

Recorded and applied:

- **B is the leading placement** — Desk and Reviews together under knowledge.
  The gate stays open for the final call; S2 is marked the leading candidate in
  the gallery.
- **"The door" became Home**, with a home icon, in every screen. The row's job
  is unchanged: attention's entrance with the held count, never a region.
- **The drawer's preview is now the default body for documents and images** —
  S7 draws docx, xlsx, pptx, image and PDF as their engines paint them. The
  mode switch stays the override where a kind has a sibling body.
- **Viewer research landed**: `docs/architecture/2026-11-23-document-viewers.md`.
  Its verdicts, each measured against the sandbox: markdown → marked 18 (MIT,
  0 deps, 53 KB) + the vendored DOMPurify; xlsx → keep the core reader and add
  styles/merges/widths (no new dependency); pptx → vendor `@meshesha/pptxjs`
  2.0.0 (MIT, 366 KB, fflate inlined) — **tested inside the exact sandbox
  shape**: a two-slide 16:9 deck rendered with run-level type, a shape fill, an
  image and slide backgrounds, from bytes in memory with no network. docx and
  PDF keep their vendored engines. A viewer **cannot** be a plugin today: the
  plugin bridge exposes twelve unparameterised read-only queries and no bytes,
  and the sandboxed frame has no network — the note records the seam that would
  change that and leaves it gated.
