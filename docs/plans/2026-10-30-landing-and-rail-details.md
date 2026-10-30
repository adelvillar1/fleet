---
status: active
created: 2026-10-30
updated: 2026-10-30
slug: landing-and-rail-details
---

# Plan: the rail's collapse at the top, the instrument that fits, and the landing that greets the owner's work

## Context

Three owner-reported frictions from the rail revamps' first real use
(2026-10-30), each diagnosed on the live face:

- **The left rail's collapse lives in the foot.** `#rail-toggle`
  ("collapse the rail") sits at the bottom of the surfaces rail, below
  "jump to anything" and "this machine" — a thumb-reach position on a
  page the owner reads from the top. The right pane's expand icon sits
  in the topbar row. The owner's ruling: the collapse belongs **at the
  top, level with the pane's expand icon, on the right side of the left
  rail** — one row of chrome, both rails' hands at the top.
- **"The stream" card's text overflows the buttons.** The calibration
  instrument carries three actions (the day before, the day after, run
  a gate review); at the face's width the buttons wrap mid-label ("the
  day / before") and the second line clips past the card's bottom edge
  — the screenshot shows it. The instrument's actions row does not
  wrap; the card does not grow.
- **The landing shows the machine's sessions, not the owner's.** The
  recent list ("pick up where you left off") renders bare session ids
  of **unclassed runs** — `owner_recent` passes `class == None`, the
  exact hole KI-372 closed on the work rail (the ruling: the owner's
  screen carries `class == "yours"` and nothing else). Scheduled
  corpus-refresh runs, subagent sessions, and shells are invisible on
  the rail and visible on the landing. And the landing's actions no
  longer match the day: the queues the rail revamps built (skills
  proposals awaiting, the memory cycle's queue, calibration's gate)
  don't surface where the owner lands.

The owner's direction: fix the two, rethink the landing — *"the landing
page needs to be rethought as I don't think it's showing relevant
actions any more"*.

## Decisions (the owner's, dated and quoted)

1. **2026-10-30 — the collapse's place.** *"the left rail collapse
   button should be at the top at the same level of the right side pane
   expand icon, but on the right side of the left menu rail."*
2. **2026-10-30 — the overflow.** *"on the calibration page on 'the
   stream' card, the text is overflowing the buttons."*
3. **2026-10-30 — the landing.** *"the landing page needs to be
   rethought as I don't think it's showing relevant actions any more,
   and the list of recent sessions seems to be showing background app
   sessions which should be invisible to the user."*

## Approach

**The collapse moves to the rail's head.** `#rail-toggle` leaves
`.surfaces-foot` for a new head row: the rail's brand block (the fleet
mark) gains a right-aligned toggle on the same line, whose top aligns
with the topbar row the pane's expand icon lives in. Same button, same
handler, same persisted choice (`fleet-rail-mode`); the foot keeps
"this machine". In icons mode the head keeps the mark and the toggle's
chevron flips — the collapsed rail's top stays one row tall.

**The instrument grows to fit its actions.** The actions container
wraps (flex-wrap with a row gap) and the card's height follows its
content — no clipped second line, at any width. Verified by measurement
at 1440 and ~1100: every button whole, inside the card.

**The landing greets the owner's work.** Two halves:

- **The leak, closed first.** `owner_recent` becomes exactly
  `class == "yours"` — the `None` arm dies, KI-372's rule applied at
  the third surface that reads it. The recent rows then carry the
  owner's conversations' titles, not bare ids of machine runs.
- **The rethink, proposed.** The landing's greet becomes **what needs
  you** and **what is waiting**: needs-you sessions first (approval
  seats, paused runs — one row each, straight to the conversation),
  then the queues the rail revamps made legible (skills proposals
  awaiting an approve, the memory cycle's queue when non-empty,
  calibration's fit cells short of fit) — each a row with its count and
  its surface address — then "pick up where you left off" (the owner's
  recent conversations, titled), then the counts strip as it stands.
  The capability cards stay on demand. The machine's activity timeline
  keeps every run — it is a count, not a list. The composition is the
  owner's to steer at review; the plan builds the shape and the
  harness pins the grammar.

High-level steps:

1. **N0 — the leak.** `owner_recent`'s predicate; the recent rows'
   titles.
2. **N1 — the collapse at the top.** The head row, the CSS, the
   collapsed mode.
3. **N2 — the instrument fits.** The wrap, the measurement.
4. **N3 — the landing's greet.** The needs-you rows, the queue rows,
   the recent list, the harness case, the measurement, the docs.

## Increments

Each lands whole — code, checks, docs — and each is green on its own.

### N0 — the owner's recent, exactly

`owner_recent`'s filter becomes `class == Some("yours")` — the
`None |` arm dies with a comment naming KI-372's ruling and the third
surface. The rows the landing then shows carry the sessions' own
labels; the unit pin: a fixture with a scheduled row, a subagent row,
an unclassed row, and an owner row answers exactly the owner row.

*Checks:* L1.

### N1 — the collapse at the top

`index.html` moves the toggle into the rail's head beside the brand
mark (right-aligned); the CSS aligns the head row's top with the
topbar's, keeps the icons-mode head one row tall, and the foot keeps
the where-line. The handler and the persisted choice are untouched.

*Checks:* L2, L3.

### N2 — the instrument fits its actions

`.surface-instrument`'s actions wrap (flex-wrap, row-gap) and the card
grows with the content; the calibration page measured at 1440 and
~1100 shows every button whole and inside the card. The same fix
serves every surface's instrument — the overflow was calibration's
three buttons, the fix is the shared card's.

*Checks:* L4.

### N3 — the landing's greet

The landing gains **what needs you** (the owner's needs-approval and
paused sessions, one row each, address straight to the conversation)
and **what is waiting** (skills proposals, the memory cycle's queue,
calibration's short cells — each with its count and surface address,
rendered only when non-empty); "pick up where you left off" keeps the
owner's recent conversations beneath them; the counts strip stays. A
harness case over a home fixture with a needs-you session, waiting
queues, and an unclassed run pins the grammar and the absence.

*Checks:* L5, L6, L7.

## Use cases

- [ ] A: The owner collapses the left rail from its top, beside the brand mark, level with the pane's expand icon.
- [ ] B: The calibration instrument's three actions are whole at any width.
- [ ] C: The landing's recent list carries only the owner's conversations, titled.
- [ ] D: The landing greets with what needs you and what is waiting, not the machine's run ids.

## Acceptance criteria

- [x] **L1 — the recent list is the owner's, exactly.** `owner_recent`
      passes only `class == "yours"`; the unit pin answers one owner row
      from a fixture carrying a scheduled, a delegated, an unclassed,
      and an owner row; the landing's recent rows render session labels,
      not bare ids.
- [x] **L2 — the collapse is at the top.** The toggle sits on the rail
      head's row, right-aligned, its top level with the topbar; the
      handler, the aria state, and the persisted choice are unchanged;
      the foot keeps the where-line.
- [x] **L3 — the collapsed mode keeps the head.** In icons mode the
      rail's top stays the mark and the toggle (chevron flipped), one
      row tall, and expanding returns the full head.
- [x] **L4 — the instrument fits.** At 1440 and ~1100 the calibration
      instrument's three buttons are whole, inside the card's bounds;
      the actions wrap with a row gap and the card grows; the fix is
      the shared card's.
- [x] **L5 — the landing greets what needs you.** A needs-you owner
      session renders as a row (its state named, its address the
      conversation); a paused run likewise; the rows come only from
      `class == "yours"`.
- [x] **L6 — the landing shows what is waiting.** Skills proposals, the
      memory cycle's queue, and calibration's short-of-fit cells render
      as rows with their counts and surface addresses, only when
      non-empty; an empty queue renders nothing.
- [x] **L7 — the grammar is pinned.** A harness case over a home
      fixture: the needs-you row, the waiting rows, the titled recent
      rows, and the unclassed run's absence — each asserted against its
      named element, in the same commit as the element.
- [x] **L8 — the gate and the measurement.** fmt, clippy, build, test
      green by exit code; the probe quotes the before/after (the
      instrument's clipped buttons, the bare ids) with the screenshots
      read as the review.

## Landed 2026-10-30

L1: `the_owner_recent_answers_the_owners_rows_only` — one owner pair
from a fixture carrying scheduled, delegated, kind-less, and probe
rows; and the build's own catch: the three recent rows on the live face
were `class: yours` with **empty titles** — the owner's own runs
reading as machine noise — so an untitled conversation now reads as
"a conversation · <when>", never a bare id fragment, and the waiting
count filters by the queue's own statuses (the greet said 6 proposals
when the queue holds 3). L2–L3: the toggle in the rail head
(`surfaces-head`), its center measured at **28.5px — exactly the pane
expand icon's** — with the head at half the topbar's height and the
icons mode keeping the mark centered, chevron floating on the edge. L4:
the buttons wrapped their labels *inside* the fixed-height buttons
(flex-shrink squeezed three past their labels' width; the 28px height
clipped the second line) — whole buttons wrap, row-gapped, and the
card grows: whole at 1440 and 1100. L5–L7: the face harness, **472
checks across 50 cases** — the LAND case pins the greet's rows (the
seat run named, the queues with their counts, the empty queue absent),
the addresses, and the titled recent list. L8: the gate whole (fmt 0,
clippy 0, build 0, the suite green — quoted in the commit) and the
probe's shots read as the review (tmp/n3-landing.png: the greet, the
collapse at the head, the counts).

## Files to be touched

**Backend / API:**
- `src/gui/bridge.rs` — `owner_recent`'s predicate + the unit pin; the home document's waiting-queue counts (skills proposals, memory preview, calibration cells short of fit)

**Frontend:**
- `src/gui/assets/index.html` — the toggle's move to the rail head
- `src/gui/assets/app.js` — the landing's greet (needs-you rows, waiting rows), the recent list's labels
- `src/gui/assets/app.css` — the head row's alignment, the icons-mode head, the instrument's wrap

**Tests:**
- `tests/face/harness.mjs` — the landing case
- `src/gui/bridge.rs` tests — the `owner_recent` pin

**Documentation (see Linked artifacts below):**
- `docs/features/gui.md` — the landing's greet and the rail head
- `FUNCTIONAL-SPECIFICATIONS.md` — the landing passage

## Out of scope

- **The work rail's grouping** — unchanged; the landing reads its
  document, it does not re-group it.
- **A configurable landing** — the composition is fixed; the owner
  steers at review.
- **The machine's activity timeline** — keeps every run; it is a count.
- **The palette (⌘K)** and the jump behavior — unchanged.

## Verification

**L1** — the predicate pin:
```bash
cargo test the_owner_recent_answers_the_owners_rows_only
```
**L4–L8** — the face harness + the probe:
```bash
node tests/face/harness.mjs
cargo fmt --all --check && cargo clippy --all-targets -- -D warnings \
  && cargo build --release && cargo test
```
Expected: every new check green; the probe quotes the instrument whole
at two widths and the landing's rows titled; the screenshots are read
as the review.

## Linked artifacts

- `docs/features/gui.md` — the landing's greet and the rail head
- `FUNCTIONAL-SPECIFICATIONS.md` section "What Fleet tells you when" — the landing's rows

## Risks

- **The landing's queue rows read three surfaces' documents.** The home
  read already carries memory and skills; calibration's cells ride the
  home read too or fall back to the panel reads — a queue that has not
  landed renders nothing rather than a stale count.
- **Needs-you rows double with the work rail's own.** They are the same
  rows on purpose — the landing is the door, the rail is the desk; the
  address is one click either way.

## Dependencies

- The KI-372 ruling (`4753d13`) — the class predicate this plan applies
  at the landing.
- The four rail revamps — the queues the greet surfaces.

## Notes

- The before-states were measured on the face as it serves: the recent
  list showing three bare session ids (`20261029T2351…`) — unclassed
  runs; the stream instrument's buttons wrapping mid-label and clipping
  past the card's bottom edge; the collapse at the rail's foot.
- `owner_recent`'s `None` arm predates the class stamping's
  completeness — kind-less rows read as the owner's before the probe
  stamp and the scheduled kinds existed; the arm is a fossil of that
  era, and KI-372's rule is its retirement.
