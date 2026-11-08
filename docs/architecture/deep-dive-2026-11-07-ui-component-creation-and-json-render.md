# Deep dive 2026-11-07: how fleet creates UI components, read against vercel-labs/json-render

Research only — no plan, no code. The question: what does fleet's UI-creation
model actually look like, what does json-render's look like, and which of
json-render's ideas could make fleet better without breaking fleet's own
contracts.

The two projects are philosophical opposites, so the value is in the
*contract discipline*, not the code. Fleet's hard rules (no Node, no
framework, one static binary, self-containment) rule out adopting anything
wholesale; the findings below are the transferable ideas only.

---

## Part 1 — Fleet's UI creation model

### The stack: a compiled-in artifact, not a build output

The face is plain web-platform code embedded in the Rust binary at compile
time (`src/gui/mod.rs:1-11`, `server.rs:71-103`). There is no framework, no
bundler, no module system, and no Node at runtime. The only build-time step
is `build.rs:17-52`, which computes an FNV-1a stamp over the served assets
and emits it as a cache coordinate.

| File | Lines | Role |
|---|---|---|
| `src/gui/assets/index.html` | 1,264 | the shell: rail, topbar, palette, every `<main class="stage">`, pane, drawer; ~280 ids |
| `src/gui/assets/app.css` | 10,266 | tokens, themes, component classes, breakpoints |
| `src/gui/assets/app.js` | 25,994 | the entire client: one IIFE, ~600 functions, zero `import`/`export` |
| `src/gui/assets/sw.js` | 159 | service worker |
| `src/gui/assets/bot.{html,css,js}` | 831 (js) | a second, assistant-first face over the same bridge |
| `src/gui/assets/vendor/*` | — | DOMPurify, PDF.js, docx-preview, JSZip, with `VENDOR.json` provenance |

The server is a hand-rolled HTTP/1.1 loop over `tokio::net::TcpListener`,
loopback-only, with `Host`/`Origin` checks on every request
(`server.rs:130-248`). The machine is the authentication boundary.

### What a "component" is

A component is a **paint function** that imperatively builds DOM with a
`node()` helper and appends it to a container (`app.js:1329`):

```js
function node(name, className, text) {
  var made = document.createElement(name);
  if (className) made.className = className;
  if (text != null) made.textContent = text;
  return made;
}
```

There is no `render()` returning a tree, no props object, no PropTypes, no
TypeScript, no virtual DOM, no diffing. Builders take positional arguments;
state reaches them by closure over the single `face` object and the `el` map.

The reusable builder library (the closest thing to a component library):

- **Surfaces**: `pageHead` (`:5035`), `surfaceHead` (`:6417`), `surfaceStrip`
  (`:6428`), `surfaceInstrument` (`:6440`), `surfaceRail` (`:6462`),
  `boardCard` (`:5016`)
- **Counts**: `statStrip` (`:5063`), `tile` (`:5068`)
- **Filters**: `filterBar` (`:5119`)
- **Rows**: `card` (`:1506`), `badge` (`:6410`), `rowAction` (`:6468`),
  `sectionRail` (`:19805`) — the shared left rail for memory / skills /
  calibration / settings / bots / boards
- **Settings inputs**: `cfgField`, `cfgSelect`, `cfgCombo`, `cfgChips`,
  `cfgSwitch` (`:19909-20089`)
- **Inspector**: `renderSections` (`:8393`), `openDrawer` (`:8407`)
- **Content**: `markdown` (`:24612`) — a hand-rolled block parser, then
  DOMPurify as a second layer
- **Charts**: `dayChart` (`:5148`), `donut` (`:5205`), `svgPlot` (`:5881`),
  `treeCard` (`:6008`), `graphCard` (`:6155`) — hand-rolled SVG

### The registries that stand in for a component registry

| Registry | Location | Maps |
|---|---|---|
| `ROUTES` | `app.js:238-255` | 16 hash addresses → labels |
| `GROUPS` | `:266-299` | 4 groups × destinations; rail and palette both derive from it |
| `PANEL_EL` | `:23653-23673` | address → the `<main>` element |
| `BUILT` | `:25257-25259` | addresses the palette will not badge "later" |
| `ICONS` | `:480-741` | 57 drawn keys, 24 viewBox, 1.6 stroke |
| `el` | `:750-1023` | id → element |

### The state model: one log, read twice

One mutable `face` object holds every piece of client state
(`app.js:1037-1281`). Every live event goes through one reducer,
`apply(event)` (`:2788-3191`). On load, `replay(state)` (`:8560-8727`)
reseeds `face` from the server's state document and runs the same reducer
over the stored event log.

**The state document and the SSE stream are the same log read twice**, so a
reload and a live update cannot disagree. This is why there is no client-side
state library: the state is the log, and the log is on the server.

### The bridge

`UiEvent` (`src/agent.rs:656-908`) is the RPC/state seam — a Rust enum with
~35 variants (`Token`, `Thinking`, `Turn`, `Usage`, `ToolCall`, `ToolResult`,
`Routed`, `Memory`, `Deck`, `Artifact`, `Delegation`, `Seat`, `Finished`,
`Failed`, …). Its doc comment is explicit that these are transport, not log
content.

`fan_out()` (`bridge.rs:11128-11242`) is the single server-side reducer: it
resolves the target session, calls `describe(&event)` (`:10860`) to turn the
enum into a JSON object, appends to the session's `VecDeque`, and broadcasts.
`MAX_EVENTS = 4096` caps the per-session log; `BROADCAST_DEPTH = 256` caps
the channel; a lagging client lands on `/api/state` for the replay and the
`dropped` counter names what it missed.

Transport: `GET /api/state` (the replay document, `bridge.rs:843-1006`),
`GET /events` (SSE, `server.rs:3030-3078`), `POST /api/run|plan|control|
approve`. Every write rides a bridge method; **the face never writes files**.

### The design system

`app.css:1-16` states the philosophy: a three-tier colour chain — reference
palette → alias → component tokens. A theme re-points the alias tier only; no
component rule moves. Four themes (paper, console, graphite, contrast).

`src/gui/themes.rs` reads the shipped `app.css` with `include_str!`, parses
every theme block, resolves `var()` chains, and measures every text token
against all three surfaces at WCAG AA 4.5:1. The test
`every_themes_text_clears_the_contrast_floor_on_every_surface` (`:269-310`)
fails the build if any token drops below the floor. Design tokens are an API
contract with a test, not a styling detail.

The icon set has the same treatment: the harness reads `app.js` as text,
extracts every `icon('…')` call and every section-map key, and fails on any
key missing from `ICONS` (`harness.mjs:9608-9633`).

### Verification

- `tests/face/harness.mjs` — 10,462 lines, zero-dependency Node ESM runner.
  Boots the real `app.js` in a `vm` context against a hand-written DOM stub,
  feeds it documents in the bridge's own shapes, drives it like a pointer or
  keystroke, asserts on what it built. 111 cases. It pins **behavior, not
  layout**.
- `tests/gui_sse.rs` — a real scripted mock server, a real agent thread, and
  the server's own `/events` stream read to the run's end.
- A Playwright/CDP pass drives real Chromium at the shipped widths for
  rendering, interaction, overflow, console errors, and the `data-act` audit
  (`docs/features/gui.md:840-846`).
- Mockups-first: every new surface ships a static HTML mockup in the app's own
  tokens, measured across themes and widths, and read before code
  (`docs/plans/mockups/…`).

### Adding a surface: the 11 registration points

1. `ROUTES` — the address
2. `GROUPS` — the rail entry and group
3. `index.html` — a `<main class="stage stage-board" id="panel-<name>" hidden>`
4. the `el` map — `panelX: document.getElementById(...)`
5. `PANEL_EL` — `name: el.panelX`
6. `BUILT` — so the palette does not badge it "later"
7. `showRoute`'s `arrive` map — `paintX()` then `readX(true).then(paintX)`
8. `ICONS` — a drawing for the concept
9. the bridge: a read method + a route + a feature doc
10. harness cases
11. the four-leg gate + the docs

### The other surfaces

- **TUI** (`src/tui/`, 13,346 lines, ratatui) — a second, independent renderer
  over the same `UiEvent` stream and the same substrate reads. No shared
  rendering code. `App::apply_ui_event` (`src/tui/app.rs:1317`) is a parallel
  switch over the same variants.
- **Tauri shell** (`src-tauri/`) — a thin window over the same server; it does
  self-containment work (bundled services, lane env, first-run install), not
  UI.
- **Bot face**, **plugins** (sandboxed `srcdoc` iframes with a declared
  read-only query set), **design frames** (opaque HTML pages with `:root`
  variable injection).

---

## Part 2 — json-render

### What it is

A **Generative UI framework**. A typed catalog of components and actions is
the guardrail; the AI emits a JSON spec constrained to that catalog; a
renderer renders it. The same spec renders on React, Vue, Svelte, Solid,
React Native, Ink (terminal), Remotion (video), React PDF, React Email,
Chat SDK (Slack/Teams/Discord), image (Satori), 3D (R3F), and full Next.js /
TanStack Start apps.

~30 packages under `packages/`, a pnpm + turbo monorepo, Node ≥24.

### The core loop

```ts
const catalog = defineCatalog(schema, { components, actions });
const { registry } = defineRegistry(catalog, { components });
<Renderer spec={spec} registry={registry} />;
```

- **Catalog** (`schema.ts:1597`, `schema.createCatalog`) — component names,
  zod prop schemas, descriptions; action names and descriptions. It is both
  the runtime guardrail and the AI's prompt (`catalog.prompt()`), and it
  exports JSON Schema for structured outputs.
- **Spec** — a flat `{ root, elements }` map; children referenced by id.
- **Registry** — the name → implementation map, per framework.

### The mechanisms worth naming

- **SpecStream** (`types.ts:884`) — a JSONL patch compiler. The model emits
  patch lines; the compiler applies them progressively so the UI fills in as
  the response streams.
- **Dynamic props** — `$state`, `$cond`, `$template`, `$computed`,
  `$bindState`; any prop value can be data-driven.
- **Directives** (`directives.ts`) — user-defined `$`-prefixed value
  transforms (`$format`, `$math`, `$concat`, `$count`, `$truncate`,
  `$pluralize`, `$join`, `$t`), validated and resolved at runtime.
- **Actions** — a built-in `setState` plus custom handlers; `watch` fires on
  state changes; `visible` is a declarative condition list.
- **State store** (`state-store.ts`) — framework-agnostic, structural
  sharing, with adapters for Redux, Zustand, Jotai, and XState.
- **Validation** — `validateSpec` + `autoFixSpec`; malformed output is
  repaired or refused, never silently rendered.
- **Devtools** (`packages/devtools`) — a drop-in inspector: spec tree, state
  editor, action log, stream log, catalog browser, DOM picker. Framework
  adapters for React/Vue/Svelte/Solid.
- **Codegen** (`packages/codegen`) — traverse a spec tree and serialize it
  back to code.
- **Edit modes** — patch/merge refinement of an existing spec.
- **Ink** (`packages/ink`) — a terminal renderer with ~28 built-in
  components (Box, Text, Heading, Table, Sparkline, BarChart, Card,
  StatusLine, Metric, Callout, Tabs, Markdown, TextInput, Select, …).

### The philosophy

The UI is data; the AI is the author; the catalog is the contract. The
framework's whole reason to exist is that a model can be trusted to emit a
valid interface when the vocabulary is closed and the schema is enforced.

---

## Part 3 — The comparison

### The opposition

| | Fleet | json-render |
|---|---|---|
| The UI is | a compiled artifact | data |
| The author is | the owner | the AI |
| The contract is | the event log | the catalog |
| Renderers | one per surface family | one spec, many renderers |
| State | server-owned log, read twice | client store, framework-adapted |
| Guardrails | structure guards + harness + mockups | schema validation + autofix |

### What json-render has that fleet does not

1. A typed component catalog with validated props.
2. Spec validation and autofix — fail closed on malformed output.
3. Progressive/streaming render of a spec.
4. A dynamic prop/expression language.
5. An in-app devtools inspector.
6. Codegen from UI trees.
7. One spec rendered by many renderers.

### What fleet has that json-render does not

1. A compile-time design-system guard (AA contrast in every theme).
2. Mockups measured and read across themes and widths before code.
3. An event log as the single source of truth — no client/server divergence.
4. Self-containment: one binary, no toolchain, no Node.
5. A source-pin test for the icon set.
6. A zero-dependency committed UI harness.

### The one big idea fleet has already declined

One spec, two renderers. json-render's core premise is that a single catalog
and a single spec can drive a web UI, a terminal UI, a PDF, and an email.
Fleet has two independent renderers over the same event stream, and its own
doc is explicit about why: the TUI is the instant, keyboard-first, ssh-able
face; the GUI is where the surfaces a terminal cannot hold honestly get room
(`docs/plans/2026-10-16-gui-revamp.md:216-225`). **The split is deliberate,
not a gap.**

---

## Part 4 — Areas that could improve fleet

Classified per the house rule: confirmed gap, enhancement, assumed gap
(likely intentional), not applicable. No plan is proposed; each item names
the assumption to confirm.

### 1. Confirmed gap — the client reducer can silently drop a new event type

**What I see.** `apply()` ends in `default: break;` (`app.js:3186-3187`).
Nothing enumerates `UiEvent`'s variants against the client's `case` arms:
the face harness does not read `src/agent.rs`, and no test asserts that every
variant `describe()` can emit has a matching `case` in `app.js`.

**Why it matters.** Adding a `UiEvent` variant in Rust compiles cleanly —
the Rust `match` in `fan_out` is exhaustive and compiler-checked. But the
JavaScript reducer is a hand-maintained switch with a silent default. A new
event could be emitted, broadcast, stored, and **invisible on the face**, with
no test failing. The same shape as the icon drift the 2026-11-04 audit
caught, one layer up.

**Fix shape.** A source-pin test in the same family as the icon test: read
`src/agent.rs`, extract the `UiEvent` variants, read `app.js`, extract the
`case '<type>'` arms, and fail on any variant with no arm. Cheap, and it
turns a silent default into a loud one.

**Risk.** Low. It is a test, not a runtime change. The only cost is keeping
the two lists in step, which is the point.

### 2. Enhancement — no in-app inspector for the face

**What I see.** json-render ships a devtools panel: spec tree, state editor,
action log, stream log, catalog browser, DOM picker. Fleet has the raw
material — the state document (`bridge.rs:843-1006`), the ordered event log,
the bridge read cache, the harness fixtures — but no surface that shows it
live. The harness proves behavior; the Playwright pass proves rendering;
neither lets the owner look at *what the face is actually holding*.

**Why it matters.** When the face looks wrong, the current path is to read
the code or add a harness case. An owner-only `#/devtools` surface (the
loopback boundary already makes the machine the authentication boundary)
showing the raw state document, the event log with type filters, and the
read cache would make debugging far cheaper — and would surface finding 1's
silent drops directly.

**Assumption to confirm.** That the owner wants this. It may be developer
tooling the owner never asked for; the harness plus the browser pass may be
enough for the person who writes the code.

**Risk.** A new surface is the 11-point checklist; keep it read-only and
small.

### 3. Enhancement — no machine-readable component catalog

**What I see.** json-render's catalog is a typed registry: component name,
props schema, description, and the harness can validate a spec against it.
Fleet's builders are positional-argument functions; the only contract pins
are the `data-act` audit and the behavior cases in the harness.

**Why it matters.** The design system is already treated as an API contract
(themes.rs, the icon source-pin). The component layer is not. A
`COMPONENTS` table in `app.js` — builder name → required `dataset` keys, ARIA
attributes, role — read as text by the harness would catch a builder whose
contract drifted, and would give the docs a living component inventory
instead of a hand-maintained list.

**Assumption to confirm.** Worth it only while the surface count keeps
growing; the design-system doc already lists the anti-patterns, and the
`data-act` audit already covers the interactive contract.

**Risk.** Low to medium. The table can rot if it is not tested; the test is
the point.

### 4. Enhancement — the 11-point surface registration wants a factory

**What I see.** Adding a surface touches 11 places, all mechanical: `ROUTES`,
`GROUPS`, the `<main>` in `index.html`, the `el` map, `PANEL_EL`, `BUILT`,
the `arrive` handler, `ICONS`, the bridge read, the harness, the gate. The
surfaces are highly uniform — head, strip of counts, column of cards, rail —
which is exactly the condition a factory or a codegen step exploits.
json-render's `defineCatalog` + `defineRegistry` collapse the same shape
into one declaration.

**Why it matters.** The memory `a-top-level-surface-needs-an-address` records
the friction: a surface was shipped as a card inside Settings because the
registration was not obvious. A `defineSurface()` declaration that emits the
route, the rail entry, the panel element, and the arrive handler from one
table would remove a class of forgotten-registration defects.

**Assumption to confirm.** That the uniformity is real and stable. It looks
real (the agent's survey found the same grammar on every surface), but a
factory that hides the explicitness could make the face harder to audit —
the opposite of the project's value.

**Risk.** Medium. Keep it a thin declaration over the existing registries,
not a framework; the harness should still assert each registration point.

### 5. Enhancement, measure first — the transcript has no windowing

**What I see.** `face.rows` accumulates every row; the server caps the log at
`MAX_EVENTS = 4096` (`bridge.rs:61`); `replay()` rebuilds the whole log on
load. There is no virtualization or windowing — the only "window" in the
client is the DAG minimap (`app.js:14902, 15297`). A long session's first
paint is O(n) DOM nodes.

**Why it might matter.** json-render's SpecStream renders progressively, so a
large spec never blocks first paint. Fleet's equivalent would be a windowed
transcript: render the visible range, keep the log in `face`, and recycle
rows. The 4096 cap is a correctness guard against unbounded growth, not a
rendering strategy.

**Assumption to confirm.** That this is a real problem at the sizes the
owner actually runs. A 4096-row DOM is heavy but workable; the measurement
should come first, not the rewrite.

**Risk.** Medium to high. Windowing complicates the in-place `update` rows,
the minimap, and the transcript search.

### 6. Assumed gap, likely intentional — one spec for the TUI and the face

json-render's headline idea is one catalog and one spec rendered by many
renderers. Fleet's TUI and face share the event stream but not a rendering
description. Fleet's own doc says the split is deliberate: the TUI is the
instant keyboard face, the GUI is where terminal-unfriendly surfaces get
room. **Not a recommendation** — named here so the comparison is honest
about what fleet has already decided.

### 7. Not applicable

- **AI-authored UI.** json-render exists so a model can emit a valid
  interface. Fleet's agent does not author the face; the owner does, and hard
  rule 11 holds the owner's surfaces to a visual bar no model is currently
  asked to meet.
- **React / framework adoption.** Hard rules 6 and 12: no Node, no foreign
  runtime, one static binary. A component framework would be a design defect
  under the founding contract, not an improvement.
- **A client-side expression language.** json-render's `$state`/`$cond`/
  `$computed` exist because the spec is data and the renderer is generic.
  Fleet's values come from the server, which already computes them; a client
  expression layer would add risk without removing server work.

---

## What fleet already does better

To keep the comparison honest: fleet's compile-time AA contrast guard, its
mockups-measured-and-read gate, its event-log-as-single-source-of-truth, and
its self-containment are all stronger than json-render's equivalents.
json-render is a 30-package pnpm monorepo with no design-system guard and no
mockup gate; its advantages are the catalog contract, the inspector, and the
progressive render — not the visual discipline.

---

## Part 5 — Fleet as a consumer: building UI components for other projects

### The reframing

The question is not how json-render improves fleet's own face. It is how
fleet — the agent — could use json-render as a tool to build UI components
for **other projects**: the owner's other repos, client work, deliverables.

### What fleet already builds

- **The design canvas** (`#/design`, `docs/features/design.md`): a frame is
  one complete, self-contained HTML page — opaque, network-dead, tweakable
  through `:root` variables, versioned, reviewable, bound to a project,
  digested into the corpus, exportable, and servable as a static site at
  `/site/<slug>/`. The model-facing tools are `frame` (generate/refine) and
  `review_frame`, under the `designer` role.
- **The deck** (`src/deck.rs`): a three-stage pipeline (research → structure
  → render) with a rigid serde schema, consultant constraints that refuse
  (Pyramid Principle, MECE, bolding), a layout grader, and an HTML render.
  The deck is a *value* the agent builds, checked before it renders.
- **`send_file`** (`src/tools.rs:1122`): hand a file to the owner.
- **The skills system** (`src/skills.rs`): SKILL.md loading, trigger-shaped
  prompt lines, `use_skill`, the self-authoring drafter, a 60-skill cap.
- **The MCP registry** (`src/mcp_registry.rs`): mcp.json, scanning foreign
  harness configs, import, health.

Fleet already has a deliverable path for visual work. The frame is opaque
HTML; the deck is a checked structure. What neither has is a **typed
component vocabulary** or a **multi-target render**.

### What json-render offers a builder agent

- **A catalog**: component names, zod prop schemas, descriptions, actions.
  It is the guardrail and the prompt — the agent's vocabulary is closed and
  validated.
- **A spec**: a flat JSON tree the agent authors.
- **Renderers**: React, Next.js (a full app), Vue, Svelte, Solid, React
  Native, Ink (terminal), Remotion (video), React PDF, React Email, Chat SDK
  (Slack/Teams), image (SVG/PNG via Satori), 3D.
- **Validation + autofix**: malformed specs are refused or repaired, never
  silently rendered.
- **Agent interface**: ~20 skills under `skills/` (core, next, ink,
  react-email, shadcn, codegen, devtools-*, chat) and an MCP server
  (`@json-render/mcp`) that serves a spec as an interactive MCP App.
- **Codegen**: a spec tree serialized back to code.

### Where it fits

The fit is real because **fleet's agent is the author json-render is built
for**. The catalog constrains what the agent can emit; the validation catches
its mistakes; the renderers turn one description into the target platform.

The output formats split into two classes:

- **Static artifacts** (the artifact is the deliverable): SVG/PNG
  (`renderToSvg`/`renderToPng`), HTML email (`renderToHtml`), plain text,
  PDF (render to buffer/stream/file), chat cards.
- **Live apps** (the project is the deliverable): a Next.js app
  (`createNextApp`), a React/Vue/Svelte/Solid renderer, React Native, Ink.

### Integration shapes, lowest friction first

1. **Load json-render's skills into fleet.** They are SKILL.md files; the
   skill loader already reads that format. The agent learns the
   catalog/spec/render workflow with no code change. The cheapest first step.
2. **A `render` deliverable tool.** Alongside `frame` and `review_frame`:
   `{spec, catalog?, target}` → the render runs in a scratch dir or the
   target project, and the file is handed off. The agent's deliverable path
   for component-driven UI.
3. **A `ui` deliverable type.** Spec + catalog + render command, checked by
   `validateSpec` (the same consultant pattern as the deck), rendered to the
   target. The design canvas could gain a spec mode beside the opaque frame.
4. **The image renderer for the canvas and gen1.** Spec → SVG/PNG is a
   natural export for the design canvas's export card and the gen1 media
   lane.
5. **The chat renderer for huddles and bots.** Spec → Slack/Teams card fits
   the owner's huddle and bot ecosystem.
6. **The ink renderer for terminal deliverables.** Spec → a terminal UI,
   fleet's own idiom.
7. **MCP Apps.** `@json-render/mcp` serves a spec as an interactive MCP App
   for Claude, Cursor, and VS Code — a fleet-built UI embedded in the owner's
   other tools.

### The honest tension

Fleet's deliverable philosophy is **network-dead, zero-toolchain,
self-contained**: frames are opaque HTML, decks render to HTML, sites serve
static files. json-render is a Node ≥24 + pnpm + React framework; most of its
outputs are runtime-bound (a Next.js app, a React component), and even the
static ones are *produced* by a Node process.

So json-render is not a replacement for the frame model. It is a **different
deliverable class**: a live app, a document, a card, an image, a terminal UI —
with a structured intermediate (the spec) that the frame model lacks. For the
owner's other projects that is fine: the toolchain belongs to the target
project, not to fleet's binary, so hard rule 12 is not violated. But it is a
philosophical shift from "the artifact is the file" to "the artifact is the
project".

The static targets (SVG/PNG, HTML email, PDF) are the bridge: they keep
fleet's network-dead, hand-off-a-file philosophy while gaining the catalog's
constraint and the multi-target render.

### Risks

- **Toolchain**: Node ≥24 + pnpm + React for the live targets. The owner's
  other projects carry it.
- **Version churn**: a fast-moving Labs project (v0.22.0 at the time of
  writing). Pin exact versions.
- **Catalog quality**: the pre-built set is shadcn's 36 components; a custom
  project needs a custom catalog, which is the real work.
- **The spec is only as good as the catalog**: a thin catalog produces thin
  UIs.

### The shape of a first move

Load the skills, add a `render` tool, and start with the static targets
(image, email HTML, PDF) on a real owner project. That keeps fleet's
philosophy intact while proving the catalog-and-render loop. The live-app
targets follow only when the owner has a project that wants one.

---

## Part 6 — The pieces worth absorbing (not the framework)

The question is not "adopt json-render" but "which of its pieces belong in
fleet". The sharpest framing: **fleet already proved the core idea in
`src/deck.rs`.** The deck is a rigid schema + consultant checks + a render —
a catalog, a validator, and a renderer for presentations. json-render's main
contribution is generalizing that pattern to arbitrary components and
targets. So the work is not importing a framework; it is generalizing a
pattern fleet already has.

### Worth absorbing

1. **A typed component catalog.** A declared set of components with typed
   props and descriptions, validated in Rust. Generalizes the deck's
   `Component`/`Layout` enums. Plugs into the `frame` tool and the design
   canvas. The descriptions feed the prompt and the `designer` role, so the
   agent knows the vocabulary.

2. **A spec as the deliverable's structured intermediate.** A JSON tree
   (`{root, elements}` or fleet's own shape) instead of opaque HTML. Makes
   the artifact diffable, versionable (the canvas already versions frames),
   themeable (the canvas already has `tweak_spec` and `brand`), and
   re-renderable. The frame model's opaque HTML stays as one render target,
   not the source.

3. **A refusing validator.** The deck's `check()` pattern generalized: walk
   the spec, refuse a bad component, name the fault. Pure Rust, no new
   toolchain. This is the reliability piece.

4. **Multi-target render from one spec.** HTML frame (existing), static site
   (existing), SVG/PNG (fleet has hand-rolled SVG), terminal (ratatui, the
   TUI's own widgets), code (a component scaffold for the target project).
   This is the "build UI components for other projects" capability.

5. **The catalog as the prompt.** The component descriptions become part of
   the system prompt / the designer role. Cheap, high leverage.

6. **A spec inspector in the review card.** The canvas's review card already
   puts a deterministic scan above the model's scores; a component-tree view
   (the components, their props, the faults) makes the review concrete.

7. **A fleet-authored skill.** A SKILL.md teaching the catalog, the spec
   shape, the render targets, and the check rules. The skill format is
   fleet's own; the content is the workflow.

### Leave behind

- The Node/pnpm/React runtime (hard rule 12).
- The client state store and the expression language (`$state`, `$cond`,
  `$computed`) — fleet's server computes; the deliverable is a file.
- The streaming SpecStream — fleet's deliverables are artifacts, not live
  streams.
- The MCP server — fleet has its own MCP registry; json-render's MCP serves
  its own framework's apps.

### The sequence

Catalog + validator first (pure Rust, generalizes the deck). Then the spec
as the frame's intermediate. Then the render targets. Then the inspector and
the skill. Each step is a plan, not a rewrite.
