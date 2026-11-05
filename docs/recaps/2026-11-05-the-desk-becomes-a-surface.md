# Session recap — 2026-11-05: the desk becomes a surface

**Summary.** The desk shipped on 2026-11-04 as a data model with a face
that rendered wrong, and the owner's direction was to plan the repair
before developing it. `docs/plans/2026-11-05-desk-ux.md` landed whole —
**19 of 19 criteria ticked** — as three commits: the audit and its
measured mockup matrix, the surface build itself (the stylesheet
translation that never happened, the three zones, the three verbs the
face had no route for), and the live pass's one real find, KI-447. The
plan's C19 is closed on Chromium measurements against the release
binary, not on assertions about CSS.

## What was built

- **The translation, finally.** `paintDesk` emitted nine classes —
  `hsection`, `hrow-name`, `chip`, `chip-ok`, `chip-warn`, `chip-down`,
  `serif`, `mono`, `sub` — that no stylesheet ever defined, so the
  surface was unconstrained markup. The mockup's own `mockup.css` says of itself
  that it was hand-copied from `app.css` "so the translation is a
  translation and not a reinvention" — and that translation was the
  missing increment. 733 lines of `app.css` and a rewrite of the desk
  painters land it: the three zones, the state on the shelf row, the
  refusals above the review, the pinned way out.
- **The way out is reachable.** `.dk-endband` and `.dk-submitrow` are
  `position: sticky; bottom: 0` inside `.stage-board` (`overflow-y:
  auto`) rather than inside the `.stage`/`.face` pair that carries
  `overflow: hidden` — the distinction that is the whole defect, because
  sticky silently never engages when a hidden ancestor becomes the
  scrollport. The shipped end action sat at 1252px on a 700px screen.
- **Three verbs the face had no route for.** `POST
  /api/desk/<slug>/run`, `/end`, `/delete` in `server.rs` over the
  bridge, so the end button is a control rather than a label. An
  unbound desk answers 409 with its reason: *"pricing-memo is unbound —
  a desk project seeds its corpus only when a project keys it."*
- **The stage rail is five elements**, each with its own bead and its
  own label, carrying the refused stage's reason beside it — where the
  shipped rail concatenated the stages because its payload
  (`Vec<(Stage, StageState)>`) serialized as objects, not strings
  (KI-444).
- **KI-447, found by the live pass and fixed.** Every desk verb's 409
  reason was written into the live `.dk-note` node, and
  `refreshOpenDesk()` — which runs immediately after every verb —
  rebuilt the board and erased it. Measured immediately after pressing
  a refused action: `text: ""`, `height: 0`, `className: "dk-note"`.
  The reason is now state the next paint reads (`face.deskNote`, painted
  `dk-note loud`); `refreshOpenDesk` clears it only when it is not
  fresh, and `openDeskProject` clears it so a reason never rides onto
  another deliverable.

## The verification

- **Gate on the committed tree** (`038f80b`, detached worktree):
  `cargo build --all-targets` exit 0, `cargo test` **2081 passed / 0
  failed** across 16 binaries, exit 0.
- **The face harness alone:** **910 checks / 98 cases**, exit 0.
  `aRefusedVerbKeepsItsReason` is the new case (3 checks: the note
  exists, carries `unbound` after the repaint, and is `loud`), and
  reverting the two fix lines reddens exactly its two note checks and
  nothing else.
- **The live pass** (Chromium against `target/release/fleet`,
  `FLEET_HOME=tmp/desk-dive-home`): five views × 1440/1024/390 at h700,
  **no horizontal page overflow in any cell**, the gate above the review,
  `.dk-endband`'s bottom inside the viewport, every shelf card a real
  `<button type="button">`. `/api/desk` carried all four states in one
  list — `landing-copy` running, `beta-brief` refused with 2 refusals,
  `onboarding-onepager` complete, `pricing-memo` never started. The
  gate sits at top **372** of 700 with a 32px `add the figure` target
  that pre-fills the composer and lands `#/chat`. The walk is real:
  `POST /api/desk/pricing-memo/run` advanced the store through the
  stages; `POST /api/desk/onboarding-onepager/end` answered 200 on the
  clean desk. Re-measured after the KI-447 fix on the rebuilt binary:
  the note reads the full unbound sentence, class `dk-note loud`, at
  top 195.
- **Delivery:** `~/.fleet/bin/fleet` replaced through a new inode
  (19,322,000 bytes, 10:32:20) and the service restarted —
  `launchctl kickstart -k` left it running as **pid 93437, started
  10:34:41**, port 3080 answering 200, the served `app.js` carrying 9
  `deskNote` hits and 2 `dk-note loud`. The push landed
  `fb6d6ca..c387192` on `face-plugin-host`.

## Found on the way

- **KI-446 (open)** — a shared `CARGO_TARGET_DIR` silently breaks eleven
  `agent::tests` cases: the mock-server spawn falls back to a hardcoded
  `env!("CARGO_MANIFEST_DIR")/target/debug/mock-server`. The committed
  tree's gate run *without* the variable came back green (16 binaries,
  `TEST_EXIT=0`), which is what identified the variable as the cause
  rather than a real regression.
- **A harness that only ever answers 200 cannot exercise a refusal.**
  The desk verb stub answered `{ok: true}` unconditionally, so KI-447
  was invisible to 907 green checks. The seam is now
  `opts.deskVerbReply(slug, verb, documents)`; a failure path needs a
  stub that can take it.
- **A transient DOM write plus a repaint is a write the repaint eats.**
  The general shape behind KI-447: a message the owner must see belongs
  in the state the next paint reads, never in the node the paint
  replaces.
- **A python slice keyed on a repeated anchor destroyed a file's tail**
  mid-session (the harness was recovered with `git checkout --` and
  redone by anchored `.replace()` only) — the standing lesson, incurred
  again.
- **`cargo fmt` and `clippy` remain unrunnable** on this machine
  (rustup `profile minimal`, KI-007), so C18's wording names two tools
  the machine does not carry and the plan records that rather than
  claiming them.

## Where it stands, honestly

- The desk is **deployed and serving**: the desk work that shipped on
  2026-11-04 to a stale binary is now in `~/.fleet/bin/fleet` and live.
- `pricing-memo` on the scratch home is deliberately unbound — it is the
  refusal case the live pass measures, not a broken desk.
- The branch `face-plugin-host` is **46 commits ahead of `master`**, and
  local `master` stands 8 commits ahead of `origin/master` (another
  session's context-health work). Trunk is the owner's call to sequence;
  nothing here is blocked on it.
- `.path.mono` at 390px ellipsizes by design (`overflow-x: hidden` +
  `text-overflow: ellipsis`), not by clipping.
