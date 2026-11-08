# Deep dive 2026-11-07: how fleet creates deliverables, read against tw93/Kami

Research only — no plan, no code. The question: what does fleet's deliverable
machinery look like, what does Kami's look like, and which of Kami's ideas
belong in fleet.

The two are closer than json-render was. Kami is a document design system for
professional deliverables (PDF, PPTX, PNG) delivered as an agent skill; fleet's
deck and desk are the same kind of thing. So this comparison is a real
overlap, not an analogy.

---

## Part 1 — Fleet's deliverable machinery

### The taxonomy: five first-class kinds

Fleet has five deliverable kinds, each with its own store, creator, and receipt.
They share house rules (a home-persisted document, a named refusal, a
substrate trace) but each has a bespoke pipeline.

1. **Design frames** (the canvas). One complete, self-contained HTML page —
   `canvas::Frame` (`src/canvas.rs:139`) with `html`, geometry, `tweak_spec`,
   `tweak_values`, `versions`, `pins`, `review`. A board is `canvas::Project`
   (`src/canvas.rs:193`). Lives at `<home>/canvas/<slug>/project.json`; assets
   beside it; exports at `exports/`; review captures at `reviews/`. Created by
   the `frame` tool (`src/tools.rs:304`), handled by `canvas::frame_from_tool`
   (`src/canvas.rs:3388`). Received on the board (`#/design`), the preview
   route, exports (html/tokens/zip/png/site/tsx/set), downloads, and the live
   network-dead host `/site/<slug>/`.
2. **Decks** (presentations / infographics / posters). `deck::Deck`
   (`src/deck.rs:145`) with `ProjectType`, `Layout`, `Node`, `Component`.
   Rendered HTML and an optional real `.pptx` land beside the session log
   (`src/tools.rs:636-662`). Created by the `deck` tool (`src/tools.rs:379`),
   handled by `tools::run_deck_with_event` (`src/tools.rs:477`). Received as a
   deck card in the transcript plus the file.
3. **Files handed over** (`send_file`, `write_file`). Any file inside the run's
   folder. `send_file` is "the one tool whose result is for the owner rather
   than for the model" (`src/tools.rs:1117-1132`); it emits `UiEvent::Artifact`
   (`src/agent.rs:4899-4911`), drawn as a card (`src/gui/assets/app.js:2320`).
4. **Desk deliverables** (evidence-first documents). A brief walked through
   five stages — Claims → Outline → Render → Review → Knowledge
   (`src/desk.rs:114-128`) — with a verified claim set, a coverage verdict, a
   number guard, and a digest. Lives at `<home>/desk/<slug>/project.json`.
   Created by the owner's brief via `POST /api/desk`, not by a model tool.
5. **Huddle / session deliverables.** The same `send_file` artifacts, plus a
   room's goal converted into a desk deliverable; the session's shelf is
   reconstructed from the log (`session_deliverables`, `src/gui/bridge.rs:7844`).

### The model-facing deliverable tools

| Tool | Schema | Handler | Refuses before it lands |
|---|---|---|---|
| `frame` | `project`, `name`, `html`, `frame_id`, `set`, `assets`, `tweak_spec` | `canvas::frame_from_tool` | empty/no html, malformed tweak_spec, unknown board, asset MIME/size/count, incomplete document, over 512 KiB, unresolved `{{img:}}` |
| `review_frame` | `project`, `frame_id`, `scores`, `summary`, `strengths`, `fixes` | `canvas::review_from_tool` | clamps scores, caps strengths/fixes/summary, 10-review history |
| `deck` | `suggest`, `deck`, `format` | `tools::run_deck_with_event` | `deck::check` (Pyramid/MECE/bolding/bounds/identity), residual layout faults after auto-fit |
| `send_file` | `path`, `note` | `tools::check_send_file` | no run root, path outside the root, missing file |
| `write_file` | `path`, `content` | `tools::write_file` | path outside the root, missing parent |

### The canvas pipeline

The writer is the only mutation path: `design_write_in`
(`src/gui/bridge.rs:8892-9269`) with a named-op vocabulary (`create`, `rename`,
`frame-set`, `frame-update`, `version-restore`, `pin-*`, `chat`, `asset-*`,
`brand-*`, `review-put`). Every write crosses `canvas::save`
(`src/canvas.rs:325`), the single funnel that regenerates the digest. Caps:
512 KiB HTML, 20 versions, 10 reviews, 256 KiB per asset, 64 assets, 50
brands, 32 tokens.

Deterministic checks: `a11y_scan` (`src/canvas.rs:3998`) over the page's own
`:root` token pairs, `<img>` alt, control labelling, `:focus`, viewport;
`coherence` for a page set; `token_diff`; the SVG sanitizer; MIME sniffing by
magic bytes. Review is two-track: the deterministic scan plus a vision read
over desktop and phone captures with eight scores (`src/canvas.rs:3678-3718`).

### The deck pipeline

`deck::check` (`src/deck.rs:344`) refuses with `Pyramid`, `Overlap` (MECE,
≥0.6 content-word overlap), `Unemphasised` (no bold), `Bound`, `Identity` —
each refusal names the node and the rule. `deck::review` (`:848`) is a layout
grader, deliberately arithmetic because a static Rust binary has no HTML
rasterizer: `CHARS_PER_LINE = 52` against a 13-line box, flags `Overfull`,
`Unbalanced` (heaviest ≥3× lightest), `HeadlineOverflow`. `deck::auto_fit`
(`:1047`) repairs in place and reverts wholesale if a pass introduces a new
refusal. `deck::render` (`:503`) emits `deck-*` classes that reference the
face's alias tokens, so a deck inherits the theme. `to_pptx` (`:1569`) writes
a real OOXML ZIP with text boxes at real positions.

### The desk loop

`run_stages` (`src/desk.rs:1585`) walks Claims (sys1 extraction) → Outline
(blend1) → Render (deterministic) → Review → Knowledge (corpus seed). A stage
that fails marks `StageState::Refused { reason }` and stops by name. The end
gate is structural: `end_stands` (`src/desk.rs:1508`) blocks the end while any
refusal stands. The document law refuses an unsourced assertion by naming its
nearest miss (`check_assertion`, `:453`); `check_numbers` (`:1224`) refuses an
invented figure.

### The substrate chain

A bound board's `save` writes a deterministic digest at
`canvas/digests/<project>/<slug>.md`; the corpus-refresh cron jobs embed those
digests (`src/cron.rs:839`); a run born in that project reads a bounded design
section from them (`src/canvas.rs:507`, wired at `src/agent.rs:2720`); board
lifecycle moves a `design-board-<slug>` fact through the memory writer. The
desk mirrors this exactly. Compaction survival preserves deliverable paths
(`src/agent.rs:3588-3610`).

### The distinctive decisions

1. **The deliverable is a persisted document, not a message attachment.** The
   board file is the truth; the face is a client of one writer.
2. **The writer is the only mutation path**, a named-op vocabulary.
3. **Refusals are first-class, named, and model-visible**, and the deterministic
   check runs before the artifact lands.
4. **The artifact is project knowledge**, not just an output.
5. **Review is two-track and bounded** (deterministic + vision, 10-entry
   history).
6. **The network-dead artifact**: the site host's CSP refuses every network
   hop, so self-containment is a runtime property of the serve.

### Evidence gaps found

- `viewport` is read by the handler and instructed by the designer role but
  **not declared** in the `frame` ToolSpec.
- External-URL refusal is **doctrine, not code**: `validate_frame_html` checks
  only doctype and size; enforcement is the site CSP plus asset inlining.
- Deck cards are **live-only**: no `Record::Deck` in the session log, so a
  reload loses the card (unlike `send_file`).
- Desk references are **claimed but not structured**: `SourceKind` is only
  `File|Fact|Url`.
- The design review ask is **not yet exercised end to end**.

---

## Part 2 — Kami

### What it is

Kami (紙, "paper") is a document design system delivered as an agent skill. It
gives AI agents layout rules and templates for PDF, PNG, and editable
PowerPoint, with nine document templates and a landing-page system. It is the
"delivery" piece of a trilogy (Kaku writes code, Waza drills habits, Kami
delivers documents). MIT licensed. The code is Python (WeasyPrint for PDF,
python-pptx for PPTX, PyMuPDF for page images, optional Pygments/Node).

### The agent skill (SKILL.md)

The skill is a workflow contract, not a reference dump:

1. **Lock the contract** — language, template, output formats, page/length
   target, visual acceptance, verification command. Stated in one line;
   continue without waiting.
2. **Question budget** — infer first; ask at most once, in one compact
   question, only when two or more of purpose/audience/constraints/success are
   unresolvable.
3. **Route by the artifact's state** — new document / content-only / visual
   repair / generated asset.
4. **Templates** — nine (One-Pager, Long Doc, Letter, Portfolio, Resume,
   Slides, Equity Report, Changelog, Social Cards) plus a landing-page system,
   in EN/CN/KO.
5. **Content IR** — `content.json` with a strict envelope (`type`, `lang`,
   `brief`, `content`) and per-type JSON schemas
   (`references/schemas/*.json`) whose `$comment` fields carry the per-field
   quality bar.
6. **Gap report** — everything the template needs that the input lacks goes
   into one compact table; inside the document a gap is `[DATA NEEDED: what]`;
   **never fill a gap with a guess, a stock image, or an invented value**.
7. **Distill** — every factual claim, number, date, name, source, and material
   reference from the input must land in a template section or the gap report.
8. **Read the right amount** — a tiered table: CHEATSHEET → template → schema →
   design.md, so a new document does not load the full spec.
9. **Fill the template** — copy the template, never write HTML from scratch;
   CSS stays untouched during a content fill.
10. **Deliver** — one command (`build.py --deliver`) runs every gate in order
    and prints one verdict; fix every ERROR and re-run until `READY`.
11. **The visual pass** — open every page image and walk a fixed checklist;
    fix the three most visible defects per round; stop after three rounds.
12. **Fresh review** — review once from the contract, not from your own
    rationale.
13. **Definition of done** — the paths, the verdict and page count, every gap
    and assumption, and an honest visual verdict ("build verified, visuals
    unconfirmed" when the images could not be inspected).
14. **Feedback protocol** — the render is the evidence; name the concrete
    defect; lock the repair boundary (`target` vs `preserve`); make the
    smallest change; verify the affected matrix; escalate after two rounds
    with a labeled A/B/C comparison artifact.

### The pipeline (scripts/)

`build.py` is a thin CLI shell; the implementation is split:

- `deliver.py` — **the ordered delivery gate**. Its docstring: "The individual
  checks exist so they can be debugged one at a time; a finished document
  needs all of them, in order, every time. Remembering that order is the part
  an agent drops, so this module owns it." Order: HTML checks (placeholders,
  math, markdown residue, data charts, template style, content coverage) →
  render the PDF → PDF checks (page contract, resume balance, fonts, markdown
  residue, density, orphans, page images) → one verdict (`READY` / `NOT
  READY`) plus `GAP` lines.
- `content.py` — schema validation plus **coverage**: every short atomic value
  in the content IR must survive into the visible HTML text, with an
  indeterminate verdict when visibility cannot be proven statically.
- `checks.py` — placeholders, markdown residue, orphans (last line ≤2 words /
  15 chars), density (ink coverage per page, 25% warn / 50% sparse / 60% last
  page), resume balance (83–95% fill, ≤12% gap), rhythm (no run of >5 content
  slides, dividers in large decks, density variation), card fit/orphans.
- `verify.py` — the font gate: which family actually drew the CJK text.
- `visual.py` — renders every PDF page to PNG and prints the **fixed review
  checklist** (fonts, tags, figures, page breaks, emphasis, density, wraps,
  alignment, tables, subtraction). The agent is the judge; the script
  guarantees the evidence exists and the criteria are fixed.
- `lint.py` — template style, off-palette colors, cross-template token
  consistency, doc snippets.
- `render.py`, `chart.py` (data spec → SVG, with a drift check), `math_render.py`,
  `mermaid_normalize.py`, `html_visibility.py`.
- `mcp_server.py` — a zero-dependency MCP server exposing `templates`,
  `doctor`, `render`, `check`, `deliver`, `screenshot`, so any MCP-capable
  agent can drive Kami as an engine.

### The design system

Ten invariants: parchment `#f5f4ed` background, a single ink-blue accent
`#1B365D` (≤5% of surface), warm grays only (no cool blue-grays), one serif
family per page at weight 500 (no bold), tight headlines / dense body line
heights, Chinese letter-spacing, solid tag backgrounds (never rgba — WeasyPrint
renders a double rectangle), flat surfaces, **no italic in print**. A
`references/anti-patterns.md` catalogs **58 named AI document failures** with
bad/fix pairs (content emptiness, metric fabrication, structure mimicry,
visual excess, source gaps, tone contamination, landing-page and slide
failures). `references/writing.md` carries quality bars per document type.

### The output formats

HTML + PDF by default; PPTX on request (python-pptx, real editable boxes);
Marp for Markdown-first decks; PNG for sharing/embeds; DOCX for long docs;
a white-paper print variant; social cards at exact pixel sizes. A brand
profile (`~/.config/kami/brand.md`) persists identity, brand color, language,
page size, and tone across documents.

---

## Part 3 — The comparison

### What Kami has that fleet does not

1. **A content IR and a coverage check.** Kami separates content
   (`content.json`) from layout (the template) and verifies every atomic
   content value survives into the filled page. Fleet's deck has a schema but
   no content/layout split and no coverage check; the canvas frame is opaque
   HTML with no content IR at all.
2. **The gap report.** Kami never fills a gap with a guess; it marks
   `[DATA NEEDED]` and lists every gap in the closing message. Fleet has no
   equivalent at the deliverable layer (the desk's number guard covers
   numbers only).
3. **The ordered delivery gate with one verdict.** Kami's `deliver.py` owns
   the gate order because agents drop it. Fleet's deck runs check → auto-fit →
   render; the canvas runs a11y + review. Neither has a single ordered gate
   with a `READY`/`NOT READY` verdict.
4. **The perceptual pass with a fixed checklist.** Kami renders page images and
   walks a fixed 10-item checklist. Fleet has the vision judge and the
   deterministic scan, but no fixed page-image checklist — and no
   deterministic font gate (Kami's CJK fallback "shows no fallback boxes, only
   a heavier, flatter page").
5. **Geometric checks.** Ink density per page, orphan lines, resume balance,
   slide rhythm, card fit. Fleet's deck has arithmetic layout grading
   (`CHARS_PER_LINE`, box budgets) but no density/orphan/rhythm.
6. **The anti-patterns catalog.** 58 named AI document failures with bad/fix
   pairs. Fleet's design-system anti-patterns are shorter and
   surface-specific.
7. **The feedback protocol.** The render-is-the-evidence loop: name the
   defect, lock the repair boundary, smallest change, verify the affected
   matrix, escalate after two rounds with A/B/C variants. Fleet's frame
   refine + review is similar but less structured.
8. **Per-type content schemas with quality bars.** Kami has a schema per
   template, with the quality bar in each field's `$comment`. Fleet's deck has
   one schema with bounds.
9. **The brand profile.** An owner-level persisted identity/defaults file.
   Fleet's canvas has a per-board `brand`; there is no owner-level profile.
10. **Multi-format output.** PDF, DOCX, Marp, PNG at exact pixel sizes. Fleet
    has HTML, site, PNG, and PPTX (deck). PDF and DOCX are new.

### What fleet has that Kami does not

1. **The persisted-document + single-writer architecture.** The board file is
   the truth; one writer with a named-op vocabulary; a reload, the CLI, and a
   cron job read the same artifact. Kami is stateless per document.
2. **The corpus/memory integration.** Fleet's deliverables are project
   knowledge: digests, memory facts, corpus recall, compaction survival. Kami
   has no memory.
3. **The evidence-first desk loop.** Claims → Outline → Render → Review →
   Knowledge, with sys1 extraction, a coverage judgment, a number guard, and a
   structural end gate. Kami has schemas + coverage but no evidence law.
4. **The network-dead artifact.** The site host's CSP refuses every network
   hop; self-containment is a runtime property of the serve.
5. **The compile-time AA contrast guard** (`themes.rs`) and the
   **mockups-first rule** (measured mockups before code). Kami has evals and
   page images, but no pre-code measured mockup gate and no compile-time
   contrast guard.
6. **Self-containment as a binary contract.** One static Rust binary, no
   Node/Python toolchain. Kami needs Python + WeasyPrint + Node + Pygments.

### The honest tension

Kami is **print-first** (PDF/PPTX, parchment, WeasyPrint pagination); fleet's
deliverables are **screen-first** (HTML, network-dead, the face's tokens).
Kami's toolchain is foreign to fleet's hard rule 12. So Kami is not a
dependency fleet can adopt — it is a source of ideas for fleet's own Rust
deliverable layer. The overlap is sharpest on the deck (both do
presentations/infographics/posters) and the desk (both do evidence-first
documents).

---

## Part 4 — The pieces worth absorbing

Ranked by value and fit. None of these require Kami's toolchain.

1. **A content IR and a coverage check.** A `content.json` beside the
   deliverable, validated against a per-type schema, with a check that every
   atomic value survives into the rendered page. This is the largest gap: the
   deck has a schema but no coverage; the frame has neither. It also connects
   to the component catalog — the catalog's spec *is* fleet's content IR.
2. **The gap report.** A deliverable that is missing a fact marks
   `[DATA NEEDED: what]` and reports it, rather than inventing a value. The
   desk's number guard is the precedent; generalize it.
3. **The ordered delivery gate with one verdict.** A `deliver` equivalent over
   the deck and the frame: check → render → verify → page images → one
   `READY`/`NOT READY` verdict. Fleet already has the pieces; the value is the
   order and the verdict.
4. **The perceptual pass with a fixed checklist.** Page images plus a fixed
   checklist, with the deterministic checks (contrast, font, density, orphans)
   settled mechanically and the rest left to the vision read. Fleet has the
   vision judge and `a11y_scan`; the fixed checklist and the font gate are new.
5. **Geometric checks.** Ink density, orphan lines, and slide rhythm over the
   deck's existing arithmetic grader. Fleet has no rasterizer, but the deck's
   line-budget arithmetic already models the page.
6. **The anti-patterns catalog.** A named catalog of AI deliverable failures
   with bad/fix pairs, beside the existing design-system anti-patterns. Cheap,
   and it teaches the model what to avoid.
7. **The feedback protocol.** The render-is-the-evidence loop: name the
   defect, lock the repair boundary (`target` vs `preserve`), smallest change,
   verify the affected matrix, escalate after two rounds with a labeled
   comparison artifact. Fleet's frame refine + review is the seed.
8. **Per-type schemas with quality bars.** The deck's one schema becomes a
   family, with the quality bar in each field's comment.
9. **The brand profile.** An owner-level persisted identity/defaults file, so
   a deliverable inherits the owner's brand without re-declaring it per board.
10. **Multi-format output.** PDF and DOCX for the deck and the desk. This needs
    a renderer decision (a Rust PDF writer, or a bundled tool) and is the
    largest piece — a plan of its own.

### Leave behind

- The Python/WeasyPrint/Node toolchain (hard rule 12).
- The print-first design language (fleet's deliverables are screen-first HTML).
- The MCP server (fleet's deliverables are in-process Rust with model-facing
  tools already; Kami needs MCP because it is a skill, not a binary).
- The Marp/MathJax/Pygments dependencies.

### The sequence

Content IR + coverage first (it is the component catalog's foundation). Then
the gap report and the ordered delivery gate. Then the perceptual checklist and
the geometric checks. Then the anti-patterns catalog and the feedback protocol.
The brand profile and multi-format output last. Each is a plan, not a rewrite.

---

## Part 5 — What fleet already does better

To keep the comparison honest: fleet's persisted-document-and-single-writer
architecture, its corpus/memory integration (the artifact is project
knowledge), its network-dead self-containment, its compile-time AA guard, its
mockups-first rule, and its evidence-first desk loop are all stronger than
Kami's equivalents. Kami's advantages are the content IR, the coverage and gap
discipline, the ordered delivery gate, the perceptual checklist, and the
anti-patterns catalog — the *content and verification* layer, not the
*architecture*.
