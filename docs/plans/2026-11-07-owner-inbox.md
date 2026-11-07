---
status: draft
created: 2026-11-07
updated: 2026-11-07
slug: owner-inbox
---

# Plan: the owner's inbox — one queue of everything waiting on you

## Context

The owner's ask (2026-11-07): an inbox for the user's pending actions
and approvals — a queue of what the owner needs to do — including
onboarding actions (the first LLM provider key, the sys1 key for sys1)
and, by the mid-draft steer, **questions coming from the different
huddles**.

The substrates all exist; nothing unifies them:

- **Seats.** The approval gate's pending call renders as one card on
  the showing conversation's transcript — invisible from every other
  surface. The archive already derives a per-session `needs` flag
  (`state == "needs-approval"`, `bridge.rs:7204`) and the work list
  groups by it ("needs you", `app.js:12274`), but answering still
  requires opening that conversation. The answerer is cross-session
  already: `bridge.answer(&session, &decision, &by)`
  (`bridge.rs:4884`) takes the session the seat belongs to.
- **Onboarding.** `/api/onboard` carries the ladder — `steps[]` with
  `done` per step and `first_incomplete` naming the current one
  (`app.js:4541`'s reader, the door's one-decision screen). The steps
  are exactly the owner's ask: the provider key, the sys1 key, the
  home's own setup.
- **Huddle questions.** `Record::Mention { member, kind, from }`
  (`session.rs:444`) is the typed routing fact: `member` is
  `MENTION_OWNER` when a room asks the owner ("which asks and never
  starts a run"), `kind` is `DIRECTIVE`/`ASK`/`REVIEW`. The mention
  rides a message's own content; a mention is *unanswered* until an
  owner `User` record follows it in that room's log. Nothing today
  collects them across rooms — the ask lives and dies in the room's
  transcript.

No mockup-less shortcut: **the inbox is a new surface**, so the house
mockups rule applies — drawn, measured, read, and owner-read before
code.

Decision-model check: the inbox's assembly is structural record-reading
(a seat waiting, a step incomplete, a mention unanswered) — no per-item
judgment, no scoring, no extraction. Said so rather than left unnamed:
no sys1 job here.

## Direction lock

- **One surface, one read.** `#/inbox` on the rail, fed by one bounded
  document (`GET /api/inbox`) assembled from the reads that already
  exist — the home's own law: the face never walks every log on every
  look.
- **The inbox is a view, not a store.** No new persistence; an item
  exists while its condition does (the seat unanswered, the step
  incomplete, the ask unanswered) and leaves when it resolves. No
  dismissal, no snooze — resolving is the only exit.
- **Rows act in place where the machinery allows.** Seats answer
  inline (the answerer is cross-session); onboarding rows carry their
  own action; room asks jump to the room with the composer focused —
  the answer is the owner's words in the room.
- **The badge is the queue.** The rail row carries the live count; the
  count recomputes from the same document, so the badge cannot disagree
  with the page.

## Approach

0. **The mockup first.** `docs/plans/mockups/2026-11-07-owner-inbox/`
   holds the surface as static HTML in the app's own tokens — the three
   row classes with their actions, the empty state, the rail badge —
   measured rendered in every theme (no overflow, no clipped text, ink
   ≥ 4.5:1), the PNGs read in a light and a dark theme, and the owner
   reads it before any code of C1+ lands (C0).
1. **The read.** `GET /api/inbox` assembles one document:
   - `seats`: the pending seat(s) — the driving session's live seat
     and any `needs-approval` session from the archive's own groups —
     each with the session, the tool, the label, the summary, and the
     seat's `since`.
   - `onboarding`: the incomplete steps of `/api/onboard`'s ladder,
     each with its id, its narration, and its action's route.
   - `asks`: per room, the unanswered `MENTION_OWNER` mentions —
     `kind: ASK` (and `REVIEW`; the owner trims) — each with the room,
     who asked, the kind, the message's own words, and the mention's
     age. Unanswered means no owner `User` record follows the mention
     in that room's log; the scan is the bounded tail read, the same
     honesty the goals' tail scan carries.
2. **The surface.** `#/inbox`: one list, three row classes, each row
   saying what / where / why / since, with its action riding the row —
   the seat's once/always/deny, the step's own control, the ask's jump.
   Empty state: "nothing needs you" — an honest absence, never a
   fabricated row.
3. **The badge.** The rail row's badge is the document's total,
   recomputed on every read and on the live events the face already
   receives (`seat`, `seat_cleared`, `mention`, `answered`) — the same
   events, one more reducer.
4. **Pins, gate, docs.** `tests/face/harness.mjs` gains the surface's
   cases — the read's three classes rendered, the seat answered in
   place, the onboarding rows with their controls, the ask's jump, the
   badge's live total, and the honest empty state (C1–C6) — and the
   live pass drives the running face end to end (C7). The gate runs
   whole before any push carrying code — `cargo fmt --all --check`,
   `cargo clippy --all-targets -- -D warnings`, `cargo build --release`,
   `cargo test`, each exit code read on the invocation, never through a
   status-losing pipe (C8). The doc updates land with the work:
   gui.md's inbox surface, FUNCTIONAL-SPECIFICATIONS's navigation
   section, and the rail's icon sheet (C9).

## Precedents this plan answers

- **The work list's "needs you"** (`app.js:12274`): the archive's
  `needs` flag is the seat's cross-session shadow — the inbox is where
  it becomes answerable.
- **The composer-actions plan's F1**: a stop ends the approval wait and
  names it — the inbox reads the same seat state, so an answered or
  stopped seat leaves the queue without a stale row.
- **The huddles plan's C3/C8**: the mention row is "who was asked and
  what for, not a detail to bury in prose" — the inbox is that row,
  collected across rooms.
- **The rail badge (N3)**: "the rail's badge says where the work is —
  needs plus working" — the inbox joins the badge economy with its own
  count.
- **KI-431's named degradation**: an onboarding row exists because an
  absence (no key) is *named* — the inbox is where the naming collects.

## Use cases

- **UC1 — three rooms ask at once.** Two huddles pose questions and a
  third requests a review while a run in another conversation waits on
  a `write_file` seat. The rail's badge says 4; the inbox lists all
  four; the seat is answered in place; the questions are answered by
  jumping — each jump lands with the composer focused.
- **UC2 — the fresh install.** A new home's inbox lists the onboarding
  ladder's incomplete steps in order; each row's action is the door's
  own decision; the rows leave as the steps complete.
- **UC3 — the quiet morning.** Nothing waits: the badge is gone, the
  surface says "nothing needs you", and no row pretends otherwise.

## UX routes

One new read: `GET /api/inbox`. Seat answers reuse the existing
`POST /api/approve` (it already takes the session); onboarding actions
reuse `POST /api/onboard`; the ask's jump is the room's own address.
The rail row and the badge ride the existing rail markup.

## Acceptance criteria

- [ ] **C0 — the mockup is drawn, measured, read, and owner-read.**
      `docs/plans/mockups/2026-11-07-owner-inbox/` holds the surface in
      the app's own tokens (the three row classes, the empty state, the
      badge), measured in every theme (no overflow, ink ≥ 4.5:1), the
      PNGs read in a light and a dark theme, and the owner reads it
      before any code of C1+ lands.
- [ ] **C1 — the read is one bounded document.** `GET /api/inbox`
      returns seats, incomplete onboarding steps, and unanswered room
      asks in one response, assembled from the existing reads; a room
      ask carries the room, the asker, the kind, the message's words,
      and its age. Pinned by bridge tests.
- [ ] **C2 — seats answer in place.** A seat row's once/always/deny
      calls the existing answer seam with the seat's own session; the
      row leaves when the seat resolves. Pinned by a bridge test and a
      harness case.
- [ ] **C3 — onboarding rows carry their actions.** Each incomplete
      step renders with its own control (the door's decision or its
      route); a completed step never renders. Pinned by a harness case.
- [ ] **C4 — huddle asks are collected and answered.** Every room's
      unanswered owner-mention renders with its words and its age; the
      jump opens the room with the composer focused; an owner reply in
      the room retires the row. Pinned by a bridge test and a harness
      case.
- [ ] **C5 — the surface and the badge.** `#/inbox` mounts on the rail
      with the badge showing the live total; the empty state is the
      honest absence. Pinned by harness cases.
- [ ] **C6 — the live events keep it true.** A seat, an ask, or a
      resolution arriving on the wire updates the queue and the badge
      without a reload. Pinned by a harness case.
- [ ] **C7 — the live proof.** On the running face: a seat pending in
      one conversation and a question pending in a room, reload the
      tab — the inbox returns with both; answer the seat in place and
      the question by jumping; both rows leave. Quoted in the recap.
- [ ] **C8 — the gate.** fmt, clippy, release build, test — exit codes
      read directly, the whole suite.
- [ ] **C9 — the docs.** gui.md gains the inbox surface (its read, its
      rows, its badge); FUNCTIONAL-SPECIFICATIONS's navigation section
      names it; the rail's icon sheet gains the inbox's mark.

## Files to be touched

- `src/gui/bridge.rs` — the inbox read (seats, onboarding steps, the
  room-ask tail scan) and its assembly.
- `src/gui/server.rs` — the `GET /api/inbox` route.
- `src/gui/assets/index.html`, `app.css`, `app.js` — the surface, the
  rail row, the badge, the reducer's new case.
- `tests/face/harness.mjs`, `src/gui/bridge.rs` tests — the pins.
- `docs/plans/mockups/2026-11-07-owner-inbox/` — the mockup.
- `docs/features/gui.md`, `FUNCTIONAL-SPECIFICATIONS.md` — the surface.

## Out of scope

- **Cron failures and doctor findings as items.** The taxonomy v1 is
  the owner's three: seats, onboarding, huddle questions. Doctor rows
  and cron misses are candidates for a v2 — named here so the omission
  is a decision.
- **Snooze, dismissal, priority ordering.** The queue is ordered by
  age within its classes; resolving is the only exit.
- **Answering a room from inside the inbox.** The jump is the answer's
  door (the owner's words in the room, the transcript's own context) —
  an inline reply box is a v2 candidate.
- **The bots' own inbox.** The owner's queue only; what the bots owe
  each other stays in the room.

## Verification

```sh
# the mockup: measured in every theme, the PNGs read
# the face
node tests/face/harness.mjs

# the gate — exit codes read directly
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo test
```

C7's live pass: the running face, a seat and a question pending, reload
— both return; answered in place and by jumping — both leave. Quoted in
the recap.

## Linked artifacts

- `docs/features/gui.md` — the inbox surface.
- `FUNCTIONAL-SPECIFICATIONS.md` — the navigation section.
- `docs/architecture/surfaces.md` — the rail's inventory.
- `docs/recaps/` — the proof.

## Risks

- **The room-ask scan walks every room's log tail.** Bounded per room
  (the tail window), and the room count is small — but the read is one
  document assembled from several reads; the home's caching law applies
  if it shows up slow.
- **The unanswered rule's edge.** A room where the owner answered in
  words that never landed as a `User` record (an interrupted send)
  reads as unanswered — the honest reading of the log's own truth; the
  row leaves when the record lands.
- **The badge's honesty.** The count and the list come from one
  document, but the live events land between reads — the reducer
  recomputes from the same document shape, so a stale badge resolves on
  the next read, never disagrees mid-frame.
- **House CSS collisions.** The mockup and the surface prefix their
  classes (`ib-`) rather than reusing the owned `.pane`/`.card`/`.rail`
  names.
