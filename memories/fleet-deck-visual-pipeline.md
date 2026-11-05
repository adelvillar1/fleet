---
name: fleet-deck-visual-pipeline
description: "owner directive 2026-10-17 — fleet must excel at infographics and presentations; a 3-stage research/structure/render pipeline, adapted to the Rust crate contract (python-pptx refused, HTML-first rendering); COMPLETE 2026-10-17 — schema, HTML renderer, agent tool, intent-router skill, PPTX export, the router's vision workload, image-capable API layer, the vision tool, and an arithmetic layout grader; the browser is the rasterizer and the owner the bridge to the model's eyes"
type: project
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The owner's directive: "we also need this to excel at infographics and
presentations" — to match systems like Kimi and Codex. They proposed a
3-stage agentic blueprint: (1) a research & expansion agent that routes by
format, runs a search/RAG loop and fact-checks; (2) a structure & content
planner that forces content into atomic JSON nodes with consultant
constraints (Pyramid Principle, MECE, bolding); (3) layout & rendering
engines (JSON-to-PPTX, text-to-image for focal graphics, and a vision-LLM
feedback loop that judges a rendered thumbnail and rewrites coordinates).

**The blueprint's stage 3 cannot be taken as written.** python-pptx and
LangChain violate the crate contract the owner themselves set — one static
Rust binary, no Node runtime, no checkout dependency (TECHNICAL-
DOCUMENTATION.md). The adaptation agreed in conversation:

- **Stage 1 largely exists.** The agent already searches, recalls over the
  memory substrate and fact-checks. The missing piece is an *intent router*
  that separates infographic (one high-density frame) from presentation
  (paced narrative) from poster (read across a room) — now a `ProjectType`
  the planner routes on — plus a research skill.
- **Stage 3 renders HTML first, not PPTX.** The browser face already
  renders HTML with a three-tier token chain, so a deck as an HTML surface
  is native, needs zero new dependencies, and previews in the right-hand
  pane. PPTX becomes a later *export*: it is a ZIP of XML, so a minimal
  emitter can be hand-rolled with an uncompressed ZIP and no new crate.
- **Image generation is a provider decision, not an architecture one** —
  FLUX/SDXL need a key and network, which is a roster addition governed by
  the no-credential-in-browser rule.
- **The vision feedback loop goes last**, once there is something to judge;
  GLM-5.3-Flash is the available multimodal model.

**Stage 2 shipped as `src/deck.rs` (commit 2842d61, 2026-10-17)** — the
stage the owner correctly identified as the secret, and the only one of
the three needing nothing from outside itself. A rigid schema
(`ProjectType`, `Layout`, eight atomic `Component`s, self-contained
`Node`s, `Deck`) that round-trips through JSON so content and rendering
never share a Rust type. The consultant constraints are **checks that
refuse, not suggestions in a prompt**, because a constraint a model can
ignore is decoration: the pyramid (a headline must assert, so a topic or a
question is refused by name), MECE (two sub-points sharing 60% of their
content words are one point, so the pair is named), bolding (a bullet
with no emphasis would be missed by a skim), and per-type bounds
(infographic = one frame, presentation = 2–40 paced nodes, poster = one
frame). Every refusal names the node and the rule so the agent fixes one
thing rather than starting again.

**The `kimi-slides` skill in `~/.agents/skills` already encodes stages 1
and 2 as a prompt workflow** for an agent with bash and Python (a .pptd
YAML DSL over OOXML plus a companion CLI). That works in zcode; in fleet
it would mean shelling out to a Python CLI, the checkout dependency the
contract forbids. The knowledge transfers — the pptd DSL, the
pyramid/MECE discipline, "read the context thoroughly" — but the execution
must be native. That is what `deck.rs` starts.

**Why:** the GUI revamp made fleet's hidden intelligence visible; this is
the next capability gap — a harness that matches Kimi/Codex has to produce
considered visual deliverables, and the difference between considered and
generated is the structure stage, not the renderer.

**How to apply:** build in the agreed order — **HTML renderer** (stage 3,
native, immediately previewable in the pane) → **agent tool** so the model
can build and validate a deck mid-run → **intent router** as a skill →
**PPTX export** (hand-rolled uncompressed ZIP) → **vision feedback loop**
last. Never add a crate for this; never shell out to Python. Related:
[[gui-revamp-browser-face-decision]], [[canonical-skills-library-agents-skills]],
[[fleet-gui-skills-manager-ruling]].

**Update 2026-10-17: stages 3 and the agent tool are DONE — by the other
session.** `6cc0b80` ("the render stage and the agent tool") landed the HTML
renderer and a `deck` tool (`DECK_TOOL` in tools.rs) so a run can build and
validate a deck mid-run, plus `UiEvent::Deck` carrying the fragment and
`Entry::Deck(DeckCard)` in the TUI. So the first two items of the order above
are closed, and what remains is the **intent router as a skill**, **PPTX
export**, and the **vision feedback loop** last. Their renderer consumes the
schema shape rather than re-deriving it — see
[[fleet-deck-two-sessions-collide]] for the coordination facts and for the
`DECK_TOOL` test-count breakage their commit left behind.

**The renderer's load-bearing property:** it references the face's own alias
tokens (`var(--dsw-alias-*)`) rather than a palette of its own, so a deck
rendered inside the browser face inherits the theme switch for free — which is
why the renderer carries no CSS and the `deck-*` classes live in the
stylesheet. Content is escaped on the way out, because a deck renders into a
page that also runs a conversation. And the tool splits its output: the summary
rides back to the model and the fragment rides a `Deck` event to the surface,
because the two audiences want different things — the model wants to know it
passed, the owner wants to see the deck. In the TUI the same event lands as a
card saying what was built and where to see it, since a terminal cannot render
a fragment. Note the attribution between the two sessions is tangled — see
[[fleet-deck-two-sessions-collide]].

**Update 2026-10-17 (later): stage 1, the PPTX export and the vision workload
are in; the vision loop is blocked on an API-layer gap.**

- **Stage 1 shipped as a skill, not code.** `visual-deliverable-brief` in
  `~/.agents/skills` is the intent router the blueprint asked for, in the shape
  fleet ships procedural knowledge: classify the ask into
  infographic/presentation/poster from *what the owner will do with it* (an
  infographic or poster is **one node with many components**, not many nodes —
  the mistake the type exists to prevent), then decide how much research it
  needs (a topic with no numbers means read the tree; numbers or dates mean
  search; anything about this project means recall first, because a deck
  contradicting an owner ruling is worse than no deck), then structure with the
  three constraints as working method, then choose the layout from the content
  rather than the other way round, then read it back — headlines in order, then
  the bold words, and if that does not deliver the argument the deck is wrong
  even though it passed every check. "The checks are the floor, not the goal."
- **PPTX export shipped as `deck::to_pptx` (04a702b).** A hand-rolled
  uncompressed ZIP of the OOXML parts a PowerPoint-compatible reader needs —
  stored rather than deflated, because the crate contract is no new dependency.
  Text rides as real text boxes at positions **derived from the layout the
  planner chose**, so a `split_two_column` node's evidence really sits beside
  its conclusion; the theme carries the console's own palette. Verified with
  readers rather than asserted: `unzip -t` passes every part's CRC, all ten XML
  parts parse, and a slide carries four text boxes with the headline bold and
  the `**` marker stripped.
- **The `vision` workload joined the router's preference table** (GLM-5.3-Flash
  at the head — the owner's pinned multimodal model, declaring text/image/video
  — then step-5-preview, whose own input declaration carries images). It is one
  more row the owner can retune in `routing.json`, which is what "whatever
  model the fleet router has configured for vision related tasks" means in
  practice. Note the sys1 lane is a vendored Python MCP server, so its workload
  vocabulary is outside the repo; the vision tool therefore routes with an
  **explicit** workload, the way the orchestrator pins `hard`.
- **The vision loop is NOT built, and the blocker is named rather than papered
  over: fleet's API client is text-only.** `Message::content` is an
  `Option<String>` with no image part anywhere in `api.rs`, so no model can be
  sent a picture yet. The workload is the routing half only. The API-layer half
  — multi-part content across every provider's disagreeing image format
  (OpenAI's `image_url`, Anthropic's `image` blocks, GLM's, stepfun's) — is the
  next piece and is **not small**. A screenshot is the second prerequisite:
  fleet has no browser engine, so something must rasterize the deck (a `vision`
  tool accepting an image path the owner already has, or driving the
  `computer_use` lane).

- **The vision loop's API-layer blocker is CLOSED — `966b0cd` (2026-10-17).**
  `Message` now carries `images: Vec<ImageRef>` beside its text. The finding
  that collapsed the hard part: **every candidate on the roster is an
  OpenAI-compatible `/chat/completions` endpoint** (zai's paas/v4, mimo's /v1,
  stepfun's /v1, alibaba's compatible-mode, liquid's /v1, deepseek's), so there
  is exactly ONE parts array to write, not five provider-specific image
  formats. Both directions are hand-written rather than derived, because a
  derive would have forced a single shape and there are two that matter: with
  no images the content is the string it has always been (a text message's
  wire format and log payload are byte-for-byte unchanged); with images it is
  the parts array — a text part then one `image_url` part per image carrying a
  base64 data URL. Reading back splits the array and decodes the bytes, so the
  log holds what was sent rather than a derived URL, and an image that arrived
  by http reference is dropped rather than stored as though it were inline,
  because a log that pretends otherwise lies on replay. Base64 and its inverse
  are hand-written (no new crate); the inverse refuses input that is not base64
  rather than decoding it to nothing. Verified: RFC's own base64 vectors both
  directions, a text message still serializes as a plain string with no
  `images` key, and the round trip returns identical bytes.

- **The screenshot source is IN — the `vision` tool shipped as `751e1cb`
  (2026-10-17).** It takes a question and image paths, reads them, and asks
  the roster's multimodal model. It rides every run (it reads a file the
  owner already has and touches nothing outside itself) and — the
  load-bearing part — **the route is the `vision` workload's own chain, not
  the run's**, so a vision ask reaches a model that can see whatever the
  task was judged as: a run routed to `quick` for a text question can
  still grade a rendered deck. The images are read under the same boundary
  the browser's file pane holds, re-implemented in `agent.rs` rather than
  shared — the working directory is the root, a `..` escape refused
  **lexically before anything touches the disk**, `.env` refused by name,
  bytes capped at 8MB. A question with no readable image is a
  model-visible error rather than a call asking a model to look at
  nothing. The roster for the second route comes off the run's own router
  (`Router::resolved()`, added for this) rather than resolving the owner's
  config a second time, so the two routes cannot disagree about who is on
  the roster. One gap named in the commit rather than papered over: the
  chain is the compiled-in default, because the agent does not carry the
  preference table, so an owner's `routing.json` override of the vision
  workload's candidates does not yet govern this route.

- **The grader is IN — `8a46cbd` (2026-10-17) — and the pipeline is complete.**
  `deck::review` grades the blueprint's three failure classes, and it is
  deliberately **arithmetic rather than a model call**. Contrast is already
  guarded better than a grader could do it — the theme guard *fails the build*
  when any theme's text drops under 4.5:1, and a runtime check cannot beat a
  compile-time one — so what is left is overlap (estimated lines against the
  lines a box holds) and balance (one column's weight against its neighbour's,
  flagged at 3x). Every estimate is against a **named** nominal column and line
  height, because an estimate against an unknown width is a guess. The `deck`
  tool runs it *after* the constraints pass: the checker is the floor, the
  review is the ceiling, so a deck that would render badly is refused at the
  tool rather than shipped and seen to be bad.
- **The owner corrected my framing of the screenshot gap, and it improved the
  design.** I had said "fleet has no browser engine, so there is no screenshot
  for a vision model to look at." Their reply: "Isn't the entire app itself a
  server that renders the gui." The engine was never missing — it lives one
  component over. So a deck now renders **full-page in the pane** rather than
  only in the inspector drawer, which is what makes a screenshot one keystroke;
  that screenshot is the image the `vision` tool grades. The loop divides
  honestly: arithmetic for what is computable, the model for what needs
  judgment, the browser as the rasterizer between them, and the owner as the
  bridge. See [[name-the-component-that-has-the-capability]].

**Where it stands:** every stage of the agreed order is in — schema and
refusing constraints, HTML renderer, agent tool, intent-router skill, PPTX
export, the router's `vision` workload, image-capable API layer, the
`vision` tool, and now the grader. What is *not* closed, named rather than
papered over: the vision route reads the compiled-in default chain because the
agent does not carry the preference table, so an owner's `routing.json`
override of the `vision` workload does not yet govern it; and the grader is
estimating, not measuring — a real font-metrics pass or a browser-measured
render would tighten it. Flagged for the looks session: the deck's full-page
view is the first surface where a rendered artifact and the conversation share
the pane, and it currently inherits the pane's own card chrome — whether a deck
should render on a clean ground of its own is a design call to make
deliberately.

**Update 2026-10-17 (latest): the next gap is layout GEOMETRY, not models.**
The owner proposed an ultra-low-latency triad for this pipeline (EmbeddingGemma 2
+ Jev/GLiDE + a small LLM) with five use cases — brand-aware theme injection,
cross-asset sourcing, spatial squishing/layout swapping, Matryoshka-truncated
RAG, adaptive clustering. The triad already exists as **blend1** (see
[[blend1-is-the-latency-triad]] for the exact lane mapping), so nothing new is
needed on the model side. What *is* missing is what three of those five use
cases depend on: `deck::Layout` is a shape enum and `Component` is content with
**no geometry at all**, so nothing in fleet can answer "does this text fit this
slot" as a number. The proposed next increment: give each layout real slots —
a width in characters and a height in lines — and have the planner **measure**
the drafted content against them, refusing or re-routing before render,
deterministically, with the model only re-drafting when a slot genuinely cannot
hold the argument. A Matryoshka 128-dim prefilter is nearly free on top of that
(`vector_info()` already reports the dims; truncating is a slice); an asset
library to search is the one genuinely new corpus. Also pushed back on: the
blueprint's "small LLM truncates strings to fit max_chars" spends a model call
on arithmetic — the LLM writes concisely up front, the check verifies, and the
cut is `&s[..n]`.

**Update 2026-10-17 (latest): the "layout GEOMETRY is the next gap" claim above
is SUPERSEDED — the geometry is already built.** Going to build slots and a
measurement primitive, I found `deck::review` already answers it: it estimates
each component's lines against a box's using **named nominal constants**
(`LINE_PX`, `BOX_PX`, `CHARS_PER_LINE`, `HEADLINE_HOLDS`) and reports
`Overfull` / `Unbalanced` / `HeadlineOverflow`, and it is **wired** — `run_deck`
runs parse → `check` (consultant constraints, the floor) → `review` (the ceiling)
→ `render`. So "nothing in fleet can answer 'does this text fit this slot' as a
number" was already false when it was written.

**I nearly shipped a second measurement rule, and the save was reading the
co-edited file first.** I had written `Slot`/`Takes`/`Fit`/`measure`/`budget` and
`Layout::slots()` before noticing it duplicated the grader's `component_lines` —
the house rule is one rule, not two that can drift. Reverted the whole thing,
including a `Code` variant I had added to `Component` that would have broken
their renderer's match arms. See
[[read-a-coedited-file-before-adding-to-it]].

**The one thing the grader does not yet do is a re-layout ladder.** On overflow
it reports the fault but offers nowhere roomier to go. The small addition worth
making is a `roomier()` on `Layout` (a `SplitTwoColumn` or `Comparison` node that
overflows suggests `FullBleed`) folded into `LayoutFault` as a variant — that is
half of what an "auto-compress or swap template" action card needs, the other
half being the compression itself, which is `&s[..n]` against the named
constants. **SUPERSEDED 2026-10-19: both halves are built (see below).**

**The triad's self-correction loop is BUILT — `auto_fit` (25cdbaf, 2026-10-19,
KI-251 closed; the owner was right: "it was supposed to be built").** The
grader refused overflow and stopped — the blueprint's step 4b existed nowhere.
Now `deck::auto_fit` runs the deterministic repair inside `run_deck`: text
compressed to the grader's own measured budgets (holds × CHARS_PER_LINE, now
public) with the **bolded takeaway kept whole** (`compress_text` clips around
the `**` span), headlines trimmed with the pyramid re-verified per candidate,
heaviest columns balanced toward their lightest, and `Layout::roomier()` — the
swap ladder (split_two_column/comparison/timeline/metric_row/chart →
full_bleed) — used when compression would cut more than the content can spare.
The load-bearing rule: **the consultant verdict may not change** — `auto_fit`
snapshots the input's `check()` baseline and reverts any pass that adds a
refusal (a bare `check().is_empty()` revert would break on decks with
pre-existing deck-level refusals, which is exactly how the 1-node test fixture
exposed it). `run_deck` renders the repaired deck with each repair named
beside the verdict; only what the repair cannot clear returns to the agent,
naming the budgets (13 lines × 52 characters) to compress against. Pinned by
three auto-fit tests plus `the_deck_tool_auto_fits_an_overfull_deck_and_renders_it`.

**KI-252 (OPEN) — the triad's vector half has no deck-pipeline wiring**: no
template index, no cross-modal asset library, no Matryoshka slice, no
clustering step (the blueprint's use cases 1, 2, 4, 5). The owner named the
models: ollama cloud gemma4, the sys1 pool, the local EmbeddingGemma 2 — the
embedder exists (127.0.0.1:8901) and blend1 already routes the triad; what is
missing is deck-side wiring. Agreed three-move build order, each gated: (1)
the template index — embed filled layout examples, match a draft's intent by
cosine beside the intent skill's classification (smallest; unlocks the owner's
use case #1); (2) the Matryoshka pre-filter — 128-dim slice for the recall
fan-out, full 768-dim for the final rank, measured against today's recall;
(3) the asset library — icons/graphics keyed by embedded captions, grader
certifies aspect and spacing before placement.

**Live end-to-end audit, 2026-10-19** (the owner, burned by the rail strip, asked whether the deck features were actually implemented — verified, mostly yes):

- **Verified working, live**: a real `fleet run` loaded `visual-deliverable-brief` on turn 1, researched genuinely (Cargo.toml cross-checked against Cargo.lock: 7 direct deps, 248 lock packages, indexmap absent), called `deck`, was **refused once by the grader's balance rule** (`columns [9, 7, 3] — one side is carrying the node`), trimmed, re-called, and passed. The rendered fragment viewed with app.css in the browser is a real infographic — accent metric callouts, conclusion headline, bolded bullets, prose column.
- **`to_pptx` is product-unreachable.** The function is real (valid OOXML zip: PK signatures, EOCD, content-types, master, layout, theme — all test-asserted) but has **no caller**: no CLI command, no agent path, no GUI route. "PPTX export shipped" means the library function shipped; the owner still cannot produce a .pptx file.
- **The turn cap eats deck runs.** The first run spent all 25 turns researching (including reading deck.rs's source to learn the schema that is already in the tool spec) and hit the cap before calling `deck`; resuming the session completed it in one call. A deck prompt should tell the agent the facts are enough — or the cap needs a deck-shaped allowance.
- **The vision loop is owner-mediated by design** (render → owner rasterizes in the browser → `vision` tool reads the PNG path). The automated rasterize→vision→rewrite loop needs the browser lane (the owner-directed Chrome-over-CDP proposal) and is not wired.
- **Headless runs don't carry the render**: `run_deck` returns the review line + HTML in the ToolOutcome, but the transcript shows the summary and the HTML rides the GUI's `Work::Deck` surface event — a headless owner sees the verdict, not the deck.
