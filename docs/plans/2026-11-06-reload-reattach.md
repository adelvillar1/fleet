---
status: draft
created: 2026-11-06
updated: 2026-11-06
slug: reload-reattach
---

# Plan: a reload re-opens the conversation it held

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
The defect is purely that the reloaded face never re-attaches the
conversation whose state would rebuild from them.

No mockup: there is no new surface — the chip and the card exist and
are proven; this plan is the re-attach that feeds them.

## Direction lock

- **A reload is the same address again** (the face's own law,
  `app.js` header): a page reloaded at `#/chat` re-opens the
  conversation it held. The owner should not be able to tell a reload
  happened.
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
2. **The boot re-attaches.** At boot, when the address is `#/chat` (or
   bare) and a stored id exists, the route's chat branch runs the same
   re-attach the sessions case runs: `openWork(stored, {keepHash:
   true})` — the address stays `#/chat` (N1: the chat page is the
   standing place), the seed fills the transcript, the chip, and the
   card. A stored id the archive no longer holds takes `openWork`'s
   existing refusal path (a named notice, the welcome standing).
3. **The detach clears the memory.** `detachToWelcome` removes the key
   — the home press and New produce a welcome that *stays* a welcome
   across reloads, exactly as the owner chose it.
4. **The chip's read is already right once the re-attach lands** —
   `load(true)` names `face.session` — and the plan pins that the bare
   read happens only when no conversation is shown.
5. **Pins, gate, docs.** `tests/face/harness.mjs` gains the surface's
   cases — the stored id written on bind and cleared on detach, the
   chat boot's re-attach, the chip and card rebuilding from the seed,
   the deliberate detach surviving a reload, the bare read only on a
   true welcome, and the refused id landing on a named welcome (C1–C6)
   — and the live pass drives the running face end to end (C7). The
   gate runs whole before any push carrying code — `cargo fmt --all
   --check`, `cargo clippy --all-targets -- -D warnings`,
   `cargo build --release`, `cargo test`, each exit code read on the
   invocation, never through a status-losing pipe (C8). The doc updates
   land with the work: gui.md's reload sentence and the
   FUNCTIONAL-SPECIFICATIONS conversation section (C9).

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

- **UC1 — reload mid-goal.** A goal run is working; the owner reloads
  the tab. The chip returns with the standing goal, the tasks card
  returns with its list, and the run's live events keep flowing —
  nothing to click.
- **UC2 — reload after a deliberate home.** The owner pressed the
  rail's home (or New) and reloaded: the welcome stands, as chosen.
- **UC3 — the stored conversation is gone.** The session was archived
  or pruned while the tab was closed: the reload lands on the welcome
  and the face says so — never a blank surface pointing at nothing.

## UX routes

No new HTTP routes. The re-attach reuses `POST /api/resume` and the
existing `GET /api/state?session=` read. One localStorage key
(`fleet-open-session`) rides beside the other `fleet-*` keys.

## Acceptance criteria

- [ ] **C1 — the open conversation is remembered.** Every path that
      binds `face.session` to a real conversation writes
      `fleet-open-session`; `detachToWelcome` clears it. Pinned by
      harness cases.
- [ ] **C2 — a reload at the chat address re-attaches.** Boot at
      `#/chat` with a stored id runs the resume seed and shows that
      conversation; the address stays `#/chat`. Pinned by a harness
      case.
- [ ] **C3 — the goals return with it.** After the re-attach, the chip
      shows the conversation's `goal`/`goal_done` from its own state
      document and the tasks card shows the log's list — the rebuild
      chain is the seed's, no second one. Pinned by a harness case.
- [ ] **C4 — a deliberate detach stays detached across reloads.** After
      the home press or New, a reload shows the welcome. Pinned by a
      harness case.
- [ ] **C5 — the chip never names an unshown conversation.** A bare
      `/api/state` read happens only when no conversation is shown; the
      shown conversation's read always names it. Pinned by a harness
      case.
- [ ] **C6 — a vanished conversation is named.** A stored id the
      archive refuses lands on the welcome through `openWork`'s
      existing refusal path (the notice), never a silent blank. Pinned
      by a harness case.
- [ ] **C7 — the live proof.** On the running face: a goal conversation
      open, reload the tab, the chip and card return without a click;
      then New, reload, the welcome stands. Quoted in the recap.
- [ ] **C8 — the gate.** fmt, clippy, release build, test — exit codes
      read directly.
- [ ] **C9 — the docs.** gui.md's reload sentence (the face header's
      own law) gains the re-attach; FUNCTIONAL-SPECIFICATIONS's
      conversation section names it.

## Files to be touched

- `src/gui/assets/app.js` — the stored id's writer/clearer, the chat
  branch's re-attach, the boot's read of the key.
- `tests/face/harness.mjs` — C1–C6's cases (the storage seam exists).
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
