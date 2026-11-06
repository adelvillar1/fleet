# Session recap — 2026-11-06: the goal's task pane

**Summary.** The owner's ask — while a goal runs, the right pane shows
the tasks and each one's completion status — landed whole:
`docs/plans/2026-11-05-goal-task-pane.md`, **13 of 13 criteria ticked**.
The find that shaped it: the `todo` tool already WAS the run's task
list (full-replacement, fold-surviving, alias-normalized) and the face
had never heard of it. The plan gave that list a log record, a live
event, and a card in the pane's run tab — and closed the `goal-done`
gap the code had been declaring without shipping.

## What was built

- **The mockup first** (`32e2086`): 48 cells (4 states × the grip's
  300/440/720 × 4 themes), every token hand-copied from `app.css`, the
  new family `gp-`-prefixed. Measured clean 48/48 — no overflow, no
  clipped text, worst ink 5.39:1, the long list's internal scroll
  proven — and read: the eye caught the done state's process card
  still showing `act` as current, fixed before the code started.
- **The record and the event.** `Record::Tasks { items }` writes beside
  the tool result on every accepted `todo` call; `UiEvent::Tasks`
  carries the same typed list live; `describe()` and
  `cards_from_records` speak one shape, so the face's reducer covers
  live and reload with one case. `TaskItem` is the log's vocabulary;
  the parse lives beside the tool's schema (`todo_items`), and a
  malformed call leaves the list standing.
- **The card.** In `#pane-run` after the process card, the mockup's
  marks verbatim: hollow = pending, ring = in progress, filled =
  completed, the goal's state word and the done share in the head.
  Hidden while the list is empty — the face never fabricates rows.
- **The goal's end.** `UiEvent::GoalDone` + `Record::GoalDone` — the
  event the code comment had been declaring since the goal tool landed.
  The emission lives at the dispatch site beside the tool result
  (because `run_work` holds no agent) and reads the tool's own output
  JSON — the shape `run_goal` builds. The semantics the harness
  caught: `complete` removes the goal from the store (a done goal is a
  fact, not an objective), so the event clears the standing goal and
  the card keeps the line it served from the completion's own text.
- **The start.** A goal set with no run in flight opens the pane on the
  run tab now, off the set reply's own `started` — a set riding an
  existing run never grabs the surface, and the owner's close still
  obeys. A goal run's prompt carries one fixed sentence naming `todo`
  as the tracker.

## The verification

- **Gate on the committed tree** (`85fa835`): `cargo fmt --all --check`
  exit 0, `cargo clippy --all-targets -- -D warnings` exit 0,
  `cargo build --release` exit 0, `cargo test` **2,077 passed / 0
  failed**, exit 0.
- **The face harness:** **933 checks / 103 cases**, exit 0 (was
  910/98). The four new GP cases pin the live paint, the empty-hide and
  full replacement, the goal-done flip with the note, the auto-open
  (started → open on run; riding → closed; the owner's close obeys),
  and the replayed-log rebuild.
- **The live proof** (scratch home `tmp/goal-pane-home`, release
  binary, port 3099): the warm-up run, then
  `POST /api/goal` → `started: true`. The SSE stream carried the
  model's own list as it evolved — frame 1: step 1 `in_progress`, 2–3
  `pending`; frame 2: 1–2 `completed`, 3 `in_progress`; frame 3: all
  `completed` — then `goal_done` with the tool's note, and
  `tmp/goal-pane-proof/notes.md` held exactly `the picker ships
  Thursday`. The model called `todo` first, unprompted beyond the
  prompt sentence. After a full server restart, `/api/resume` seeded
  **3 `tasks` cards and the `goal_done` card from the log alone** — the
  rebuild needs no store but the log. No fold occurred (three steps,
  the context stayed well under budget); the fold's carry is C8's pin
  and the fold itself was live-proven 2026-11-04. The seats the run
  asked for were answered by hand and the reason quoted: the scratch
  home's sys1 lane is unbound, so every destructive judgment fails
  closed to the standing class — correct behavior in an unbound home.
- **The scratch home now carries the owner's real `routing.json`** —
  the first goal attempt walked the compiled default chain (see
  KI-451), which is not what the owner's home walks.

## Found on the way

- **KI-451 (open)** — an account-state 400 stops the routing walk:
  `failure_class` classes every 400/404 "client-error" (not
  failover-eligible) on the premise every candidate would fail them
  identically; "you have no active step plan subscription" is one
  provider's account state, and the walk died at its first candidate
  with the deepseek tail never attempted. The same request served on
  `deepseek-flash` once pinned. The cure is one arm beside the
  existing `names_model_unavailable` heuristic.
- **KI-452 (open)** — the blend1 doctor tests are load-sensitive
  (`tests/blend1.rs:360/444` red in a full suite, green alone in
  0.85 s, the full re-run 2,077/0): the KI-385/KI-405/KI-426 class.
- **The compiled fallback table is a frozen snapshot** —
  `default_preferences()` (`src/route.rs:616-624`) still opens `hard`
  with stepfun, retired from the owner's real chain. A home *with*
  routing.json replaces chains wholesale (`slot.candidates = candidates`,
  no union), so real walks never see it; only a home without the file
  does. Refreshing the fallback is the owner's call.
- **The working tree's broken half-edit was stashed, not discarded** —
  the `Row::Shape` arm removal that made `op()` non-exhaustive, plus
  the usage plan's status flip, both stamped 12:32:33 on 2026-11-05,
  sit in `stash@{0}` with a name saying exactly what they are.
  Keep-or-revert is still the owner's.

## Where it stands, honestly

- The pane card is **implemented and proven, not yet deployed** — the
  running face at port 3080 serves the previous binary; the delivery
  pass (new inode + service restart) is the next step's opening move.
- `face-plugin-host` is one commit ahead of `master` (which itself
  fast-forwarded to `de5d643` and pushed during the session — the
  53-commit gap from the warmup is closed). Trunk sequencing stays the
  owner's.
- The scratch home, its SSE captures and the proof's raw frames remain
  under `tmp/goal-pane-home` and `tmp/goal-pane-events2.sse` — the
  evidence behind every number quoted above.
