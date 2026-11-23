---
status: active
created: 2026-11-23
plan: the-surface-pass
---

# The surface pass — connections into knowledge, and every surface measured

The owner's word (2026-11-23):

> "now let's go one surface at a time, because I think this is the weakest
> link. first let's move the connections side pane into knowledge, it's just
> conflicting with the look and feel and the contextual drawer. second inspect
> every single page, analyze the functionality of each, inspect dead objects,
> those you would expect to click and either get a contextual card or drill
> down into them, those seem to be all over the place. also think about how to
> best present that information because quite honestly it's underwhelming at
> best, if we are presenting a bunch of cards without interaction, contextual
> meaning, and details then we are not doing a great job because that
> information is essentially useless. an example a skills card with a number
> tells me nothing about, it's just noise"

## The bar, applied object by object

Every rendered object must do one of three jobs:

- **act** — a press whose effect is real and named (open, filter, answer,
  run, copy, retry);
- **drill down** — a press that opens the object's own card or body with its
  details (what it is, its state, its relations, what can be done);
- **explain** — a sentence that says what the thing is, why it matters, and
  what it connects to.

An object that only shows a number or a label is **noise**: it is either
given one of those jobs or removed. The audit measures every object against
this bar; "it renders" is not a pass.

## Rows

### R1 — connections into knowledge (the move)

- The persistent `aside.tissue` column retires: the shell becomes rail |
  stage (+ the drawer), and the graph becomes a **knowledge-region surface**
  at `#/connections`, leading the region.
- The surface renders the same read (`/api/tissue`) at page scale: the
  focus's neighbourhood, the typed legend, the focus card, the honest empty
  state.
- The graph's objects gain their jobs: a **node press focuses in place**
  (drill down — the graph re-centres and the card names it, no navigation);
  the card's **"open <kind>" press navigates** (act); a **legend chip
  isolates its kind** (the count becomes a filter, pressed again to clear);
  the "N more" node still expands.
- Retired with the column: the head chevron, the reveal strip (U12), the
  console `connections` switch, `body[data-tissue]`, and the drawer's
  `--pane-right` reservation. The drawer may take the stage's edge — which is
  KI-555's conflict (the open drawer over the connections column) removed by
  construction.
- Pins: the map carries `connections` under knowledge; the surface paints the
  graph, legend buttons and focus card; a node press refocuses without
  navigating; a chip press isolates its kind; the column's controls are gone.

### R2 — the audit: every surface measured, one at a time

- The ledger: `docs/architecture/2026-11-23-surface-audit.md`, appended per
  surface.
- Per surface: every object it renders, and for each one **act / drill /
  explain / dead / noise**; the dead objects named (pressable-looking with no
  press, or a press with no effect); the presentation verdict; the proposal.
- Evidence per surface: a probe face (a separate `fleet gui` instance,
  read-only) driven page by page; never the owner's live session captured
  into a document.
- Rows R3+ are added as each surface is measured — the house rule, a row on
  intake, never an appended thread. Each row names its fixes, its pins, and
  (where the visual grammar changes) its mockup, which gates the code.
- The drawer's **connections tab** is a different object from the column being
  moved (it carries the conversation's goal edge and delegated work); it is
  audited with the conversation surface, not here.

### R3 — conversations: human work only

The owner's directive (2026-11-23): *"conversations should only see human
initiated or human involved conversations. there are a bunch of system
initiated sessions that are just noise."*

- The Conversations surface lists only what a human started or took part in.
  System-initiated sessions — scheduled runs, cron, delegated workers,
  background auto-tasks — leave the list.
- They are not deleted or hidden from the product: their evidence belongs
  where the system's work is audited (the run's receipts, the board, the
  desk), and a delegated child rides under the parent goal (the findability
  ruling, 2026-11-15: per-class window, spine user-initiated-only, children
  under the parent). The audit confirms the destination read before the
  filter ships — no row is dropped into nowhere.
- **Measure before designing:** count the population by initiator and read
  how the session record names it (the sessions document's counts already
  separate `background`; the spine carries the user-initiated
  classification the findability pass built). The row ships with those
  numbers, not an assumption about which field means what. — **Measured
  2026-11-23** (`docs/architecture/2026-11-23-surface-audit.md`): the window
  holds 99 rows, 40 of them `scheduled`/`background`; the list draws 12
  ("active 12 · archived 47 · all 59"), the archive filter 47, the graph's
  16 thread nodes — **zero scheduled in all of them**. The rule already
  holds on these surfaces; the audit traces where the unfiltered population
  still surfaces (Home's changed block, the inbox rows, the PWA, any count
  of `background`/`verification`/`empty`).
- The count strip follows the list: a number over a filtered population must
  count the same population (the owner's own bar — a number that does not
  match what it sits over is noise).
- Pins: a fixture with owner, delegated and scheduled sessions; the list
  carries only the human ones; a system run stays reachable its own way; the
  strip agrees with the list.
- This row is measured first in R2's pass (the owner named it), and its
  measurement is the population evidence the filter is built from.

### R4 — the shared read-only strip retires

- The audit found ONE builder putting four label+number cards — runs,
  machines, skills, tools — on six surfaces (inbox, learning, calibration,
  second brain, armada, intelligence), plus a duplicate of the same counts on
  Intelligence's main column. The owner's example, system-wide.
- Retired: `spineCards` and its five callers, and the memory capability
  cards. The numbers stay where they are actionable (armada's map, the
  board's plans, Intelligence's rail).
- Measured after: inbox 4 → **0** cards, memory 4 → **0**; harness green.
- Pins: no surface renders a label+number card with no press; the surfacing
  cases that counted those cards are replaced by the surface's own pins.

### R5 — stat strips become doors

- Calibration's six numbers (512 rows, 0 judged, 0 fit cells, …), the
  router's six (15 calls, 15 OK, 0 errors, 100% success, 0 tokens, judge),
  memory's six (facts/live/protected/retired/hits/recalls): each becomes the
  press that reveals its population (the chip pattern from connections).
- Pins: each number's element is a press with a named effect; no numeric
  strip with no press remains on those surfaces.
- **Shipped:** `surfaceStrip` items may carry a press and render as real
  buttons (dotted rule, visible focus, aria-label). Calibration's six
  numbers are doors — the row counts open the routing section, the
  fit/gate numbers open the gate. Memory's facts/retired/hits open the
  corpus/retired/recall (live and protected are facets with no view, so
  they stay plain rather than lying). The router's "calls today" opens the
  day's route rows (`#/calibration/routing`). Live-verified: 6/3/1 doors,
  each landing on the section it names.

### R6 — empty and loading states gain their head and their one press

- Inbox: a bare "waiting" chip and a floating title → a page head, the
  honest empty state, the one door.
- Board: the "no approved plan" card names the rule and offers no press →
  "open the plans folder" / "ask for a plan".
- Learning and Second brain: "Still loading" at 5 s with no door → elapsed
  time, retry, and the surface's invitation.
- Pins: one per surface (empty state has a press; a loading state carries
  elapsed and a retry).
- **Shipped:** the board's empty state opens the plans folder in the pane's
  tree; learning and the health lanes show `waiting Ns` + a retry that
  re-reads; the empty inbox renders through the one head and its door opens
  the conversations. The bridge fix behind it: `read_work_board` reported an
  empty plans folder as a **refusal**, painting the error grammar and hiding
  the empty state — empty is a result, refused is a failed read. New pin:
  `R6 empty and loading states name their one press` (the read is held open
  to see the loading state; the plans tree paints the folder).

### R7 — the router's chains flatten and its crumb tells the truth

- Nine nested chain rows inside a card → rows at one depth; the six numbers
  become doors (R5); the `SETTINGS / advanced` crumb on an INTELLIGENCE page
  is corrected (G6).
- Pins: no nested card rows on the surface; the crumb's group equals the
  rail region (intelligence for the router? it is an advanced page — the
  ledger records where it truly lives).
- **Shipped:** the chain rows are rows (hairline-separated, no card inside
  the card); the crumb map is rebuilt from the rail's own map, so a folded
  surface reads `advanced / Routing` (it used to say `SETTINGS / advanced`,
  naming no page) and a railed lane keeps its region (design is `design /
  Design`); the eyebrow agrees with the crumb across memory, learning,
  calibration, router, delegation, workspace history, second brain. New pin:
  `R7 every surface head wears its place` (six surfaces, eyebrow == crumb
  group).
- **Found while measuring:** calibration's routing section already renders
  the router's chain card (8 rows live) — so the standalone router page
  duplicates a section of Reviews, exactly what D3 of the redesign says
  ("Routing is Reviews' first section"). Folding the router page into that
  section is the next product decision; it lands with a mockup and the
  #/router address redirecting to `#/calibration/routing`.

### R8 — the design-system conformance row

- The contract (D1–D7 of the redesign) is one Indigo Studio token chain, one
  accent, the elevation law, one head grammar, and the shared builders. The
  audit measured three head grammars, bespoke containers on seven surfaces,
  a 4–10 size spread per surface, and one raw colour/shadow pair.
- Done in this row: **one head** (`surfaceHead` folded onto `answerHead`,
  the health page's own header folded in), the one raw shadow tokenized.
  Zero raw colours outside the token blocks, zero inline colours.
- Carried into every surface row: one `page-head`, one `.section-rail` where
  sections exist, `.board-card` units at one depth, the type ladder, no new
  raw values without a token first.
- Pins: the harness's head assertions read `.page-head` (the old class is
  gone); the per-surface rows pin their own composition.

### R9 — armada (done)

- The map's rows are real buttons (they were click-divs); the relay opens the
  pairing; the corpus copies its remote; the empty states already explained
  and stay. Pinned in the P6 armada case.

### R9 — the affordance sweep (done across all surfaces)

- The click-div census: **19 sites**; 18 already carried role + tabindex,
  **zero showed a cursor**, and **seven had no keyboard at all** (no local
  keydown). Fixed centrally: one CSS rule gives every `role=button/tab/
  option/menuitem/switch` its cursor; one document-level keydown makes a
  focused div-button answer Enter/Space exactly like a native button (and
  keeps out of a local handler's way via `defaultPrevented`). The one site
  with no semantics at all — the desk's empty card, which starts the brief —
  now carries role, tabindex and its label.
- Learning's duplicated title is gone: its column names the list
  (`projects`), the page head names the surface once.
- Pinned in the R10 case: the cursor rule, the keydown handler, and the
  learning column/head split.

### R9 — G1 across the settings family (done)

- The audit's nested-card counts were the form groups: `.cfg-form` and
  `.cfg-details` are styled as cards (border, radius, page background), so
  inside a board card they were boxes in boxes — the vault measured 2
  nested. Scoped fix: a form group **inside** a card is a section (hairline
  + spacing, no box); an unparented form keeps its frame. Measured after:
  vault 2 → **0** nested, settings/plugins/bots 0. Pinned in the R10 case.

### R10 — the card grid holds: aligned, even, rails collapsed, PWA sizes

- **Measured live** (probe face at 1440×900, each surface with the rail
  **open and collapsed**): 15 surfaces — no horizontal overflow, no row
  holes, every `data-span` card on a column start (one learning-shell
  artifact: its card sits inside the work-column shell, not the board grid;
  visually aligned, noted not chased).
- **Defect found and fixed:** `boardCard(6)` — used twice — had **no column
  rule at all**, so pairs meant to sit side by side stacked as two
  full-width cards. The half rule now exists and collapses at ≤1180 with
  the other split spans. New pin: `R10 the card grid spans are all declared
  and collapse` (every span the source asks for has a rule; the collapse
  lists; the bot PWA's split breakpoint).
- **PWA:** the shell/pair page measured clean at all four viewports —
  390×844, 844×390, 820×1180, 1180×820: no overflow, no element wider than
  the viewport, the form fluid (342 → 420 centred). **Gap, named:** the
  paired views (inbox, conversation, iPad split) need a device token; the
  probe face's bot pair API rejected the CLI's code (403 — that code is the
  relay's, not the bot face's), so those views were not re-measured live in
  this pass. `bot.css` is unchanged by this pass, so the earlier 390/820
  viewport evidence stands. Next action: pair through the relay
  (`fleet pair` → open the printed URL) and capture the four viewports, or
  add structural per-viewport pins to the bot harness.

### R9 — the remaining per-surface passes

- The audit's fix order: armada, calibration's report cards, delegation,
  huddles, workspace history, vault, plugins, bots, learning, desk, board,
  settings; each one row with its inventory, its fixes and its pins.
- The drawer's connections tab renames to **work** (it carries the goal edge
  and delegated work, not the graph); this amends G11's four-tab grammar and
  is recorded in the gap ledger.

## Constraints

- No new tokens; the theme AA guard stands.
- Every address resolves (U47); the palette and the expert census keep
  working, and every retired control's function has a named successor.
- No new network; the plugin sandbox is untouched.
- A visual redesign lands a mockup before code (the owner's gate).
- A finding that is not this plan's to fix is a named gap (a KI row), never a
  silent omission.

## Out of scope

- New backends or reads the face does not already hold; a card that needs one
  is a gap, not a card.
- Release cuts (standing rule: the owner's explicit word).

## Risks, named

- The move touches the shell's width math, the console switches and the focus
  law; every retired pin is replaced by a named one, so a lost behaviour is a
  red check.
- The surface at page scale is new composition; the graph's existing language
  (hue, typed legend, focus card) is kept so the change is placement and
  interaction, not a re-skin.
