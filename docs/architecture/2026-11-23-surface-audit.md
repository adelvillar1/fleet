# The surface audit — every page measured against the bar

The method (the surface pass plan, R2): per surface, every object it renders
is classified **act / drill / explain / dead / noise**; dead objects are named
(pressable-looking with no press, or a press with no effect); noise is
anything that only shows a number or a label. Evidence is a probe face
(`fleet gui --no-open --port 3995`, the same home, read-only) and the
documents it reads — never the owner's live window captured into a document.

The bar: an object must **act**, **drill down**, or **explain**. A card with
a number and nothing else is noise.

## 2026-11-23 — connections (the moved surface)

The graph moved from a permanent third column to a knowledge surface
(`8c131cac`). Inventory as measured on the live home:

| Object | Count | Job now | Class |
|---|---|---|---|
| focus node (centre) | 1 | opens the focused object's lens | act |
| ring nodes | 9 | drill the focus in place (graph re-centres, card names it) | drill |
| far nodes / "N more" | 1 | expands / collapses the rest | drill |
| legend chips | 5 | isolate a kind; press again to clear | act |
| focus card | 1 | kind, title, status, delegated processes | explain |
| card action ("open threads") | 1 | opens the kind's lens | act |
| where-line | 1 | "what this connects to · N more below" — states the picture | explain |
| hover card | 1 | names a node and its kind | explain |

**Dead object found and fixed:** the card's **"open skills" press did
nothing** — `pivotTo` read `FOCUS_LENSES` only, and skills carries a lens in
the kind table but no follow hook. Fixed in `8c131cac` (the kind's own lens
is the fallback), pinned by a skill-focus boot that must land on
`#/memory` with the intelligence panel shown.

**Noise removed:** the legend was five captions with counts; each is now a
press that isolates its kind, so the number is the reason to press rather
than the whole story. The old column's collapse chevron, reveal strip and
console switch were controls over a column that no longer exists — retired,
not left lying.

## 2026-11-23 — conversations (R3's measurement)

The owner's rule: only human-initiated or human-involved conversations.

Measured on the live home (2026-11-23):

- `/api/sessions` in the window: **99 rows**, of which **40 are
  `class: background` / `kind: scheduled`**; the whole home counts
  `background: 948`, `verification: 90`, `empty: 45`.
- The conversations list draws **12** rows ("active 12 · archived 47 ·
  all 59"), and **none** is a scheduled run; the head says "12
  conversations", which matches `counts.yours` and the drawn rows.
- The archive filter draws **47** owner rows, still **zero** scheduled.
- The connections graph's thread nodes: **16, zero scheduled**.

**Verdict:** the rule already holds on the conversations list, its archive
filter and the graph — `railRow()` (class `yours` or absent, and not a shell)
is the one predicate, applied before every state filter and the search
counts. So the system sessions the owner calls noise are **not** coming from
these surfaces. The audit's next targets, named rather than guessed: the
Home's changed-since block (events across sessions), the inbox's rows, the
PWA/bot face, and any surface that prints a count of the unfiltered
document (`background`, `verification`, `empty`).

## The deep pass — every surface, what it is, carries, and should be

Measured on the live home at 1440×900 (probe face, drawer closed), and read
against the owner's bar. Counts are the auditor's (semantic presses, visual
cards, cards with no semantic press inside, nested cards).

### The grammar the fixes follow

- **G1 — one container depth.** A surface is sections (heading + rows). A
  card is an object unit, never a box inside a box: the router's chain rows
  inside a card inside the page (9 nested) and the vault's form inside a
  card are flattened.
- **G2 — a card is a press or a host.** Every card either opens its object
  (whole-card press, `role=button`, cursor, keyboard) or contains presses.
  A card with neither is deleted — the read-only strip proved the point.
- **G3 — a number is a door.** A count that matters sits on the press that
  reveals its population (a chip, a row, a filter). Calibration's "0
  judged", the router's "15 calls", memory's strip: numbers with no press
  are noise.
- **G4 — one representation per fact.** The Intelligence rail already lists
  memories/skills/tools/workflows interactively; the four read-only cards
  above it duplicated them. The rail wins; the cards retired.
- **G5 — explicit affordance.** Every press is a `<button>` or carries
  `role=button`/`tabindex=0` with `cursor: pointer`. The audit found **zero
  pointer-cursor divs in the whole face**: where a div has a listener (the
  board's task rows, the receipts), nothing tells the owner and the
  keyboard cannot reach it.
- **G6 — one page head.** One eyebrow/title/sentence per surface. Learning
  renders "learning" twice (column and page); the router's crumb says
  `SETTINGS / advanced` while its eyebrow says `INTELLIGENCE`.
- **G7 — an empty state names its one press.** The board explains the rule
  and offers nothing; the inbox shows a bare "waiting" chip.
- **G8 — a loading state carries elapsed time and a retry.** Learning and
  Second brain were still "loading" at 5 s with no door (the read-heavy
  lenses are known to sit past 25 s — the state must say so).

### Home / inbox
- **Purpose:** what needs the owner, ordered by the age of the hold.
- **Today:** one section chip ("waiting"), a title floating top-center, the
  rest empty; no head sentence, no empty-state press. 0 cards after the
  strip retired.
- **Should be:** a page head (what it is, what it holds), the hold rows
  (why + age + the one press each), an honest empty state ("nothing needs
  you") with the one door (conversations), the held count in the head.

### Conversations
- **Purpose:** the owner's work threads; open one, start one.
- **Today:** 12 rows ("active 12 · archived 47 · all 59"), zero system
  sessions (the owner's rule already holds); a duplicated "work"/head.
- **Should be:** rows carry title, age, state and open on press (they do);
  the duplicate count logic collapses to the head; the composer is the
  only create door.

### The board
- **Purpose:** approved plans at execution level — each plan's tasks in
  their column (queued / in-process / done) and who each is for.
- **Today:** a head and one card: "…/docs/plans holds no approved plan — a
  plan contributes work once it is approved". Read-only; no door.
- **Should be:** plans as cards (a plan = its columns of tasks, tasks are
  presses to their context); the empty state's press is "open the plans
  folder" or "ask for a plan"; the counts on the columns, never alone.

### Intelligence
- **Purpose:** what the home knows and can do — memories, skills, tools,
  workflows.
- **Today (post-fix):** the segments rail is interactive (today's recall
  64, project 63 → skills 460, tools 26) and the main column holds the
  selected segment's rows; the four read-only capability cards retired.
- **Should be:** the rail owns the segment choice; the main column is the
  selected segment's list (rows that open the drawer, and "460 skills" becomes
  browse/search); the corpus strip's six numbers become filters; the "ask
  what fleet knows" instrument stays.

### Connections
- **Purpose:** what the focused object connects to.
- **Today (R1):** the page, its nodes drill, its chips filter, its card
  acts; the dead "open skills" press fixed.
- **Should be:** resolved; the audit of the focus card's meta (status,
  delegated processes) is at explain-level, acceptable.

### Learning
- **Purpose:** the measure → plan → teach loop — projects, lessons,
  propositions, practice.
- **Today:** "learning" twice (column + page), a "reading…" list and a card
  stuck at "Still loading", then only the composer.
- **Should be:** one head; projects and lessons as rows with state and a
  press; the loading state names elapsed and retry; the empty state offers
  "start a lesson" (the composer's own command).

### Desk
- **Purpose:** name a deliverable and the one claim it must carry; the desk
  reads the sources and refuses to call work done while a claim stands.
- **Today:** a deliverables rail (2) + a large invitation + composer; the
  rows carry "0 frames · design · unbound" (jargon); the main is an
  invitation (good).
- **Should be:** the invitation stays; the rows name their stage in plain
  words and press to the deliverable's stages; "unbound" → "no claim yet".

### Calibration
- **Purpose:** the day's stream, the judged quality of what served, the
  gate that decides whether the failover may learn.
- **Today:** a strong rail (the gate, routing, routes 7, turns 7, eval runs
  2, recall hits 180, …), six stat numbers with no press, four report cards
  (gate, scorecard, counterfactual replay) with honest empty states, and a
  floating "the stream" instrument (three presses).
- **Should be:** the six numbers are doors (0 judged → run the judge; 512
  rows → the stream); the rail stays the nav; the report cards keep their
  explanation and gain their one press; the instrument keeps its three.

### Design
- **Purpose:** boards as canvases — draw, arrange, brief the designer.
- **Today:** the most usable surface: boards rail, toolbar (zoom/fit/grid/
  assets), a canvas with an honest empty state, the conversation card; the
  crumb says `SETTINGS / advanced` while the rail says DESIGN.
- **Should be:** the crumb fixed; the empty canvas names its one action;
  the conversation card is the composer (keep).

### Armada
- **Purpose:** the fleet — instances, their liveliness, the relay, dispatch.
- **Today (post-fix):** nine cards, seven read-only (deployment map, relay,
  dispatch bar, …); one card carries a real press (refresh); the title sits
  after the cards in reading order.
- **Should be:** the head leads; every state card gains its press (refresh,
  pair, open the paired device); the map's rows (when present) are presses;
  the empty states already name what fills them.

### Second brain / health
- **Purpose:** the background services, the doctor's checks, the build.
- **Today:** a rail (services/doctor/build) and one services card stuck at
  "Still loading the connection checks", no page head.
- **Should be:** a head; services as rows with live state + retry; the
  doctor's checks with their fix presses (the cards exist); the build
  receipt with its apply press.

### Workspace history
- **Purpose:** every version of the work.
- **Today:** two cards, one read-only; sparse.
- **Should be:** versions as rows (what changed, when) that press to the
  diff/restore the CLI already supports; measured in its own pass.

### Vault
- **Purpose:** the keys this home holds.
- **Today:** a tool card plus a config form nested inside a card (2 nested);
  rows exist for secrets but the surface reads as a settings form.
- **Should be:** secrets as rows (name, scope, age) with reveal/copy/rotate
  presses; one container depth.

### Plugins
- **Purpose:** surfaces installed on top of fleet, their tiers and diffs.
- **Today:** the tier table, two manifest-diff cards, nested sections; 52
  presses (the most on the config side).
- **Should be:** plugins as rows with enable/disable and query details; the
  tier table as a reference section, not the page.

### Bots / assistants
- **Purpose:** the named helpers and their channels.
- **Today:** one card and two flat units; 32 presses.
- **Should be:** assistants as rows with role/channel/state and the
  add-a-bot door; measured in its own pass.

### Settings
- **Purpose:** every knob, with its source.
- **Today:** 79 presses, zero read-only cards — the counter-example to the
  rest; forms and configs.
- **Should be:** kept as the reference form grammar; audit only its labels
  and grouping.

### Router
- **Purpose:** where the work went, what it tried first, what the day served.
- **Today:** six stat numbers (15 calls, 15 OK, 0 errors, 100% success, 0
  tokens, sys1 judge) with no press; "the declared chain" card holding nine
  nested chain rows; a floating instrument (ask sys1, open settings); the
  crumb `SETTINGS / advanced` contradicts the eyebrow `INTELLIGENCE`.
- **Should be:** the chains flattened to rows; the six numbers doors; the
  crumb fixed; the model chips keep their copy press (they work and are
  keyboard-reachable — the one good nested unit).

### Delegation / helpers
- **Purpose:** the helpers a task can be given to.
- **Today:** three read-only cards; 34 presses elsewhere.
- **Should be:** helpers as rows with capability, state and a press;
  measured in its own pass.

### Huddles / rooms
- **Purpose:** the owner's rooms — members, goal, state.
- **Today:** one card, one read-only; a room rail exists.
- **Should be:** rooms as rows (name, members, state) pressing to the room;
  the archive press stays.

### The drawer's four tabs
- **Today:** read · act · connections · made here. The "connections" tab is
  the conversation's goal edge and delegated work — a different object from
  the connections page, and the shared name confuses.
- **Should be:** rename the tab to **work** (what it carries); the other
  three are right; audit each tab's objects in its own pass.

## The fix order

1. **The read-only spine strip** — six surfaces, one shared builder: retired
   in this row (measured: inbox 4 → 0 cards, memory 4 → 0).
2. **Stat strips become doors** — done: calibration (6), memory (3), router
   (1) — each onto the section that shows its population; armada's strip
   retired with the read-only cards.
3. **Empty and loading states** — inbox, board, learning, second brain:
   every one gains its head, its sentence, its one press, elapsed + retry.
4. **Router flattening + crumb** — the nine nested rows, the six numbers,
   the misleading breadcrumb.
5. **The remaining per-surface passes** in the order above, each with its
   own row, its mockup where the grammar changes, and its pins.
6. **The drawer tab rename** (connections → work) with the plan's G11
   amendment recorded in the gap ledger.

## Design-system conformance (measured 2026-11-23)

The system's contract is `docs/plans/2026-11-22-harness-redesign.md` (D1–D7:
Indigo Studio tokens, one accent, the elevation law, one head, rail | stage |
drawer) with the tokens in `app.css` (`--dsw-alias-*`, `--shadow-1/2/3`) and
the component builders (`answerHead`, `surfaceStrip`, `surfaceInstrument`,
`boardCard`, `section-rail`).

**Measured baseline, and what changed in this pass:**

| Check | Baseline | Now |
|---|---|---|
| Raw colours outside the token blocks (CSS) | 1 (`rgb(0 0 0 / 45%)` on the connections hover card) | **0** |
| Raw box-shadows outside the elevation tokens | 1 (the same rule) | **0** (the tokens, `none`, and focus-ring states only) |
| Inline style colours on every surface | 0 | 0 |
| Page heads per surface | **3 grammars**: `surfaceHead` (22 sites, `surface-head`/`surface-title`), `answerHead`/`pageHead` (4 sites), and the health page's own header | **one** — `surfaceHead` is now an adapter onto `answerHead`, and the health page folded in; measured live: memory, armada, calibration, router each paint exactly one `.page-head`. The health page's **loading** path still paints none (R6). |
| Section rails | 10 of 19 surfaces | unchanged; a surface with sections must use `.section-rail` (per-surface rows) |
| Card language | `.board-card` on 13 surfaces; bespoke containers on desk/learning (`div.column`), design (`design-body`), delegation/router/vault/plugins (`div.board` only) | unchanged; each surface's row brings it onto the card grammar |
| Type ladder | 4–10 distinct font sizes per surface | unchanged; the per-surface rows reduce to the ladder (U46's floor; display/body/mono) |

**The rule, added to every surface row:**

- A surface renders **one** `page-head` (`answerHead`), one `.section-rail`
  when it has sections, and `.board-card` units at one container depth.
- No raw colour, shadow, radius or font-size outside the tokens/ladder; a
  new value needs a token first.
- The head carries eyebrow · title · sentence · (instrument); the instrument
  is `surfaceInstrument`, never a hand-built box.
- The design system is not a coat of paint applied at the end: a surface is
  not done until it renders through the system's builders.
