---
status: active
created: 2026-11-20
updated: 2026-11-20
slug: face-rethought
---

# The face, rethought: answer-first surfaces, a newsroom for rooms, and a desk worth returning to

The owner's ask, verbatim: *"let's create a plan to implement them all. but one ask, let's make the visual design appealing, I spend a lot of time using tools like fleet and I want it to be a place I want to go back to. Also the huddles/bots interface needs to look like a real bot interface, because right now it's pretty lame."*

This plan carries the surface-by-surface rebuild proposed from the
2026-11-20 deep dive, a design intent for the appeal mandate, and the
rooms/bots rip. It is one body of work with eight phases, not eight
plans, because every phase shares the same primitives and the same
harness.

## Context

- **What was measured.** The 2026-11-20 dive (`docs/architecture/deep-dive-2026-11-20-fleet-state.md`) and a direct read of five surfaces' screenshots (the `docs/images/surface-*.png` gallery, **2026-11-04 vintage** — the rail has folded since U44, so layout critiques there are directional; the jargon, the zero-valued status bar, the one-weight composer, and the console-shaped surfaces are structural and current).
- **What the face is today.** Fifteen destinations in four subsystem groups, one shared grammar (head · identity card · count strip · column · section rail), and a design system that already names the right ambition: *"a console that reads like a desk"* (`docs/architecture/design-system.md`). The grammar repeats internal module names to the owner and repeats counts in every header; the screenshots show `no route yet · SPEND 0 · CONTEXT 0% · idle` on a fresh home, and a composer carrying six equal-weight controls.
- **Why rooms read as a document list** (the owner's complaint, confirmed on the screenshot): room rows carry a message snippet, not who is speaking; the cast is plain initial discs with static role text; the transcript is three blocks of prose with two action cards; nothing shows a turn, a state, or a persona's identity. A room cannot look like a *place* when its parts are indistinguishable from a file list.
- **Why bots reads as lame** beside real bot apps: no presence, no conversation, no paired-device passport, no proof the door works — three initial discs over two truncated log rows and a "0 of 0 live" badge.
- **What the build loop costs, measured** (`scripts/face-dev.sh`): a face edit is a ~10 s build or zero with `live` (edit, save, refresh); the harness is ~17 s. This work is cheap to iterate and expensive to leave untested — the plan uses that loop in every phase.

## Design intent — what "a place worth returning to" means concretely

The design system's philosophy stands: warm paper, ink, one accent, the
display serif for names, mono for anything measured, comfortable density.
This plan adds five rules that make it *feel* like a room you enter:

1. **The arrival is a question, not a dashboard.** Every surface opens
   with its eyebrow, its name, **one sentence answering its question**,
   and **one verb**. Counts live in the sentence or behind a door — never
   as a strip under the head. (The 2026-11-04 screenshots show the strip
   repeated on every surface; it is the single largest source of the
   "console" feel.)
2. **The topbar tells the truth or keeps quiet.** It renders only what is
   true now: *needs you* when something does, the route when a run is
   live, spend when non-zero, context when > 0. A fresh home shows
   nothing but the palette door and the surface's own head. `idle` is
   silence, never a badge.
3. **The cast has faces.** A persona (a bot, a role, a room member) gets a
   stable identity: an initials disc on a **persona tint**, its name in
   the display face, and a **state word** (`speaking`, `working`,
   `waiting`, `idle`). Six persona tints join the alias tier as
   `--dsw-alias-persona-*`, AA-checked against page and surface like the
   text tokens. This amends the design system's "nothing else is coloured
   for decoration" with one sentence: *identity is the cast list, not
   decoration.*
4. **Motion is a turn, not an ambient loop.** One transition per state
   change (160–220 ms, the existing curve); a room's activity renders as
   its state word plus a 2 px hairline progress, never a shimmer, pulse
   or skeleton. `prefers-reduced-motion` still turns it all off.
5. **A room is a stage.** Space is allocated like a newsroom: the cast
   beside the stage, the transcript as turns with speaker spines, the
   artifacts inline where they were made, the shelf as the room's
   outcomes bar. The document-list shape is retired for rooms.

## Rulings this plan makes (each naming what it extends or supersedes)

- **R1 — The answer line replaces the header stack.** Extends U0 (every
  surface is a lens) and U10 (work-organized rail): the lens answers its
  question in words before it shows a card. Supersedes the
  identity-card + count-strip composition for every surface in scope.
- **R2 — Carry-in is the navigation.** Extends the surfaces doc's "every
  surface is an entry point into the conversation": each surface's verb
  drops a **typed reference chip** into the composer (`#/router · the
  last decision`), so "ask about this" becomes literal instead of a new
  chat with a pasted string.
- **R3 — The status bar shows only truths** (design intent #2). Supersedes
  whatever ruling currently keeps the three zero pills persistent; the
  next session finds and annotates the ruling in `UI-DECISIONS.md` when
  the phase lands rather than guessing here.
- **R4 — Persona tints are a token tier, not hexes** (design intent #3).
  Amends the colour rule in `design-system.md` in the same commit.
- **R5 — Rooms are a stage with a cast** (design intent #5). Supersedes
  the rooms column's snippet-row and the transcript's prose-block shape.
- **R6 — The bots surface is a door with passports.** Presence, paired
  devices as passports, and a proof of life replace the current
  dashboard shape. Extends the bots surface's own doc (pairing model
  unchanged: capability fixed at pairing, tokens as digests).

## Status pass, 2026-11-20

**P0, first item landed: the persona tints.** Six `--dsw-alias-persona-*`
tokens in all four themes, chosen by computation rather than by eye — each
clears the AA floor against its own theme's ink (measured 4.61–6.20:1), and
`themes::tests::every_theme_carries_six_readable_persona_tints` extends the
existing check rather than bypassing it.

Two things worth recording from the doing:

- The contrast theme's ink is **black**, so its tints had to be found from
  the *darkest passing* lightness; the first pass produced six near-white
  discs that cleared AA trivially and would have been indistinguishable in
  use. A test can prove readability and still let you ship mush.
- The field is `#[allow(dead_code)]` on purpose: the face reads the tints from
  the stylesheet, so the struct field exists to prove them, not to serve them.

Still open in P0: `answerHead`, `carry`, `presence`, and the topbar's
truths-only render.

## Phases

Each phase lands whole (build → harness → screenshots → gate) before the
next begins, and each names the screenshots it refreshes.

### P0 — The primitives (no surface changes yet)

- One `answerHead(surface)` builder: eyebrow · display title · the
  sentence · the verb; the harness pins its shape and forbids a count
  strip inside it.
- One `carry(obj)` that composes the typed reference and focuses the
  composer.
- One `presence(persona)` that returns the disc, the tint alias and the
  state word from a single table.
- `--dsw-alias-persona-*` (six) in all four themes; `themes.rs` AA check
  extended to them.
- The topbar's render path reduced to "truths only".

**ACs.** Every builder exists once and has a harness case; a surface
rendering a count strip inside `answerHead` reds the harness; all four
themes clear AA on the six persona tints (the existing `themes.rs` check
extended, not bypassed); the topbar renders zero pills on a fresh home
(harness case on the seeded empty document).

### P1 — The conversation: one field, a receipts gutter, fork-at-turn

- **The door (first run)**: three steps, each with its receipt — an
  account answers, a first run completes, a first fact lands. The
  landing's metric bar retires with the topbar's zeroes; the door is the
  only thing on a fresh home, and the conversation is what it opens into.
- Composer: one input, a cycling mode chip (`Ask before → Autonomous →
  Full access`), the folder chip; `Plan`, `+ goal`, attach stay as `/`
  commands and the two icon doors; `send` remains the only filled button.
- **The receipts gutter**: a 28 px spine per turn carrying glyphs for
  route · recall · skill · approval · tools, each expanding to its card.
  The cards stay in the flow (hard rule 5 keeps them in the log), the
  gutter makes the anatomy scannable.
- **Fork-at-turn**: a per-turn `⋔` that opens a new conversation seeded
  from the log through that turn (the session write path already
  supports creation; the fork is a copy of records, never a mutation).
- Artifact-producing tool calls render a preview card (frame thumbnail,
  file chip, deck cover) that opens the pane.

**ACs.** The composer renders one text field and no more than three
controls beside send (harness counts); the fresh home renders the three
door steps and no metric bar (harness on the seeded empty document); a
turn's gutter carries its route/recall/skill counts equal to the session
log's own records for that turn (harness + a fixture session); a fork
from turn N produces a new session whose records equal the source's
first N records (unit test on the fork function, plus a live hand
check); a `frame` tool call renders a preview card, red-proved against
today's timer-only card.

### P2 — Work, board, desk, inbox

- **Work**: three lanes (`needs you` / `working` / `background`), one
  line per row = state word + last event, hover preview (three lines +
  artifacts). Archive becomes a row state, not a button.
- **Board**: a derived kanban (`needs you → blocked → running → review →
  done`) fed by plan graphs + session state; cards are plan nodes that
  open the plan at the node; no drag, because the plan file is the source
  of truth and a drag would create a second one.
- **Desk**: artifacts-first — "what this conversation made" as
  thumbnails with versions and a handoff packet press; the run's receipts
  move to the conversation gutter.
- **Inbox**: triage-one-card. The inbound item's transcript inline, two
  verbs (accept → becomes a conversation, decline → named reason), and
  **what the door heard** so the surface proves it works. No list until
  three items exist; the card states which peer and which capability it
  arrives under.

**ACs.** A row with no end record reads `interrupted`, in words (red-proof
against today's card); the kanban's columns equal the derived states for
a fixture set of plans+sessions (harness); the desk lists every artifact
the session's log names and nothing else (fixture comparison); an inbox
item's accept produces a conversation whose first record names the peer
(fixture + red-proof against today's list); three screenshots refreshed
(`surface-sessions`, a new `surface-board`, `surface-desk-drawer`).

### P3 — The rooms and the bots (the owner's explicit ask)

**The rooms stage.**
- The rooms column's rows carry the **speaker now**: persona disc + name +
  verb (`designer is writing`), then the room's outcome count; a snippet
  only when nobody holds the floor.
- The room head becomes **the cast strip**: every member as a presence
  disc with its role verb and state word, the lead marked, `you` first;
  the shelf becomes the outcomes bar at the stage's top.
- The transcript becomes **turns**: a speaker spine (2 px persona tint),
  the name in the display face, the message; artifact cards inline;
  gaps/hires as cards with their verb (`convene advocate`); system events
  as hairline mono lines. Mentions stay hairline-delimited lines.
- The composer: recipient chip (`to the room` / `@name`), goal chip, one
  send; `/breakout`, `/distil`, `/archive` commands; the chip states who
  hears the draft before it is sent (kept).
- **Live turn-taking**: while a run is in the room, the stage's current
  turn carries the persona's state word and the hairline progress; no
  shimmer.

**The bots door.**
- Left: **personas** (the same cast list as rooms), each row carrying its
  role verb, last spoke, and "in N rooms".
- Right: **the service** (listening / off, one press), then **paired
  peers as passports** — name, device class, capability (`acts` /
  `listens`), paired when, last heard, last asked, revoke — and **what
  the door heard**: the last 10 inbound events with refusals named.
- **One proof of life**: "send myself a message" runs a rehearsal through
  the door and shows the round trip (the pairing route already exists;
  this is a surface over it, not a new channel).

**ACs.** A room row names the speaking member while a run is in progress
and never shows a snippet in that state (harness, live-run fixture); the
cast strip renders one presence disc per member with its state word
(harness count equals the room record's members); every transcript turn
carries a speaker spine and the persona's named tint (harness);
`/breakout`, `/distil`, `/archive` reachable from the composer and the
head (harness); the bots door renders a passport per paired peer and a
door feed of at least the last 10 events (fixture + live pairing);
"sends myself a message" completes a round trip and is named in the feed
(red-proof against today's 0-of-0 card); screenshots refreshed
(`surface-huddles`, `surface-bots`, plus a phone-width `huddles`).

### P4 — Intelligence: router, delegation, memory, skills

- **Router**: the live decision as the hero (sys1 label, chain walked,
  each refusal with its reason, winner cost/latency); a **cost × quality
  scatter per workload** with quality = the calibration grade; the hourly
  bar chart and the model donut retire.
- **Delegation**: the plan graph overlaid with what ran — divergence has
  its own tone (`never spawned`, `ran twice`); each node carries its
  judged workload as "why this child".
- **Memory**: opens on **what this run recalled**; rows carry full
  sentences; each hit opens a **why-this-recall** breakdown
  (vector/FTS/importance/recency); a **stale hunter** lists
  superseded-but-live facts and contradictions.
- **Skills**: a **coverage map** (task classes × has-skill) and a **fired
  feed** per run; the 449-skill library stays behind it.

**ACs.** Router's scatter plots one point per (model × workload) with a
calibration grade and refuses to plot ungraded cells as graded (labels
them `ungraded`, the KI-547 lesson); delegation shows a fixture plan's
divergence counts equal to the fixture's declared graph minus the run's
log; memory's recall section is the first painted block (harness); the
why-this-recall panel shows four weights summing to the score; skills
shows at least one coverage gap from a fixture session; screenshots
refreshed (`surface-router`, `surface-delegation`, `surface-memory`).

### P5 — Knowledge and craft: second brain, workspace history, learning, design

- **Second brain**: a **morning read** — five lines, deliverable to the
  phone — then traffic lights per project, the atlas as drill-down.
- **Workspace history**: a **time scrubber**; the tree animates through
  the refs' own order (no invented time); the store accounting becomes a
  footer.
- **Learning**: **practice first** — today's practice generated from the
  graph's weakest link; a held proposition links to the fact it became.
- **Design**: the in-run review loop (a design run reads its own page and
  iterates in the same run), a visual version diff beside the pins'
  fix thread, and the handoff packet as the primary press.

**ACs.** The morning read is five lines sourced from the second-brain
store (harness fixture) and available on the bots door; the scrubber's
positions equal the substrate's ref order (fixture); practice card
exists for a fixture graph and names the weakest link; the design run's
review loop is proven by an eval scenario (the run reads the page it
wrote); the version diff renders two versions' pixels side by side
(screenshot).

### P6 — System: settings, vault, plugins, calibration, about, armada

- **Settings**: **five intent rows** (model posture, approval posture,
  spend posture, schedule, identity), each expanding to the knobs it
  governs; the 23-knob summary strip retires; the save receipt stays.
- **Vault**: an **unlock map** — each sealed name beside what it unlocks
  ("without it: router candidates fail by name") and a **test this key**
  press over the existing probe; the `.env` migration becomes a guided
  card that names the delete-the-plaintext step.
- **Plugins**: a **manifest diff** per bundle (allowed vs asked, answered
  vs refused) and a three-row tier table.
- **Calibration**: a **grader scorecard** per op (precision of the
  judge/delegate beats) and the **counterfactual replay** — "turn this
  gate up and these decisions from the last 7 days would have flipped,
  here they are".
- **About**: folded into Settings → system as the build receipt; the
  apply door stays.
- **Armada**: a deployment map (instance · health · role · what it
  carries) with the dispatch bar.

**ACs.** Settings' first paint shows five rows and no count strip
(harness); each intent row expands to exactly the knobs its mapping
names (fixture map, both directions); the vault map lists every sealed
name with a resolver path and a test result per name; the manifest diff
shows allowed-vs-asked per bundle from a fixture ledger; the
counterfactual names every decision that would flip for a fixture stream
and never invents one (red-proof); About's route redirects into settings
and the rail no longer carries it; screenshots refreshed (`surface-settings`,
`surface-vault`, `surface-plugins`, `surface-calibration`).

### P7 — The visual pass (the appeal mandate, measured)

- A token audit across all four themes: every new surface element reads
  aliases only; no hex, gradient, glow or glass (design-system
  anti-patterns stay law).
- The display ladder used for every surface title and every room name;
  mono for every measured value; the label voice only for eyebrows.
- Motion budget audit: at most one transition per state change; nothing
  animates on load; reduced motion off by one attribute.
- Empty states: four sentences per surface (empty, loading, refused,
  selected) — never a skeleton.
- **The gallery refreshed**: every `docs/images/surface-*.png` retaken
  from the shipped build at 1440×900 and 390×844; the README's surface
  strip points at them.
- Hand walk at the four breakpoints (1180/900/720/460) with a written
  note per surface.

**ACs.** `themes.rs` passes with persona tints included in all four
themes; a grep guard finds no hex outside the token block of `app.css`;
the harness's empty/loading/refused cases exist per surface; the gallery
files' mtimes are all this phase's; the hand walk's notes land in the
plan's Status pass; `node tests/face/harness.mjs` green with a case count
**greater** than 197 (the count never falls).

## Files to be touched

- `src/gui/assets/app.js` — every surface builder named above; `ICONS`
  gains only concepts the surfaces name
- `src/gui/assets/app.css` — persona alias tier, the stage/transcript
  components, the answer head, the gutter, the passports
- `src/gui/assets/index.html` — the shell's panel stubs where the shape
  changes (topbar truths, cast strip)
- `src/gui/themes.rs` — the AA check extended to persona tints
- `src/gui/bridge.rs` — only the reads the new surfaces need (no route
  renames; the 2026-11-20 bridge split lands first or concurrently, and
  this plan's edits rebase onto it)
- `tests/face/harness.mjs` — every phase's new pins; existing cases
  updated only where this plan changes their shape, never deleted
- `docs/architecture/design-system.md` — the persona tier, the answer
  line, the stage components
- `docs/architecture/surfaces.md` — each surface's new shape, one
  paragraph per surface
- `docs/plans/UI-DECISIONS.md` — R1–R6 recorded with their supersessions
- `docs/images/surface-*.png` — refreshed per phase, fully in P7

## Out of scope

- **The TUI.** The same answer-line law belongs there, but the terminal's
  idiom (cards, width guards, key maps) is its own plan; the card-width
  primitive plan is the terminal's prerequisite.
- The graph/mockup research behind the ideas (already recorded in
  `docs/architecture/gui-design-research-2026-10-17.md` and the design
  mockups in `docs/design/`).
- Any framework, bundler or build step (hard rule 12); the face stays
  hand-rolled, served from the binary.
- Bundle internals and the armada deploy path (their surfaces change,
  their engines do not).

## Risks, and the red-proof each phase carries

- **The harness is the blast radius.** 197 cases pin today's shapes; each
  phase updates the cases it intentionally changes and adds new ones. A
  deleted case is treated as a regression unless the phase's Status pass
  names why.
- **The zero-pill removal may hide a real signal.** The topbar keeps
  rendering a value the moment it is true; the empty-home case is pinned
  so "quiet" can never become "blind".
- **Room redesign collides with KI rows that name speakers.** The
  transcript's speaker attribution is load-bearing in the log; the stage
  renders from the same record, never from a new client-side truth.
- **The counterfactual replay must never invent a decision.** It reads
  recorded rows and the gate's own rule; a row that cannot be replayed is
  named, not guessed.
- **Persona tints are one new colour axis.** Six hues against four
  themes is 24 AA checks; `themes.rs` owns them, and a failure is a
  blocked phase, not a ship-with-warning.

## Verification

1. The dev loop: `scripts/face-dev.sh live` for every iteration
   (zero-build, `FLEET_ASSETS_FROM`), `scripts/face-dev.sh check` for the
   harness per change, `scripts/face-dev.sh build` before a phase's
   screenshots.
2. `node tests/face/harness.mjs` — green, count never below 197.
3. Hand walks at 1180/900/720/460 per phase; screenshots into
   `docs/images/`.
4. The full four-leg gate once per phase (the face is `include_str!`'d,
   so the binary is part of the check).
5. `python3 scripts/plan_check.py` at completion; every box ticked with
   its evidence named.

## Linked artifacts

- `docs/architecture/deep-dive-2026-11-20-fleet-state.md` — the dive this
  plan's surface critique came from.
- `docs/architecture/design-system.md` — the system this plan extends.
- `docs/architecture/surfaces.md` — the per-surface anatomy being
  reshaped.
- `docs/plans/UI-DECISIONS.md` — the rulings ledger this plan adds R1–R6
  to.
- `docs/images/surface-*.png` — the gallery, 2026-11-04 vintage, refreshed
  by P7.
