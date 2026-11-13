# Fleet — the UI decisions ledger

**The design-system backbone.** Fleet's surfaces are built by many sessions
and agents across three faces (browser console, terminal, native shell) over
one agent core. Without a shared record, every fresh session invents its own
palette, its own layout grammar, and its own interaction law — and the
product drifts into the disjointed surfaces this program exists to eliminate.
This ledger is what keeps the UX from diverging, the same way
[`ARMADA-DESIGN-DECISIONS.md`](ARMADA-DESIGN-DECISIONS.md) keeps the stack
from diverging.

The **reference implementation** of everything settled here is the working
mockup at [`../design/armada-workspace-mockup.html`](../design/armada-workspace-mockup.html)
— open it in a browser; it is a live, interactive workspace, not a picture.
When this ledger and the mockup disagree, the ledger wins and the mockup is
updated to match. When a component is rebuilt for production
(`src/gui/assets/`, the TUI, the Tauri shell), it must be able to say which
`U` ids it implements.

Rules (the D-ledger's, applied to design):
- `U` ids are **stable and never reused or reordered**. A decision is
  superseded only by a new `U` that names the old one and why.
- A decision is **settled** when the owner approves it. Until then it is
  *proposed* — mockups conform to it, and if the owner changes it, the
  ledger updates and the surfaces re-conform.
- Each entry: the decision, the rationale, and **what it constrains**.
- New components or lenses **append** a `U` and update the reference mockup
  in the same change. Building UI without citing the ledger is how silos
  happen.

---

## Principles

- **U0 — One workspace, one graph; every surface is a lens.** The work-object
  graph (task ↔ thread ↔ deliverable ↔ bot ↔ lesson ↔ instance ↔ fact, L7) is
  the product; Board, Threads, Deliverables, Fleet, Inbox, Memory are *views
  over it*, never separate apps. The connective tissue is made **permanently
  visible**: a graph pane of the focused object's neighborhood ships on every
  surface ≥1080px wide, and clicking any node pivots the whole workspace.
  *Rationale:* the disjointed-surfaces problem is solved by showing the
  connections, not by claiming them. *Constrains:* every face, every future
  lens, the TUI pane set, the Tauri shell.
- **U1 — The visual bar is co-equal (hard rule 11).** A surface that carries
  quantitative or structural meaning renders it visually — gauges, meters,
  colored liveness, a graph — never as a plain-text table dressed as a pane.
  *Constrains:* TUI boards (ratatui widgets, not `println` grids), face
  sections, the armada board.
- **U2 — Paint before read; never fake data.** A surface shows its loading
  state, then the read's truth; a refused read names itself. Live values come
  from the app's own state, never invented filler. *Constrains:* every read
  path in the face (the existing `paintX`/`readX` discipline), the harness.
- **U3 — One focus.** Exactly one object is focused at a time; the lens, the
  tissue graph, and the focus card all follow it through a single pivot
  (`focusOn(kind, id)`). No surface holds a private second focus. *Constrains:*
  navigation, deep links, the palette.

## Art direction — "Harbor Night / Day Watch"

- **U4 — The palette is a signal law, not decoration.** Deep harbor ink-teal
  dominates; accents are full-chroma *signals with fixed meanings*: green =
  verified/live/done · cyan = threads/relay/primary action · magenta = bots ·
  amber = tasks/working/pending · gold = lessons · red = blocked/breach ·
  teal = facts/memory · blue = instances/fleet. The eight hues are the whole
  set — no agent introduces a tenth. Every accent pairs with an icon or label
  so meaning never rides color alone. *Constrains:* every component, both
  themes, the TUI's color pairs.
- **U5 — Two themes, one token set.** All color lives as CSS custom
  properties on the surface root (`--ink --panel --text --muted --line` +
  the eight signals, each with a translucent `-d` wash for fills). Night is
  the default; Day Watch (`[data-theme="day"]`) is the warm-paper inverse.
  **No component hardcodes a color literal** — hardcoding breaks the theme
  contract by construction. *Constrains:* app.css, all inline styles.
- **U6 — Typography is three voices.** Space Grotesk (display: brand,
  headings, object titles), Inter (body, controls), JetBrains Mono (data:
  tags, timestamps, gauges, keyboard, section micro-labels — uppercase,
  letter-spacing .12–.16em, 9.5–11px). Weights ≤700; body 13–14px; nothing
  below 9.5px. *Constrains:* the font loading in the face, TUI double-width
  budget.
- **U7 — Geometry: three radii, one shadow, translucent lines.** r-lg 16
  (the shell), r-md 11 (panels/cards), r-sm 7 (chips/inputs); one deep-soft
  shadow token; borders are 1px translucent teal (`--line` resting,
  `--line2` hover) — never solid gray, never decorative double borders.
  *Constrains:* every box.
- **U8 — Motion is ambient, meaningful, and switchable.** One canvas layer
  (drifting light + grain, ≤.55 opacity, behind everything); a pulse ring on
  live objects; a sweep on running work; flow-dash on hot graph edges;
  .14–.26s transitions elsewhere. A **Motion** switch freezes all of it, and
  `prefers-reduced-motion` is honored automatically. *Constrains:* the
  ambient layer, every animation.

## Layout contract

- **U9 — The shell is one grid, never a stack of apps.** Top chrome (brand ·
  global search with ⌘K · live fleet status · sound/theme) / body
  (`236px rail · 1fr lens · 320px tissue`) / console footer. ≥1080px shows
  all three columns; ≤1080 drops the tissue; ≤720 stacks — rail horizontal,
  lenses single-column, every grid/flex item `min-width:0` so nothing clips.
  *Constrains:* app.css layout, the TUI's pane layout, Tauri window minimums.
- **U10 — The rail is work-organized, not machinery-organized.** Mission
  card (the active project + version + count) → Views nav (lens icon, label,
  live count; active state = 2px cyan inset bar) → Saved views (one click
  restores `{lens, filters, query}`; save captures the current view) → a
  short tissue explainer. *Constrains:* the rail renderer.
- **U11 — The tissue pane is the connective tissue, rendered.** Radial SVG
  graph of the focused object's neighborhood: center node (ring 2, unlabeled —
  the focus card names it), typed colored nodes with wash fills, edges to
  every linked object; nodes in the top half label above, bottom half below;
  live instances pulse; running work's edges flow. Below it: the typed
  legend (all seven kinds) and the **focus card** — kind chip, title, meta
  rows, primary actions, and the connected list (click = pivot). *Constrains:*
  the graph renderer, every lens's focus wiring.
- **U12 — The console is a permanently visible control region — never a
  modal, never hidden.** It carries the live view controls (Density · Signal
  accent · Live-feed speed · Graph spread · Motion · Tissue · Labels) and the
  data actions (Shuffle · Reset · Export). Every control applies **live**, to
  the real UI. This is the mockup's control contract; production surfaces
  carry the analogous controls for what they render. The connections column
  also collapses from **its own head chevron** — three doors on one state
  (`body[data-tissue]`): the console switch, the column's chevron, and the
  slim edge tab that holds the stage's right edge while the column is away.
  Collapsed is a 30px strip, not a vanish: the way back never hides with the
  pane it controls, and the choice persists. *Constrains:* the
  console bar, any future "view settings".
- **U13 — Overlays: drawer, palette, toasts.** Detail editing is a right
  drawer (380px, slide + scrim) — the persistent panes never hide behind it.
  The command palette (⌘K) centers over the chrome and lists commands and
  every object, kind-typed. Toasts land bottom-center, carry an icon, and a
  destructive action's toast carries **Undo**. *Constrains:* the overlay
  layer, keyboard handling (⌘K toggle, Escape closes).

## Components

- **U14 — Task card:** bulk check · title · tag row (bot magenta, instance
  blue, priority mono) · link dots (thread/deliverable/lesson, 22px, click =
  pivot) · running state = cyan sweep overlay. Cards drag between
  Working/Blocked/Done columns; the columns count and stretch full height;
  beneath them the **activity strip** ("Settled · graded against outcome")
  tells the graded story — never filler cards. *Constrains:* the board lens.
- **U15 — Status color law:** working = amber, blocked = red, done = green —
  the *same* three on board chips, card tags, the focus card, the graph, and
  the TUI. A status that changes color per surface is a bug. *Constrains:*
  everything that renders a status.
- **U16 — Instance card (the fleet):** heartbeat pulse when live · capacity
  gauge (blue→cyan) · reliability gauge (green >80%, amber below, faint
  **"un-calibrated"** when zero settles — autonomy is earned, D49) · version
  row · Dispatch + Heartbeat actions. Provisioning joins as un-calibrated and
  earns its heartbeat. *Constrains:* the fleet lens, the TUI armada board.
- **U17 — Inbox row (omnichannel):** channel chip → who + resolved
  **principal chip** (cyan — the identity graph, D31) → text → outbox state
  (delivered green / queued amber). A reply's journey is visible, not
  assumed. *Constrains:* the inbox lens, channel surfaces.
- **U18 — Fact card:** left border carries the basis color (stated cyan /
  inferred amber / validated green — the corpus basis ladder), name in mono,
  basis chip right. *Constrains:* the memory lens, recall surfaces.
- **U19 — Deliverable card:** title, claim, coverage meter (gradient track —
  meters get **no decorative borders**), end-gate line (shield-check green /
  shield-alert red with the named refusal). *Constrains:* the desk lens.
- **U20 — Threads:** avatar-initial messages, tool-call rows in mono with a
  check, composer with @mention chips; a streaming answer carries a blinking
  cursor and paints incrementally. **The composer is pinned to the bottom of
  the chat area** — messages scroll in their own well above it, the composer
  never scrolls away, and the transcript auto-follows only while the reader
  is already at its end. Dispatch switches the lens to the task's thread and
  streams there — the run is *in* the workspace, not a modal. *Constrains:*
  the threads lens, the dispatch flow.
- **U39 — The thread pane: the receipts of the run.** A collapsible side pane
  on Threads lists every object the thread generated, live — the deliverable
  with its stage and gate, recalled facts, the grade row, the born lesson, the
  instance — each row color-dotted by kind, freshly-landed rows pulsing, a
  click pivoting the whole workspace (U3). It collapses to a tools toggle
  ("Thread tissue"); the pinned composer is never covered, and on narrow
  widths the pane stacks below the chat. *Rationale:* a conversation that
  generates objects should show them accumulating where you are talking — the
  connective tissue at the point of work, not only in the graph pane.
  *Constrains:* the threads lens on every face.

## Interaction law

- **U21 — Optimistic then settled.** Every mutation updates the UI at once
  (create, edit, status move, delete), then the graph and counts re-render;
  the run's settle grades the outcome (deliverable lands, task → done/blocked,
  lesson born, reliability rises). Nothing waits on a spinner that lies.
  *Constrains:* every write path.
- **U22 — Destructive is undoable.** Delete removes optimistically and its
  toast carries Undo (~6s). The same law will bind production (the real face
  already journals first — the UI matches the contract, D3).
- **U23 — Bulk is explicit.** Card checks select; a bulk bar appears with
  Dispatch / Delete / Clear and the count. Never implicit selection.
- **U24 — Search and the palette are the two doors.** The chrome search
  filters the active lens live (debounced by typing, cleared with the field);
  ⌘K opens the palette over everything (commands + objects). Both feed the
  same focus law (U3).
- **U25 — Sound is opt-in and sparse.** Muted by default; when enabled, one
  soft ping on dispatch and on settle. A work tool never plays ambient noise.
  *Constrains:* the sound toggle.

## Anti-divergence — the never list

- **U26 — Never:** a silo surface (a view that hides the graph or breaks the
  focus law) · dashboard filler cards · decorative color (accent without
  meaning) · a plain-text table where U1 demands a visual · console controls
  tucked into a modal · autoplay sound · an accent hue outside the eight · a
  hardcoded color literal · a second focus mechanism · invented data standing
  in for a read.
- **U27 — Scoping:** the mockup/reference implementation scopes everything
  under one root id (`#fa-root`), variables on that root, component classes
  prefixed. Production surfaces follow the face's existing conventions
  (`app.css` tokens) while **keeping the U4 token set one-to-one** — the
  tokens are the contract, the prefix is per-surface.
- **U28 — Changing this system:** append a `U` here and update the reference
  mockup in the same commit; a production surface that diverges is brought
  back or the ledger is superseded by name. Agents drafting UI plans cite the
  `U` ids they implement; the plan gate reads them like `D` ids.

---

## Fleet-core — the same system, the owner's own surfaces (2026-11-12)

The armada direction expanded into fleet itself. The reference implementation
is [`../design/fleet-workspace-mockup.html`](../design/fleet-workspace-mockup.html)
— same shell, same tokens, same tissue; the lenses are fleet's own surfaces.

- **U29 — Armada is additive, never a second product.** One workspace, one
  shell, one tissue. Flipping **Armada** on grows the graph — instance nodes,
  the Fleet lens, relay traffic in the inbox, Dispatch actions — and flipping
  it off leaves fleet whole: nothing reflows, nothing fleet needs disappears.
  The console carries the switch; the brand sub-line reads *"the workspace ·
  one graph"* off and *"armada · one mind"* on. A person using fleet with or
  without armada has the same connective tissue. *Constrains:* every
  surface's armada integration, the onboarding path.
- **U30 — The desk renders the loop, not a list.** Each deliverable card
  shows the five-stage pipeline (Brief → Sources → Coverage → Draft → Digest)
  as a stage track with the current stage lit, a coverage meter, and named
  refusals as red chips **on the card**. Advancing is a visible act; settling
  grades (end gate pass/refused), lands a calibration row, and births a
  lesson on pass — the graph grows the edges in the same beat. *Constrains:*
  the desk lens, `desk.rs` surfaces.
- **U31 — Calibration is the epistemology rendered.** Every row shows
  workload, decision, a confidence meter, the graded outcome, and
  agree/disagree — and the owner's word **rides the same append-only row**
  (gold border when it does). Held/missed counts live in the lens header.
  This is "trust is earned by measurements" (D4) as a surface. *Constrains:*
  the calibration lens, the demand map.
- **U32 — Memory shows provenance and the dream.** Basis chips (stated cyan /
  inferred amber / validated green), anti-vectors tiered in red, and a Dream
  action that compresses stale inferences into an episodic summary —
  superseded, archived, never lost. *Constrains:* the memory lens, the
  consolidation surface.
- **U33 — Learning shows what held.** Lesson cards carry their propositions
  with held/missed marks and a practice meter; Practice lands its outcome on
  the card and the activity strip. *Constrains:* the learning lens, the
  lesson surface.

---

## Onboarding + setup — the graph growing (2026-11-12)

Reference implementation:
[`../design/fleet-onboarding-mockup.html`](../design/fleet-onboarding-mockup.html).
Same shell, same tokens; the Setup lens on a fresh home.

- **U34 — Onboarding is the graph growing, not a wizard.** A fresh home opens
  with the tissue holding exactly ONE node — *this install* (the minted
  identity, pulsing). Every setup step **adds a node and an edge**: the lane
  account (fact), the first answer (thread + a calibration grade), the
  workspace folder (the grant). Completion is N connected nodes, and the user
  has learned the pivot gesture by building their own workspace. Setup stays
  in the rail afterward — it is where you return to grow the fleet or re-run a
  check, never a modal that vanishes. *Constrains:* the onboard flow
  (`fleet onboard`, the gui first run), every surface's fresh-home state.
- **U35 — Empty states are onboarding.** An empty lens names the step that
  fills it and offers the action ("Memory fills from the first connection →
  Connect an account") — never a dead *nothing here*. The nav shows honest
  zero counts. *Constrains:* every lens's empty state.
- **U36 — Skipped is named, never hidden.** Optional steps (the folder) can be
  skipped, and the step keeps their consequence in view: *"filesystem tools
  stay ungranted until a folder is set"* — with a one-click path back. The
  graph simply does not grow that node; nothing pretends. *Constrains:*
  onboarding, the folder grant surface (KI-492's truth).
- **U37 — Setup tells the mechanism's truth.** The real-answer proof is
  **graded like every decision after it** — a calibration row lands in the
  workspace during setup (D4 from minute one). An instance joins
  **un-calibrated** and is trusted only after its first settle (D49/D57). The
  credentials line says what the product does: *stored in your home only; the
  vault never renders a value*. *Constrains:* onboarding copy, the trust
  story everywhere.
- **U38 — Armada setup is the same gesture.** Provision → the instance node
  appears **dashed** ("provisioning") → the shared-mind edge draws (inheriting
  the mind, not the identity) → registered · un-calibrated → first settle →
  the pulse lights and the reliability meter is born. Fleetsetup and armada
  setup are one continuous surface, gated by steps, never two wizards.
  *Constrains:* `fleet armada deploy` UX, the provisioning surfaces (L9).

---

## Intelligence — the capability surface (2026-11-12)

- **U40 — "Intelligence" is the capability surface, not a memory list.** The
  lens formerly named Memory becomes **Intelligence** — everything the harness
  knows *and can do* — organized in segments with live counts: **Memories**
  (the basis ladder, anti-vectors in red, the Dream that compresses stale
  inferences), **Skills** (procedural memory — active in gold, proposals in
  amber waiting at the gate with their drafted-from provenance, applied only
  through the gate, never silently), **Tools** (the bounded surface —
  builtin/lane/MCP, each carrying its read/write/destructive class and an
  enable switch), **Workflows** (composed chains rendered as a stage track,
  every run graded). In the tissue, the intelligence family shares the teal
  signal — **hue encodes the family, icon and label the kind**, so no tenth
  hue is needed (U4 holds). Skills link to the deliverables that used them and
  the workflows that chain them; the thread pane lists the skills a deliverable
  drew on. *Rationale:* "memory" undersold the surface — the owner's framing
  is one place for everything the harness knows and can do. *Constrains:* the
  face's memory/skills/tools surfaces, the substrate rendering, the armada
  intelligence, the onboarding empty state.

- **U41 — At minimum one system-one model, wired during setup.** After the
  provider account, setup walks a dedicated step: the judgment lane —
  closed-set classification with a confidence — must have **at minimum one
  model** before anything else is offered, because the proof answer is judged
  by it. The preferred ones are one click away with links to their own pages:
  **Fastino** (glide — the short-text sharpness, the chain head;
  docs.fastino.ai), **Liquid** (**d1** — the long-context backup on a free
  tier; liquid.ai), and **Drex** (drex-v1.5 — the long-state anchor, 131k
  tokens; drex.nace.ai), on the `/v1/systemone` chain. Liquid is d1 — never
  conflated with jev, which is a separate head. Required, not skippable: the
  responsible-citizen setup never leaves judgment unwired, and the step says
  where keys live (the home; the vault never renders a value). *Constrains:*
  `fleet onboard`, the gui first run, the routing config UX.

- **U42 — The metaphor is architecture, never a label.** The owner's ruling
  (2026-11-12): *"you know why I talked about connective tissue? because the
  cool thing about it is that holds everything together and nobody sees it
  from the outside. naming connective tissue or tissue in the app is
  absolutely meaningless to a user."* User-facing copy names a thing for what
  it DOES, never for the design rationale behind it. The graph column is
  **connections** ("what this connects to"); the thread's generated-objects
  pane is **made here** ("what this conversation made"); the rail's explainer
  is one plain sentence about clicking to see what an item touches. The word
  "tissue", "the receipts", and "the work-object graph" as labels are retired
  from every surface and every reference mockup; they survive only in this
  ledger, the architecture docs, and internal identifiers (ids, classes, the
  `/api/tissue` route), which no user reads. *Constrains:* every user-facing
  string in the face, the TUI, the shell, and the mockups; new copy is judged
  by whether a user who never read this ledger understands it.

---

**Provenance.** The initial system (U0–U28) was settled 2026-11-12 from the
armada interactive mockup ([`../design/armada-workspace-mockup.html`](../design/armada-workspace-mockup.html));
U29–U33 from the fleet-core mockup the same day; U34–U38 from the onboarding
mockup ([`../design/fleet-onboarding-mockup.html`](../design/fleet-onboarding-mockup.html)).
All three informed by the three credited inspirations — muxin (instance cards,
fleet-at-a-glance), omni (the omnichannel inbox with resolved identity), orbit
(the board and saved views) — borrowed as patterns, never as code, per the
README's Foundations section.
