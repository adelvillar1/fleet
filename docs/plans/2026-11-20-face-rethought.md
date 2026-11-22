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

**P0, second item landed: `presence(persona)`.** One table carrying a
persona's tint, glyph and state word; `huddleMark` draws from it and
`HUDDLE_TINTS` is retired (three facts that had to agree became one that
cannot disagree). An unknown persona still gets a mark — its own letter, the
accent, a neutral word — because an empty disc is worse than an unnamed one.
The harness asserts the contract from the DOM: a `designer` disc takes
`var(--lane-4)`, 1484 checks.

**P0, third item landed: `answerHead(surface)`.** One builder for a surface's
head — eyebrow · display title · the sentence · the verb — with `pageHead`
kept as its positional door so all seven call sites render through it and the
classes the stylesheet names stay put.

Its harness case **is** landed, and getting there corrected an error of mine:
the first probe opened `#/bots` and `#/huddles`, which are not the board route,
reported zero heads, and I logged "the harness never paints a board". Wrong —
`#/board` paints one. KI-583 is closed as a coverage gap that no longer exists:
the contract case now asserts eyebrow, title and sentence are present and that
no `.badge` or `.surface-strip` rides inside the head, red-proved by removing
the sentence from the builder. Harness 1486 checks, 207 cases.

**P0, fourth item landed: `carry(obj)`.** The composer's reference insert was
inline inside the `@` completion; it is now `carry(one)` — replace the trailing
token, save the draft, grow the field, close the list, fire the kind's follow-on
(a file opens its pane, a session records its link), keep focus — and
`insertReference` delegates to it. Every future card action lands through the
same door, so spacing, drafts and focus cannot disagree between surfaces.

Its coverage was already there and I nearly duplicated it: `C5 Enter inserts
the reference into the sentence` asserts the insert landed *and* that the file
pane opened — `carry`'s whole contract. The extraction keeps it green.

**P0, fifth item landed: the topbar keeps quiet.** A fresh home has no route,
no run and nothing measured, so it shows no pills and no zeroes — the old strip
read "no route yet · 0 · 0% · idle", which is a fault report on a machine that
is working perfectly. Red-proved by removing the three hide lines: the case
failed on all three, naming each.

**P0 is complete** (5 of 5). The harness carries 1489 checks across 208 cases,
including the fresh-home case, the board-head contract, the persona tint and
the carry.

### P1, scoped 2026-11-20 (measured, not estimated)

The composer does not need trimming; it needs a rebuild, and the numbers say
how much:

- **19 visible buttons** in the composer block (`run-perms`, `run-mode`,
  `run-provider`, `run-folder`, `run-goal`, `run-goal-btn`, `plan`,
  `dispatch`, `send`, `composer-steer`, `composer-btw`, `composer-dictate`,
  `composer-add-bot`, `doc-open`, `attach`, `paste`, plus three inside the
  folder and goal popovers), and **three hidden**.
- **Two textareas** (`task` and the goal popover's input).
- **Three popovers** (`folder-pop`, `doc-pop`, `goal-pop`) with their own
  submit and cancel buttons.

P1's target is one text field, a cycling mode chip, a folder chip, and `send`
as the only filled button — so the shape of the work is:

1. `run-mode` + `run-perms` collapse into **one cycling chip** (ask → autonomous
   → full access), which is a permission control and therefore the one that
   must never be hidden or ambiguous.
2. `plan`, `run-goal`/`run-goal-btn` become `/` commands; the `goal-pop`
   textarea goes with them.
3. `dispatch`, `steer`, `btw`, `dictate`, `add-bot`, `doc-open`, `paste`,
   `attach` are audited one by one: keep, fold into the `/` list, or delete.
   The AC is "no more than three controls beside send", so most of these are
   deletions, and each deletion is a decision about a habit the owner has.

That is a surface rebuild with a permission control in the middle of it, so it
lands as its own pass — the same way P0's topbar did — rather than as the tail
of a long session.

**2B started: the permission pair is one cycling chip.** `run-mode` was a
`<select>` inside `run-perms`, so the two "controls" were one control with a
menu — three sentences to read to change one word. It is now a press: ask →
autonomous → full access → ask, and **full access is skipped entirely without a
working folder**, so the cycle cannot arm a grant the server would refuse. The
chip's word is its own answer; the sentence behind it rides the title.

Coverage: the run bar had **no case at all** for any of its four controls. It
now has three — the chip's cycle, the folder door and the document door —
including a no-folder lap proving full access is out of the cycle.

The fourth (the route picker) cannot be covered: the fixture answers a generic
`div` for `run-provider` and never populates a select's options. Recorded as
**KI-584** rather than asserted on a stub, which would have been a green check
proving nothing.

**P1, the composer's nine controls: decided, and the row is two doors.**
The plan deferred these to a per-control call; the owner's standing ruling
is that the agent decides with recorded judgment, so each is an `R` entry in
`docs/plans/UI-DECISIONS.md` (`R1`–`R9`, with `R10`/`R11` recording the two
scope judgments beside them). Two doors stay — the document door and
attach — and seven retire: `/paste`, `/steer`, `/btw`, `/dictate`, `/bot`,
`/plan`, `/dispatch <task>`. Steer and btw were deleted rather than folded
because their buttons only prefilled commands that already existed; the
other five gained commands that carry their bodies unchanged.

Measured: the composer's own field is one (the goal popover's textarea is a
dialog's field, `R10`), two controls sit beside `send`, and `send` is the
only filled button. `P1 composer one field and three controls at most`
counts all three from the markup and then proves every retired capability
answers by name in the type-ahead. Red-proved twice: deleting the
`/dispatch` command failed `every retired control answers by name —
/dispatch`, and putting the paste button back failed three checks —
`no more than three controls beside send — doc-open,attach,paste`,
`send is the only filled button`, and `retires the seven folded buttons —
paste`. Eight existing cases were rewritten to drive the commands instead
of the ids they used to press (the VERBS case flipped whole, from icons to
commands). Harness 1672 checks across 235 cases.

**P1, fork-at-turn: built on the log's own turn boundary.** A turn is one
model call, so the cut is the record where the *next* model call's
assistant record begins: everything through the turn, and nothing of the
turn after it. The face's turn counter restarts when a conversation is
resumed, so the count starts after the log's last `Resume` record — the
press, the log and the new conversation name the same turn. `fork_at_turn`
shares the copy path with the reply's exchange fork (`fork_copy`), so the
lineage line, the furniture (folder, goal, title) and the "source left
whole" rule cannot drift between the two doors. The route takes `{turn}`
beside `{exchange}`; a body with neither is a 400, not a guess.

Refusals are by name: turn 0 ("a conversation's turns start at 1"), a turn
past the run's last ("holds no turn 9 to fork at — its run has 2"), a
missing log, and a run in flight (the bridge's existing guard). The face
hides the `⋔` while the run is live — the log is being written — and
unhides it at the run's end with the rewind's own sweep.

Coverage: the unit test `a_fork_at_a_turn_copies_the_logs_own_prefix`
builds a two-run log and asserts the fork's records are the source's own
prefix (the fork's start line aside), that turn 2 copies the whole log,
that the source is still nine records, and all three refusals. The harness
case `P1 fork a turn mark forks at its own turn` drives the press from the
mark and asserts the body carries `{turn:1}` with the session, that the
fork opens, and that a 409's words land as a notice without moving the
view. Red-proved twice: cutting at the turn's own assistant record (instead
of the next one) failed the unit test with the fork one record short of its
answer, and dropping the turn from the post failed
`the press names the turn and the session — /api/session/fork?session=fork-1 {}`.
Harness 1678 checks across 236 cases.

**P1, the artifact preview cards: the row carries the artifact or names why
nothing can be shown.** A receipt whose record is a tool call that produced
something holds a preview card beside it, and the card follows the call's own
kind — a `frame`/`review_frame` renders the frame's own served page as a
sandboxed thumbnail (the same document the design surface serves, not a
re-drawing, with a chip for the page and a press to its board); a `send_file`
renders a file chip that opens its pane; a call that never ran names its
resolution; a record naming no artifact draws **no card** — a preview invented
from a call's arguments would be a claim, not a preview. Coverage:
`P1 preview a frame call renders its card` (10 checks — thumbnail, chip, board
press, the honest empties, and the non-artifact negative). Red-proved by
removing the `paintToolPreview` call: all ten failed by name against the
timer-only card. Harness **1688 checks across 237 cases**.

**P1, the door: measured, then covered.** The door already existed
(`paintOnboard` → `#onboard-door`, the landing token `data-onboard="door"`), so
the work was the case the plan asked for, not a screen. It now asserts:

- the ladder carries **one line per step the document declares** (7 in the
  fixture), each with its mark, and **exactly one** saying `now` — a door that
  always showed three would tell the owner a story the document does not;
- a first-run landing carries **no metric bar**: the token is `door` and no
  `.counts` rides inside `#empty` at all.

Red-proved by truncating the face's own step list to three: the case failed
`ladder=3`, then green.

One thing the measurement corrected: the stylesheet's comment promises that the
counts, method strip, lanes and footnote all retire on a first-run home, but the
only rule keyed on the token hides `.method` and `.onboard-cards`. The counts
are not in the landing at all, so the AC holds — the comment is describing an
older landing, and the case now pins the state that matters (the token plus the
absence of a bar) rather than trusting the prose.

**P1, the gutter: built, and derived rather than invented.** The three
measured facts above decided its shape, and it landed as the rail they
implied — not as a per-turn wrapper around the log's 30-odd row builders:

1. `#gutter` is a 28 px rail inside the log, one row per turn. It ships
   its own measure (the stylesheet's visual pass owns the look), so the
   face is readable before that pass lands.
2. Every count comes from one walk of `face.events`, grouped by the
   `turn` boundaries and tagged by the same rule `row()` tags rows by —
   so the rail cannot say anything the log does not. The spine's five
   counts map to `routed`, `memory`, `tool_call{use_skill}`, `control`
   and every other `tool_call` (the `skills` library-load event is a
   different receipt: what was loaded, not what was used).
3. A glyph with a zero draws nothing, a turn with no receipts keeps its
   place as a bare mark, and a press jumps to the turn's mark and opens
   its record (the tool row's detail, or the drawer's card for the strip's
   folded receipts). The cards never move out of the flow.

Coverage: `P1 gutter the rail counts each turn's receipts from the log`
counts a two-turn fixture by hand and demands the rail agree kind by kind,
that a kind the turn never recorded draws nothing, that the tool glyph
names its tools, and that a press opens the log's own records. Red-proved
twice: `gutterTurns` returning nothing failed nine checks by name
(`one row per turn`, each count, the glyph's title, the press), and
dropping the no-zero guard failed `turn two counts only what it recorded`
with `{"route":1,"recall":0,"skill":0,"approval":0,"tools":2}`. Harness
1665 checks across 234 cases.

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

**Status 2026-11-21 — AC1 landed, measured first.** The derivation was already
correct on both sides: `bridge::work_state` answers `interrupted` for an
unfinished log whose last write is older than `LIVE_WINDOW_SECS` (600s, pinned by
the Rust test at `bridge.rs:20072`), and `WORK_STATE_WORD` already carries the
word. What was missing was the *face-side* case — and the existing sessions case
seeded its `interrupted` row as a **background** row, which that same case
asserts is filtered out of the work list, so the word had never once been
rendered under assertion. `anInterruptedRowSaysSoInWords` seeds an interrupted
conversation and asserts the row says it in words and never `unknown`; red-proved
by pointing the map at `unknown`, which read
`"…a run that never endedunknown·fleet·29darchive"`.

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

### P2 inbox — built, reviewed, and the review found two real hazards

Five commits after the ruling (`4da00c7` … `44dd2a0`, docs `ac95779`), then an
adversarial review returned **SHIP-WITH-FIXES** with two findings a green gate
could never have caught, and five fixes landed (`f48a193` … `81ac1c5`):

1. **The hold store's read-modify-write was unlocked.** Three hazards, each
   proved red on the unfixed tree before the fix: two concurrent accepts both
   took the same key and ran the inbound **twice**; an accept and a decline both
   succeeded over one hold; and a stale write-back **resurrected a hold that had
   been accepted**, because the webhook serves each connection in a spawned task
   and the sink could file while an accept was inside `deliver`. Now one
   process-wide lock, and `take` persists before anything runs.
2. **A corrupt `held.json` lied.** The sink answered `Refusal::Held` ("it will be
   accepted or declined from the inbox") when the hold was never written — so the
   delivery was gone, not held — and the inbox swallowed the load error into an
   empty queue. Now `Refusal::Unfiled` tells the truth and the document carries
   `held_error`, which the face paints instead of the empty state.

Also from the review: a driver-refused hold is re-filed under a **new id**
(documented), the decline draft lives per row with the confirm disabled until it
has words, and hold ids carry full epoch nanos (the old tail repeated every
second).

### P2 inbox — owner ruling 2026-11-21: triage, as a deferral class

The AC ("an inbox item's accept produces a conversation whose first record names
the peer") has exactly one reading that grounds all four of its prose facts —
the inbound item's transcript inline, accept becoming a conversation, the card
naming peer **and** capability, and "what the door heard": **channel-door
traffic**. The owner ruled on 2026-11-21 that triage is wanted.

The measured constraint that shapes it: a task-capable peer's delivery is
**auto-accepted today** — `Driver::deliver` (`src/channels/mod.rs:984-1041`)
authorizes, creates the peer's session and starts the run, and its own doc
comment says so ("a delivery is accepted immediately"), with the D35 dispatch
journaling at :1026-1039. So triage is built as a **deferral class**, not a
replacement: a hold the owner can act on, leaving auto-run the default for
anything not held.

Two hard facts from the spec that the implementation must respect:

1. **The first record already exists, on one path.** `Driver::session_for`
   (`mod.rs:784-826`) writes `Record::Channel { channel, peer_id, peer }` as the
   session's first record (`src/session.rs:402-418`) — the record whose own doc
   says it is "what makes an inbox able to say 'via webhook · the CI box'". An
   accept path that calls `Session::create` directly produces a first record
   that does **not** name the peer, which is exactly the red-proof.
2. **The activity log cannot be the transcript source.** Its `task` field is
   only the first line capped at 120 characters (`src/channels/service.rs:186-194`),
   so the card's inline transcript needs the held `Inbound` (channel, peer id and
   name, full task, reply, at) or the session replay after acceptance.

Also required: no list until three items exist (the card carries the state
otherwise), and decline names its reason in the activity feed as a `refusal`
line rather than a silent drop.

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

### The standing goal, restated — 2026-11-21

The owner's directive after the v0.8.24 cut: **this plan is implemented end to
end — every phase, no deferrals, no lazy shortcuts.** The earlier session goal
covered P2–P5 only; that scoping is retired. The queue on the shared `app.js`
is therefore, in order:

1. **Laya sys1 integration** (in flight) — the one app.js writer while it runs.
2. **P1's remainder** — the receipts gutter, the composer's nine controls (the
   owner's per-control questions are answered by the agent with recorded
   judgment in `docs/plans/UI-DECISIONS.md`, not deferred back), fork-at-turn,
   artifact preview cards.
3. **P6** — the six system surfaces with their ACs.
4. **P7 wave two** — builder-side visual pass: surface shapes, icons,
   four-sentence empty states, icons, the hand walk at 1180/900/720/460 with a
   written note per surface, and the gallery refresh.

(P7 wave one — the stylesheet and theme engine — runs in parallel on
`app.css`/`themes.rs` and does not own `app.js`.)

**P7 wave one: landed and measured** (`35a8bcab` token tier, `b9ab5af7` display
ladder, `36565bcd` card vocabulary, `433441c1` motion budget, `be9214c4`
no-hex guard, `3cc2506a` static spine, `70a64894` design-system doc). The
before/after deltas at the same seed and rig: board **Δ3.20** mean (5.54% of
pixels), delegation **Δ5.13** (12.84%), huddles stable band **Δ3.16** against a
0.002 JPEG-shimmer control. Breakpoints 9/9 green with the stage guard; AA
floor raised — flat accent on day 3.6 → **5.31**:1, worst persona per theme
4.61–4.67. The no-hex guard scans `app.css` via the census and the 24 stray
literals became aliases; the red-proof added a hex to the body and the guard
named it. Wave two (builders, icons, the four sentences, hand walk, gallery) is
the `app.js` half, dispatched.

No phase is called complete until its rows are landed **and** its screenshot and
hand-walk clauses are satisfied from the shipped build; status `active` until
the whole table says so.

## Status pass, 2026-11-21 — P3–P5 audited before building

A read-only audit measured all **18** AC clauses against the tree (7 in P3, 6
in P4, 5 in P5). Result: **not one is already implemented** — unlike P2, whose
first two ACs turned out to be missing cases over working code. Two of them
contradict a *passing* test, so implementing them is a deliberate behaviour
change that must flip the case:

- **P3, the room row.** `roomRow` renders its snippet unconditionally, and
  `HUDDLES the row carries its last line and its author` (`harness.mjs:7513`)
  asserts exactly that. The AC requires the speaker's verb and **no** snippet
  while a run holds the floor. *(Landed 2026-11-21: `7f172af` flipped the case
  and added a companion proving parked rows keep their snippet.)*
- **P4, memory.** The AC says recall is the first painted block; `memorySections`
  leads with `needs-you` and `MEM the rail leads with needs you`
  (`harness.mjs:10843`) pins it.

The rest, in order of how much is missing:

| Phase | AC | State measured 2026-11-21 |
|---|---|---|
| P3 | room row → speaker while running | **landed** `7f172af` |
| P3 | cast strip: disc + state word per member | **landed** `3a79e38` — `PRESENCE.state` existed unused |
| P3 | turn spine in the persona's tint | **landed** `aa25e5a` |
| P3 | `/breakout` `/distil` `/archive` from composer **and** head | **landed** `6d9732c` — head buttons existed, commands did not |
| P3 | bots door: passport per paired peer + ≥10-event feed | **landed** `6b69e98` + `ed7bd32` + `1f7150f` + `7995dc6` — `device_class` persisted with a `serde(default)` field (old stores load unchanged: proven by test), last-heard/last-asked derived from the capped activity log rather than a new store field, `acts`/`listens` aligned across both bridge documents, and the passport rows in the face. The live-pairing half still needs a run. |
| P3 | "send myself a message" round trip | **landed** `a189fb5` + `79a001b` + `ca0c85f` — `/api/channels/rehearse` pushes a real `Inbound` through `service.sink()` (no bypass of `Driver::deliver`), so the session first record is `Record::Channel` naming channel and peer; the feed derives `rehearsal {at, session, delivered, answered}` with zero migration; the card says "The door heard you; no answer came back" when the run produced no reply line. |
| P4 | router cost×quality scatter + `ungraded` | **landed** `71c44aa` + `db36732` + `6f7a50a` — the scatter grades only what the day can grade (a cell is graded iff it exists and `fit`; otherwise it carries `ungraded` and its reason and claims no axis position), and the hour chart + model donut retire while `donut` itself stays (the tower's report card uses it). |
| P4 | delegation overlay with divergence tones | **landed** `8563f17` + `ef462cf` + `56daf8d` + `20bde1f` + `465eeea` — `DelegatedRunRow.node` (serde-default, old rows read `None`), the day's `OP_DELEGATION` rows joined to graph nodes as `overlay {spawned_sessions, spawn_count, divergence}` with a graph-level `divergences {never_spawned, ran_twice}`, `declared_workload` from the plan's `execution:` verdict carrying the child's judged shape as the why-this-child line, captions in the warn tone, and the FOLD harness case pins counts against a declared graph (9 checks). Red-proofs: dropping the node threading left the overlay empty (every node `never spawned`); deriving the word in the face made all three captions say it. |
| P4 | memory: recall is the first block | **landed** `0166960` — flipped the case that asserted the opposite, red-proved |
| P4 | why-this-recall breakdown | **landed** `354d886` + `87aa84c` — six channels recorded on the calib row (old lines decode to zeros and say so), served with their weights, summed against the score. Ranking untouched: no weight or `Score::compute` line changed. |
| P4 | skills coverage map + fired feed | **landed** `037494a4` + `119b68af` + `7bbfd4bd` — the document derives the coverage map (`boards::read_skills`): per skill `fired {count, last_at}` or nothing, an `unused` list, a newest-first `feed` of `{at, skill, session, surface}`, and a named `feed_reason` when nothing records fires; the face paints the fired/unused split (a skill with no fires reads "not used yet", never a failure), the feed, and — the honesty half — when no firing records exist the block prints **"declared only — nothing records fires"** instead of any zero-count claim. The harness pins both fixtures; red-proofs: the split lying ("3 fired" against a 2-and-1 document) and the basis dropped both failed by name. The child died in the opencode-go 429 after committing; the commits were verified and the row flipped before anyone re-ran the work. |
| P5 | morning read: five lines, on the bots door | **landed** `11322a5` — five sentences derived purely from the second-brain document; a line whose section did not land prints that section's own refusal, never a zero. The repo's first `second-brain` fixture. |
| P5 | workspace-history time scrubber | **landed** `b5c89d0c` + `46078ac6` — the document derives a `timeline` from the commits' own timestamps (hour buckets under a 3-day span, else day) and answers `at=<iso|ms>` with `as_of`/`covered`; the face's `canvas-range` asks for a moment and paints what came back (no client-side filtering), with the sentence reading `the newest · 3 of 3 changes` / `as of 14:32 · 2 of 3 changes` / `before the first change · 0 of 3` / `no changes recorded yet`. Red-proofs: filter no-op → `left: 3, right: 2`; coverage broken → harness named `as of 03:34 · 3 of 3` where the document said 2. |
| P5 | practice from the graph's weakest link | **landed** `84ea702` — the weakest link is derived **in the document** (`boards.rs`), never the face: a failed run first, then a landed delta that cost the aggregate (by `diff_weight`, now carried on `DeltaShape`), then the unverified node closest to done that still names why it is held; summaries never rank. Both the delegation board's graph and the learning read carry `weakness`; the practice card prints the document's sentence and offers it as the default target while the manual start stays below it (pinned). Red-proofs: face wiring removed → 4 checks failed by name; ranking broken to "last node" → `left: second, right: first`. |
| P5 | design run's review loop (eval scenario) | **needs a new eval rig**, not a harness case |
| P5 | version diff as two versions' pixels | **landed** `3b74e8a` — `Version.image` (serde-default, old stores unchanged), captures landed beside the board through the browser lane fleet already ships, served at `GET /api/design/version-image/:slug/:frame/:version`; the card draws two panes in a grid with a divider column, per-version labels and `image-rendering: pixelated`, inline styles only (no build step). A version with no capture says **"this version carries no image"** by name, with the capture one press away — never a blank frame. Red-proof: face wiring reverted → 5 checks failed by name. |
| — | **screenshot rig** (the plan's "screenshots refreshed" clauses) | **landed** `1dbb28e` — the 2026-11-04 recipe lived in `/tmp` and was gone; now `scripts/gallery-seed.py` (a scratch `FLEET_HOME` + project, refusing `~/.fleet`) and `scripts/gallery-shoot.mjs` (Playwright over the installed Chrome, one fresh page per route, 1440×900 @2x). Proved by the new `docs/images/surface-board.jpg` — **2880×1800**, populated. Two traps the memories had not recorded: the dev build's `/sw.js` navigates every client once after activation (absorb with one reload before addressing a route), and **KI-586** — the phone-width stage collapse, which the rig now fails by name rather than committing an empty frame. |
| all | screenshots refreshed (8 named images) | **landed** `1e0ed60b` + seed fixture `09806998` — all eight re-shot through the committed rig at **2880×1800** (phone 780×1688), each passing the readiness predicate and the KI-586 stage guard; `surface-delegation` now shows the P4 overlay (per-node captions + `3 never spawned / 1 ran twice`), which required seeding delegation rows carrying `node` — the old fixture could not have shown it. Known gap stated in the commit body: the pixel compare and practice card live on the design/learning surfaces, which are not among the nine targets, so no shot shows them. |

**Two corrections to this plan's own text**, both measured:

1. **P4's "four weights" are six.** `memory::Score` carries `vector, keyword,
   trajectory, history, freshness, provenance` — and its doc comment already
   calls it "the 'why' the surface prints". The `.55/.25/.10/.10` figures in
   the original P4 bullet are dead history. The AC should be restated as six
   channels summing to the hit's total.
2. **The passport's device class is not merely unrendered — it is unpersisted.**
   `PairedPeer` carries id, name, token hash, capability, created_at and
   callback; remote `Device` carries `last_seen_at` but channel peers do not.
   So "last heard/asked" derive from the activity log (no migration) while
   device class needs a `serde(default)` field that old stores read unchanged.

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
