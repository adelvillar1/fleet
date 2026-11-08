---
status: completed
created: 2026-11-06
updated: 2026-11-07
slug: reload-reattach
---

# Plan: a reload restores what was showing

## Context

The owner's report (2026-11-06, after the goal-task-pane plan landed):
"when the app is reloaded the goals don't load again" — the goal chip
and the tasks card come back empty after a page reload.

The code has three facts that compose into that:

1. **`#/sessions/<id>` re-attaches; `#/chat` never does.** The route's
   sessions case (`app.js:23753-23765`) calls `openWork(asked)`, which
   seeds properly: `POST /api/resume` → the log's own cards
   (`cards_from_records`, `bridge.rs:6276` — the `tasks` and
   `goal_done` cards included) → `load(true)` → the state document that
   names the shown session, carrying its `goal` and `goal_done`. But
   the chat branch (`app.js:23716-23725`) detaches only `if
   (face.session)` — and a freshly loaded page has none — so a reload
   at the chat address lands on the welcome with no conversation
   attached at all.
2. **Nothing remembers which conversation was open.** The persisted
   keys (`fleet-panel`, `fleet-rail`, `fleet-work`, …) hold surface
   choices, never the conversation id. `face.session` lives and dies
   with the page.
3. **A session-less state read names the wrong conversation.** `load()`
   (`app.js:8709-8712`) reads `/api/state` bare when `face.session` is
   empty — the bridge answers with its target session's document, so
   the boot's `face.goal` can be the *launch* session's goal: not just
   empty, but wrong.

The goal machinery itself survives every restart — the goals file, the
done store, and the log's `tasks`/`goal_done` records are all on disk.
But the owner's reload happened **while a run was in flight** (91 calls,
52,413 dropped events on the pane): `openWork`'s resume refuses for a
session whose run is in flight (the composer-actions plan's F2), and its
fallback replays the bridge's **capped in-memory event log** — whose
early `tasks` cards a long run dropped long ago (MAX_EVENTS). The card
cannot rebuild from a log that threw it away. So the fix has two legs:
the state document itself carries the session's task list (tail-scanned
from the log, exactly as `goal`/`goal_done` already ride it), and the
face re-attaches the conversation whose state would rebuild from it.

And the owner widened the ruling to a principle: **every surface that
is reloaded restores what was showing.** The second gap that hides the
goal card specifically: the pane's *width* is persisted
(`fleet-pane-width`) but its **open/closed state and active tab are
not** — a reload closes the pane, so the just-rebuilt tasks card is
invisible until the pane button is pressed again.

No mockup: there is no new surface — the chip, the card, and the pane
exist and are proven; this plan is the persistence and re-attach that
feed them.

## Direction lock

- **A reload restores what was showing, on every surface** (the face's
  own law — "a reload is the same address again" — widened by the
  owner's ruling): the chat re-opens its conversation, the pane
  re-opens at its tab, the boards re-read, the filters hold. The owner
  should not be able to tell a reload happened.
- **A deliberate detach stays a detach.** The rail's home press and
  **New** land on the welcome on purpose (N1: "the home is a place") —
  they clear the memory, so the *next* reload shows the welcome too.
- **One seed path.** The re-attach runs the exact seed `openWork`
  already runs (`/api/resume` → `load(true)`); no second rebuild
  mechanism is invented.
- **The chip never names a conversation the face is not showing.** A
  bare state read happens only on a true welcome.

## Approach

1. **The face persists the conversation it is showing** —
   `localStorage['fleet-open-session']`, written wherever
   `face.session` becomes a real conversation (`openWork`'s bind, the
   New mint's bind, dispatchTask's create-bind) and cleared by
   `detachToWelcome` (the deliberate detach). One key, one writer
   helper beside the draft helpers.
2. **The pane returns with the reload.** The pane's open/closed state
   and its active tab persist beside the width (`fleet-pane`,
   `fleet-pane-tab`), written by `openPane`/`closePane`/`showPaneTab`
   and applied at boot before the first paint — the tasks card lives
   in the pane, so a reload that closes the pane hides the very thing
   the owner is watching.
3. **The boot re-attaches.** At boot, when the address is `#/chat` (or
   bare) and a stored id exists, the route's chat branch runs the same
   re-attach the sessions case runs: `openWork(stored, {keepHash:
   true})` — the address stays `#/chat` (N1: the chat page is the
   standing place), the seed fills the transcript, the chip, and the
   card. A stored id the archive no longer holds takes `openWork`'s
   existing refusal path (a named notice, the welcome standing).
   Every other surface's reload already restores through its address
   (`#/desk/<slug>`, `#/huddles/<id>`, the boards' own reads) — the
   plan verifies that rather than reinventing it.
3. **The state document carries the list.** The bridge's state builders
   gain `tasks`: the session log tail-scanned for the last `tasks`
   record (a bounded tail read, the same shape `session_goal`/
   `session_goal_done` already ride), so the face's replay sets
   `face.tasks` from the document on every load — cap-proof against a
   long run's dropped events and restart-proof. The in-flight resume
   refusal's fallback (`load(true)` without the seed) rebuilds from
   the same document; the seeded path's live events keep it current
   after.
4. **The detach clears the memory.** `detachToWelcome` removes the key
   — the home press and New produce a welcome that *stays* a welcome
   across reloads, exactly as the owner chose it.
5. **The chip's read is already right once the re-attach lands** —
   `load(true)` names `face.session` — and the plan pins that the bare
   read happens only when no conversation is shown.
6. **Pins, gate, docs.** `tests/face/harness.mjs` gains the surface's
   cases — the stored id written on bind and cleared on detach, the
   chat boot's re-attach, the chip and card rebuilding from the state
   document and the seed, the pane's open state and tab restored at
   boot, the deliberate detach surviving a reload, the bare read only
   on a true welcome, and the refused id landing on a named welcome
   (C1–C8) — and the live pass drives the running face end to end
   (C9). The gate runs whole before any push carrying code — `cargo
   fmt --all --check`, `cargo clippy --all-targets -- -D warnings`,
   `cargo build --release`, `cargo test`, each exit code read on the
   invocation, never through a status-losing pipe (C10). The doc
   updates land with the work: gui.md's reload sentence and the
   FUNCTIONAL-SPECIFICATIONS conversation section (C11).

## Precedents this plan answers

- **`openWork` is the seed** (`app.js:12512`): the resume → load →
  replay chain the work-row open runs is the one rebuild mechanism;
  this plan reuses it, per the one-seed law (the F2 fix in the
  composer-actions plan made refusing this read strand a run).
- **N1 — the home is a place** (`app.js:23710` comment): a deliberate
  arrival detaches; this plan keeps that and adds only the *reload*
  distinction the same comment's law ("a reload is the same address
  again") already implies.
- **The goal-task-pane plan's C3** (the rebuild from the log alone):
  the rebuild chain is proven; what was never proven is a *boot* that
  runs it without the owner clicking a row first — this plan is that
  boot.
- **KI-453's detachToWelcome**: the deliberate detach is the same
  function; the stored id's clearing rides it, so the two detach
  semantics cannot drift.

## Use cases

- **UC1 — reload mid-goal.** A goal run is working; the pane shows the
  tasks card. The owner reloads the tab: the conversation returns, the
  pane re-opens at its tab with the card, the chip returns with the
  standing goal, and the run's live events keep flowing — nothing to
  click.
- **UC2 — reload after a deliberate home.** The owner pressed the
  rail's home (or New) and reloaded: the welcome stands, as chosen.
- **UC3 — the stored conversation is gone.** The session was archived
  or pruned while the tab was closed: the reload lands on the welcome
  and the face says so — never a blank surface pointing at nothing.

## UX routes

No new HTTP routes. The re-attach reuses `POST /api/resume` and the
existing `GET /api/state?session=` read. Three localStorage keys ride
beside the other `fleet-*` keys: `fleet-open-session`, `fleet-pane`,
`fleet-pane-tab`.

## Acceptance criteria

- [x] **C1 — the open conversation is remembered.** Every path that
      binds `face.session` to a real conversation writes
      `fleet-open-session`; `detachToWelcome` clears it. Pinned by
      harness cases.
- [x] **C2 — a reload at the chat address re-attaches.** Boot at
      `#/chat` with a stored id runs the resume seed and shows that
      conversation; the address stays `#/chat`. Pinned by a harness
      case.
- [x] **C3 — the goals return with it.** After the re-attach, the chip
      shows the conversation's `goal`/`goal_done` from its own state
      document and the tasks card shows the log's list — the rebuild
      chain is the seed's, no second one. Pinned by a harness case.
- [x] **C4 — the state document carries the list.** The state
      document's `tasks` field holds the session log's last `tasks`
      record (a bounded tail scan); the face's replay sets `face.tasks`
      from it; the in-flight refusal's fallback therefore rebuilds the
      card on a reload mid-run, whatever the event log dropped. Pinned
      by a bridge test and a harness case.
- [x] **C5 — the pane returns with the reload, on every surface.** The
      pane's open/closed state and its active tab persist and are
      applied at boot, so a reload mid-goal re-opens the pane with the
      tasks card showing; a reload with the pane deliberately closed
      keeps it closed. Pinned by harness cases.
- [x] **C6 — a deliberate detach stays detached across reloads.** After
      the home press or New, a reload shows the welcome. Pinned by a
      harness case.
- [x] **C7 — the chip never names an unshown conversation.** A bare
      `/api/state` read happens only when no conversation is shown; the
      shown conversation's read always names it. Pinned by a harness
      case.
- [x] **C8 — a vanished conversation is named.** A stored id the
      archive refuses lands on the welcome through `openWork`'s
      existing refusal path (the notice), never a silent blank. Pinned
      by a harness case.
- [x] **C9 — the live proof.** On the running face: a goal conversation
      open with the pane showing the tasks card, reload the tab — the
      conversation, the pane, and the card return without a click;
      then New, reload, the welcome stands with the pane closed. Quoted
      in the recap.
- [x] **C10 — the gate runs whole locally before any push carrying
      code.** The four commands of the house gate — `cargo fmt --all
      --check`, `cargo clippy --all-targets -- -D warnings`,
      `cargo build --release`, `cargo test` — each exit 0 with its exit
      code read on the invocation, never through a status-losing pipe.
- [x] **C11 — the docs.** gui.md's reload sentence (the face header's
      own law) gains the re-attach and the pane persistence;
      FUNCTIONAL-SPECIFICATIONS's conversation section names it.

## Files to be touched

- `src/gui/assets/app.js` — the stored id's writer/clearer, the pane
  state's persistence and boot restore, the chat branch's re-attach,
  the boot's read of the keys.
- `tests/face/harness.mjs` — C1–C7's cases (the storage seam exists).
- `docs/features/gui.md`, `FUNCTIONAL-SPECIFICATIONS.md` — the reload
  sentence.

## Out of scope

- **Server-side session memory.** The bridge's `attached` state is
  in-memory by design; the persistence is the face's own key. A server
  restart loses nothing the log does not rebuild.
- **The relay face's storage.** The relay seal already scopes
  localStorage per origin; no relay-specific work.
- **Multi-conversation restore.** One conversation re-opens — the one
  that was showing. Restoring a set of tabs is not a thing the face has.
- **The work list's auto-pick change.** `#/sessions` without an id
  keeps its needs-first pick (`app.js:23762-23764`) — untouched.

## Verification

```sh
# the face
node tests/face/harness.mjs

# the gate — exit codes read directly
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo test
```

C7's live pass: the running face, a goal conversation open, reload —
the chip and card return; New + reload — the welcome stands. Both
quoted in the recap.

## Linked artifacts

- `docs/features/gui.md` — the reload sentence.
- `FUNCTIONAL-SPECIFICATIONS.md` — the conversation section.
- `docs/recaps/` — the proof.

## Risks

- **The re-attach races the boot's own reads.** The chat branch's
  `readHome` and the re-attach's `load(true)` both fire at boot; the
  re-attach must not double-read or fight the home read. The seed goes
  through `openWork`, whose ordering is proven — the plan forbids a
  parallel second chain.
- **A stale stored id on a shared machine.** `localStorage` is
  per-origin; two faces on one machine share it. The id is validated by
  the resume's own refusal (C6) — the failure is named, not wrong.
- **The New-mint binds before the first send.** The stored id may name
  an empty conversation after an abandoned compose (the N2 shell) —
  a reload re-opens it, which is honest: the conversation exists. The
  archive's pruning rules are unchanged.
