---
name: ui-component-creation-and-json-render-2026-11-07
description: "project 2026-11-07 — a deep dive on how fleet creates UI components, read against vercel-labs/json-render; landed as docs/architecture/deep-dive-2026-11-07-ui-component-creation-and-json-render.md; the owner REFRAMED the ask twice — not how json-render improves fleet's face, not wholesale adoption, but INCORPORATING THE BEST PIECES; the plan docs/plans/2026-11-07-component-catalog.md is COMPLETED and MERGED to master at de55b62 (its own worktree, gate green end to end, the mockup owner-read); the confirmed …"
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

The owner asked 2026-11-07 for two deep dives in one turn: how fleet creates UI
components, then a comparative read of `github.com/vercel-labs/json-render`
and the areas that could improve fleet. Landed as
`docs/architecture/deep-dive-2026-11-07-ui-component-creation-and-json-render.md`
(uncommitted, docs-only). No plan drafted — the owner has not chosen an area.

**Fleet's model, synthesized.** The face is a compiled-in artifact, not a build
output: `index.html` (the shell, ~280 ids), `app.css` (10,266 lines, three-tier
token chain, four themes), `app.js` (25,994 lines, one IIFE, ~600 functions,
zero imports). A component is a paint function building DOM with `node()`
(`app.js:1329`) — no props, no virtual DOM, no diffing. The registries that
stand in for a component library are `ROUTES`, `GROUPS`, `PANEL_EL`, `BUILT`,
and `ICONS`. One mutable `face` object, one reducer `apply(event)`
(`app.js:2788`), and `replay()` re-runs the same reducer over the server's
state document — **the state document and the SSE stream are the same log read
twice**, so a reload and a live update cannot disagree. Every panel's numbers
come from a bounded server read; the face never writes files. Adding a surface
touches 11 mechanical registration points (route, rail entry, `<main>`, `el`,
`PANEL_EL`, `BUILT`, arrive handler, icon, bridge read, harness case, gate).

**json-render.** A Generative UI framework: a typed catalog of components and
actions is the guardrail, the AI emits a JSON spec constrained to it, a
renderer draws it. ~30 packages, pnpm+turbo, Node ≥24. The transferable
machinery: schema-validated catalog doubling as the AI prompt, a streaming
JSONL patch compiler (SpecStream) for progressive render, a dynamic prop
language (`$state`/`$cond`/`$computed`), actions with `setState` and `watch`,
spec validation with autofix, an in-app devtools inspector, codegen from UI
trees, and an Ink terminal renderer.

**The confirmed gap (the one worth acting on).** `apply()` ends in
`default: break` (`app.js:3186`), and nothing enumerates `UiEvent`'s variants
against the client's `case` arms — the face harness does not read `src/agent.rs`.
A new Rust variant compiles cleanly (the Rust match is exhaustive) and then
vanishes on the face with no test failing. Same class as the icon drift
([[a-missing-icon-falls-back-to-dot-silently]]): a silent fallback plus a
hand-kept table. Fix shape: a source-pin test in the icon test's family — read
`agent.rs`, extract the variants, read `app.js`, extract the `case '<type>'`
arms, fail on any variant with no arm.

**The other four areas, all enhancements not yet chosen.** (2) No in-app
inspector over the state document/event log/read cache — json-render ships a
devtools panel and fleet has the raw material but no live surface over it.
(3) No machine-readable component catalog — tokens and icons are tested API
contracts, the builder layer is not. (4) The 11-point surface registration
wants a `defineSurface()` factory. (5) The transcript has no windowing
(`face.rows` accumulates to the 4096-event cap, `replay()` rebuilds the lot) —
measure before rewriting.

**Assumed gap, deliberately declined.** json-render's headline idea is one
spec rendered by many renderers. Fleet's TUI and face share the event stream
but not a rendering description, and fleet's own doc says the split is
deliberate (the TUI is the instant keyboard face; the GUI is where
terminal-unfriendly surfaces get room). Not a recommendation.

**Not applicable.** AI-authored UI, React/framework adoption (hard rules 6
and 12), and a client-side expression language — fleet's values come from the
server, which already computes them.

**What fleet already does better than json-render:** the compile-time AA
contrast guard (`themes.rs`), the mockups-measured-and-read gate, the
event-log-as-single-source-of-truth, and self-containment (one binary vs a
30-package monorepo with no design-system guard and no mockup gate).

**The owner's reframing (the second half of the ask).** "I didn't mean fleet
itself but how fleet could use this project for building ui components for
other projects." So the question is fleet-as-consumer, not fleet-as-improver.
Fleet already has a visual-deliverable path: the design canvas (`#/design`,
frames = opaque self-contained HTML pages, tweakable through `:root`
variables, versioned, reviewable, project-bound, digested, exportable,
servable at `/site/<slug>/`, with the `frame` and `review_frame` tools under
the `designer` role) and the deck (`src/deck.rs`, a checked structure with
consultant constraints that refuse before HTML renders). What neither has is
a typed component vocabulary or a multi-target render.

**The consumer fit.** Fleet's agent IS the author json-render is built for —
the catalog closes the vocabulary so the agent cannot emit an invalid
component, validation catches the rest, one spec renders to the target.
Integration shapes, cheapest first: load json-render's ~20 SKILL.md files
into fleet's skill loader (no code change); a `render` deliverable tool
alongside `frame`/`review_frame` (`{spec, catalog?, target}`); a `ui`
deliverable type checked by `validateSpec`; then the image renderer (canvas
export + gen1), the chat renderer (huddles/bots), the ink renderer (terminal
deliverables), and `@json-render/mcp` (a fleet-built UI as an interactive app
in Claude/Cursor/VS Code).

**The honest tension.** Fleet's deliverables are network-dead and
zero-toolchain (frames are opaque HTML, decks render to HTML, sites serve
static files); json-render is Node ≥24 + pnpm + React and most of its outputs
are runtime-bound. It does not replace the frame model — it adds a different
deliverable class (a live app, a document, a card) with a structured
intermediate. For other projects that is fine: the toolchain belongs to the
target project, not fleet's binary, so hard rule 12 is not violated — but it
is a shift from "the artifact is the file" to "the artifact is the project".
The static targets (SVG/PNG, email HTML, PDF) are the bridge. Risks: the
toolchain lands on the owner's other projects; the library is a fast-moving
Labs project (v0.22.0) so versions need pinning; the pre-built catalog is
shadcn's 36 components, and a custom project needs a custom catalog — the
real work.

**The owner's second reframing — the final ask (2026-11-07).** "I don't mean
adding it wholesale but incorporating the best pieces into fleet." So neither
fleet-as-improver nor fleet-as-consumer: **selective absorption**. The sharpest
framing the session produced: **fleet already proved the core idea in
`src/deck.rs`** — a rigid component schema + consultant checks that refuse + a
render is a catalog, a validator, and a renderer for presentations.
json-render's real contribution is generalizing that pattern to arbitrary
components and targets, so the work is generalizing machinery fleet already
has, not importing a framework.

Worth absorbing, in the sequence the owner was offered: (1) a typed component
catalog with typed props and descriptions, validated in Rust, plugging into
the `frame` tool and the canvas; (2) a spec as the deliverable's structured
intermediate — a JSON component tree instead of opaque HTML, making the
artifact diffable, versionable, themeable, and re-renderable (the canvas
already versions frames and carries `tweak_spec`/`brand`); (3) a refusing
validator, the deck's `check()` generalized; (4) multi-target render from one
spec (HTML frame, static site, SVG/PNG, terminal via ratatui, code scaffold);
(5) the catalog as the prompt / the `designer` role; (6) a spec inspector in
the canvas's review card; (7) a fleet-authored SKILL.md teaching the catalog,
the spec shape, the render targets, and the check rules.

Leave behind: the Node/pnpm/React runtime (hard rule 12), the client state
store and expression language (`$state`/`$cond`/`$computed` — fleet's server
computes; the deliverable is a file), the streaming SpecStream (artifacts, not
live streams), and the MCP server (fleet has its own registry).

**The plan (2026-11-07).** `docs/plans/2026-11-07-component-catalog.md`
(status draft, uncommitted) — "the component catalog — a checked vocabulary
for UI deliverables", the first piece of the absorption: a new
`src/catalog.rs` with a data-driven component table (kind → typed props +
description), a `UiSpec` tree (`root` + `elements`, json-render's flat
shape), `catalog::check` refusing an unknown kind / missing required prop /
wrong type / unknown prop / dangling child / missing root / cycle / size
bound, `catalog::render` to a complete self-contained HTML document in
fleet's tokens, the descriptions injected into the system prompt, and a `ui`
tool in `src/tools.rs`. Starter vocabulary: container, heading, text, quote,
callout, badge, metric, table, list, chart, timeline, comparison, image,
link, button, form_field. The criteria are C0–C9 with **C0 the mockup** (the
owner's "let's make sure this includes mockups" — the house mockups-first
rule: the catalog as static HTML in the app's own tokens, every kind with its
props plus one sample spec rendered end to end, measured rendered in every
theme with no overflow / no clipped text / ink ≥ 4.5:1, the PNGs read in a
light and a dark theme, and the owner reads it before any code of C1+ lands).
C1 the table, C2 the serde round-trip, C3 the three refusals, C4 the
known-good pass, C5 the render markers + size cap, C6 the prompt injection,
C7 the tool's refusal, C8 the four-leg gate, C9 the docs. Out of scope:
render targets beyond HTML, the canvas inspector, the fleet-authored skill,
any change to the `frame` tool or the canvas, the suggest mode.

**The gates on the plan.** arch-gate PASS with 8 conventions gaps (all the
"new module is undocumented" kind, covered by the linked artifacts). plan-gate
GAPS — it flags C0, C8, C9 as MISSING (stable across four runs at p≈0.28–0.36)
plus SCOPE? on Risks and Dependencies; the accepted `2026-11-07-owner-inbox.md`
carries the identical C8/C9 flags with a recorded waiver, because the coverage
model does not credit an Approach step that names them — C0 is the same class
(the pre-condition step 0 describes in detail). Proposed: waive all three. The
arch-gate's one DRIFT was real and was fixed — the first wording ("generalize
the deck's pattern") conflicted with the tech doc, which documents `deck` as
*the* structured-deliverable family and the canvas frame as opaque HTML; the
rework made the catalog a new sibling family beside `deck`, and the gate
passed. plan-surface's suggested order matched the plan's.

**Two plan-authoring traps this session confirmed.** (1) A `## Use cases`
section's `- [ ]` boxes are read by the gates as CRITERIA, shifting every
criterion number by the box count and mis-associating the coverage flags —
drop the section when the use cases are thin. (2) The plan-gate's coverage
model does not credit a criterion whose body step names it in prose; the
house's accepted plans carry the same MISSING flags on their gate/docs
criteria with a recorded waiver rather than a reword.

**IMPLEMENTED, COMPLETED and MERGED (2026-11-07, later the same session).** The
owner's goal — "implement the plans as designed in the order that makes more
sense using separate worktrees to avoid collisions with other sessions" — was
executed on the catalog plan first. Lane
`/Users/alejandrodelvillar/Projects/fleet-component-catalog`, branch
`component-catalog`, forked from master at `1eaba7a` (the dreaming merge landed
mid-work, so the base moved). Commits `a2610ed` (the implementation) and
`50a59ba` (the plan completed); **merged to master at `de55b62`** after merging
`origin/master` in (the self-improvement work had landed while the lane ran —
master moved to `f6f5f0d`, the merge was clean, no conflicts, and the merged
tree's gate was green). The lane was pruned with
`scripts/plan-worktree.sh remove component-catalog` — 12.2 GiB freed. The
owner's mockup read is the gate that closed C0; the plan's status is
`completed` with all ten criteria ticked.

What landed: `src/catalog.rs` (the sixteen-kind table, the flat `UiSpec`,
`check`, `render`, `prompt_text`), the `ui` tool in `src/tools.rs` (validate →
render → save beside the session → artifact card), the dispatch + prompt
section + closed-set declaration in `src/agent.rs`, 18 catalog tests plus the
tool handler, `docs/features/catalog.md`, the tech doc's module reference and
`tools.rs` section, and the functional spec's deliverable paragraph. The
mockup was drawn and measured BEFORE the code (four themes: no overflow, no
clipped text, ink ≥ 4.5:1), and the measurement caught one real defect — the
primary button's white-on-accent failed AA in the dark themes, fixed with an
`--on-accent` token (white on light, dark on the bright accents), the face's
own `.dk-btn.primary` pattern.

Two repairs beyond the plan's criteria, both recorded in the commit:
- **The tool-surface bound's fourth crossing** — the `ui` declaration put the
  surface at 10,224 against 10,050, so `MAX_DEFINITION_TOKENS` moves to 10,300
  with the crossing named in the constant's own history, the protocol it
  documents ([[a-declaration-budget-is-a-measurement-too]]).
- **A date-boundary defect in the dreaming test** — the retire path stamps
  `valid-until` from the real UTC day (`calib::today()`), but the test asserted
  against a hardcoded `2026-11-07`; it failed at the 2026-11-08 UTC rollover
  (confirmed failing on the main checkout without my changes). The assertion
  now follows the real day. See [[a-date-boundary-breaks-a-hardcoded-day]].

The gate is green end to end (fmt, clippy, release build, test). One
environment note: two lane tests need `BROWSER_CHROME` pointed at the owner's
Chrome (the install doc's own note — "Chrome is a machine resource, not
installed here"); with it set the suite is fully green, without it
`the_lanes_command_reports_every_real_lane` and
`a_lane_home_override_shadows_the_bundled_copy` fail on master too.

**Two validator repairs made after the first green run, both worth carrying.**
(1) **A shared child is not a cycle.** The first reachability walk refused any
element reached twice, so a container with two parents sharing a child was
reported as "the tree has a cycle". The spec IS a tree, so a shared child is a
real defect — but it must be named apart from a cycle ("a shared child draws
twice"), and the walk needs an ancestry set (the path) plus a visited set (the
whole tree), not one set for both. (2) **The chart's bar height is clamped**
(`(value/max*100).clamp(0,100)`) so a negative or out-of-range series value
cannot emit a negative height.

See [[gui-client-verified-by-a-vm-harness]], [[every-rail-icon-audited-2026-11-04]],
[[memory-dreaming-comparison-2026-11-07]], [[self-containment-is-a-standing-contract]],
[[fleet-deliverables-and-kami-2026-11-07]] (the same comparison pattern applied
one layer down — fleet's deliverable machinery against tw93/Kami),
[[a-date-boundary-breaks-a-hardcoded-day]],
[[a-contrast-measurement-needs-the-alpha-channel]].
