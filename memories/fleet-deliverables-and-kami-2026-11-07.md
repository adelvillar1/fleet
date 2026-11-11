---
name: fleet-deliverables-and-kami-2026-11-07
description: "project 2026-11-07 — a deep dive on how fleet creates deliverables, read against tw93/Kami; landed as docs/architecture/deep-dive-2026-11-07-fleet-deliverables-and-kami.md (committed to master at de55b62 with the component-catalog merge); fleet's five deliverable kinds (canvas frames, decks, send_file, desk deliverables, huddle/session artifacts) against Kami's content IR + coverage + gap report + ordered delivery gate + perceptual checklist; the pieces worth absorbing are the content-and-verification layer…"
type: project
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

The owner asked 2026-11-07, after the json-render comparison, to "use the same
comparative analysis pattern for how fleet creates deliverables and this repo
https://github.com/tw93/Kami". Landed as
`docs/architecture/deep-dive-2026-11-07-fleet-deliverables-and-kami.md`
(committed to master at `de55b62` with the component-catalog merge, docs-only).
No plan drafted — the owner has not chosen a piece.

**Fleet's deliverable taxonomy — five first-class kinds**, each with its own
store, creator, and receipt, sharing house rules (a home-persisted document, a
named refusal, a substrate trace) but a bespoke pipeline each:

1. **Design frames** (the canvas). `canvas::Frame` (`src/canvas.rs:139`) — one
   opaque, self-contained HTML page with `tweak_spec`, versions, pins, review.
   A board is `canvas::Project` (`src/canvas.rs:193`) at
   `<home>/canvas/<slug>/project.json`. Created by the `frame` tool
   (`src/tools.rs:304`) → `canvas::frame_from_tool` (`src/canvas.rs:3388`).
   Received on `#/design`, the preview route, exports (html/tokens/zip/png/
   site/tsx/set), downloads, and the network-dead host `/site/<slug>/`.
2. **Decks** (`deck::Deck`, `src/deck.rs:145`). Presentations / infographics /
   posters; HTML + a real `.pptx` beside the session log (`src/tools.rs:636`).
   Created by the `deck` tool (`src/tools.rs:379`) →
   `tools::run_deck_with_event` (`src/tools.rs:477`).
3. **Files handed over** (`send_file`, `write_file`). `send_file` is "the one
   tool whose result is for the owner rather than for the model"
   (`src/tools.rs:1117-1132`); emits `UiEvent::Artifact` (`src/agent.rs:4899`).
4. **Desk deliverables** (`src/desk.rs:184`). An evidence-first five-stage loop
   (Claims → Outline → Render → Review → Knowledge, `src/desk.rs:114`) with a
   verified claim set, a coverage verdict, a number guard, and a digest. Created
   by the owner's brief via `POST /api/desk`, not by a model tool.
5. **Huddle / session deliverables.** The same `send_file` artifacts plus a
   room's goal converted into a desk deliverable; the session shelf is
   reconstructed from the log (`session_deliverables`, `src/gui/bridge.rs:7844`).

**The model-facing deliverable tools:** `frame`, `review_frame`, `deck`,
`send_file`, `write_file` — each with a ToolSpec, a handler, and a refusal set
that names the rule (empty/no html, malformed tweak_spec, unknown board, asset
MIME/size/count, incomplete document, over 512 KiB, unresolved `{{img:}}`,
Pyramid/MECE/bolding/bounds/identity, no run root, path outside the root).

**The canvas pipeline.** The writer is the only mutation path:
`design_write_in` (`src/gui/bridge.rs:8892`) with a named-op vocabulary; every
write crosses `canvas::save` (`src/canvas.rs:325`), the single funnel that
regenerates the digest. Caps: 512 KiB HTML, 20 versions, 10 reviews, 256 KiB
per asset, 64 assets. Deterministic checks: `a11y_scan` (`src/canvas.rs:3998`)
over the page's own `:root` tokens, `coherence`, `token_diff`, the SVG
sanitizer, MIME sniffing by magic bytes. Review is two-track: the deterministic
scan plus a vision read with eight scores (`src/canvas.rs:3678-3718`).

**The deck pipeline.** `deck::check` (`src/deck.rs:344`) refuses Pyramid,
Overlap (MECE, ≥0.6 content-word overlap), Unemphasised (no bold), Bound,
Identity — each refusal names the node and the rule. `deck::review` (`:848`)
is a layout grader, deliberately arithmetic because a static Rust binary has no
HTML rasterizer (`CHARS_PER_LINE = 52` against a 13-line box). `deck::auto_fit`
(`:1047`) repairs in place and reverts wholesale if a pass introduces a new
refusal. `deck::render` (`:503`) emits `deck-*` classes that reference the
face's alias tokens, so a deck inherits the theme.

**The substrate chain.** A bound board's `save` writes a deterministic digest at
`canvas/digests/<project>/<slug>.md`; the corpus-refresh cron jobs embed those
digests (`src/cron.rs:839`); a run born in that project reads a bounded design
section from them (`src/canvas.rs:507`, wired at `src/agent.rs:2720`); board
lifecycle moves a `design-board-<slug>` fact through the memory writer. The
desk mirrors this exactly.

**The distinctive decisions.** (1) The deliverable is a persisted document, not
a message attachment — the board file is the truth, the face is a client of one
writer. (2) The writer is the only mutation path, a named-op vocabulary. (3)
Refusals are first-class, named, and model-visible, and the deterministic check
runs before the artifact lands. (4) The artifact is project knowledge, not just
an output. (5) Review is two-track and bounded. (6) The network-dead artifact —
the site host's CSP refuses every network hop, so self-containment is a runtime
property of the serve.

**Evidence gaps found in fleet's own machinery:** `viewport` is read by the
handler and instructed by the designer role but NOT declared in the `frame`
ToolSpec; external-URL refusal is doctrine not code (`validate_frame_html`
checks only doctype and size — enforcement is the site CSP plus asset
inlining); deck cards are live-only (no `Record::Deck` in the session log, so a
reload loses the card, unlike `send_file`); the desk's structured references are
claimed but not implemented (`SourceKind` is only `File|Fact|Url`); the design
review ask is not yet exercised end to end.

**Kami.** tw93's document design system, delivered as an agent skill: nine
templates (one-pager, long doc, letter, portfolio, resume, slides, equity
report, changelog, social cards) plus a landing page, in EN/CN/KO, rendering to
PDF/PPTX/Marp/PNG/DOCX. Python (WeasyPrint, python-pptx, PyMuPDF, optional
Pygments/Node). Its agent workflow is the interesting part: lock the contract, a
question budget, a content IR (`content.json` + per-type schemas), a gap report
that never invents, "read the right amount", fill the template, one `--deliver`
command that runs every gate in order and prints one verdict, then a visual
pass over page images with a fixed checklist. It owns 58 named AI document
anti-patterns and a render-is-the-evidence feedback protocol.

**What Kami has that fleet does not:** (1) a content IR and a coverage check —
`content.json` beside the deliverable, validated against a per-type schema,
with a check that every atomic value survives into the rendered page; (2) the
gap report — `[DATA NEEDED: what]`, never a guessed value; (3) the ordered
delivery gate with one verdict (`deliver.py` owns the order because agents drop
it); (4) the perceptual pass with a fixed checklist and a deterministic font
gate (a missing CJK serif "shows no fallback boxes, only a heavier, flatter
page"); (5) geometric checks (ink density, orphan lines, resume balance, slide
rhythm, card fit); (6) the anti-patterns catalog (58 named failures with
bad/fix pairs); (7) the feedback protocol (name the defect, lock the repair
boundary `target` vs `preserve`, smallest change, verify the affected matrix,
escalate after two rounds with A/B/C variants); (8) per-type schemas with
quality bars in each field's `$comment`; (9) a brand profile; (10) multi-format
output (PDF, DOCX, Marp, exact-pixel PNG).

**What fleet has that Kami does not:** the persisted-document + single-writer
architecture; the corpus/memory integration (the artifact is project
knowledge); the evidence-first desk loop; network-dead self-containment; the
compile-time AA contrast guard; the mockups-first rule; one static binary with
no Python/Node toolchain.

**The honest tension.** Kami is print-first (PDF/PPTX, parchment, WeasyPrint
pagination); fleet's deliverables are screen-first HTML. Kami's toolchain is
foreign to hard rule 12. So Kami is a source of ideas, not a dependency — the
overlap is sharpest on the deck (both do presentations/infographics/posters)
and the desk (both do evidence-first documents).

**The pieces worth absorbing, ranked:** (1) a content IR and a coverage check —
the largest gap, and the same idea as the component catalog's spec; (2) the gap
report; (3) the ordered delivery gate with one verdict; (4) the perceptual
checklist + deterministic font/contrast gate; (5) the geometric checks over the
deck's arithmetic grader; (6) the anti-patterns catalog; (7) the feedback
protocol; (8) per-type schemas with quality bars; (9) the brand profile; (10)
multi-format output last (it needs a renderer decision — a plan of its own).

**Leave behind:** the Python/WeasyPrint/Node toolchain (hard rule 12), the
print-first design language, the MCP server (fleet's deliverables are in-process
Rust with model-facing tools already), and the Marp/MathJax/Pygments
dependencies.

**What fleet already does better than Kami:** the persisted-document-and-single-
writer architecture, the corpus/memory integration, the network-dead
self-containment, the compile-time AA guard, the mockups-first rule, and the
evidence-first desk loop. Kami's advantages are the content IR, the coverage and
gap discipline, the ordered delivery gate, the perceptual checklist, and the
anti-patterns catalog — the content-and-verification layer, not the
architecture.

See [[ui-component-creation-and-json-render-2026-11-07]] (the same comparison
pattern one repo earlier, and the component-catalog plan it produced),
[[deliverables-core-handoff-2026-11-04]], [[knowledge-work-core-named-desk]],
[[self-containment-is-a-standing-contract]].
