# Session recap — 2026-11-06: a reload restores what was showing

**Summary.** The owner's follow-up to the goal-task-pane session: "when
the app is reloaded the goals don't load again." The plan
(`docs/plans/2026-11-06-reload-reattach.md`, **11 of 11 criteria ticked**)
landed on its own branch (`reload-reattach`, worktree
`~/Projects/fleet-reload-reattach`) — the KI burn-down session is working
in parallel, so nothing touched `master` or their worktrees.

## The defect, as the code told it

Three facts composed: the work-row open seeded properly (`/api/resume` →
the log's cards → the state read), but the chat address never
re-attached (the boot's chat branch skips when `face.session` is empty —
a fresh page always is); nothing persisted the open conversation; and
the owner's actual reload happened **mid-run** — the resume refuses for
a session whose run is in flight, and the fallback replays the bridge's
**capped in-memory event log**, whose early `tasks` cards a 497-checkpoint
run had dropped long ago. The card could not rebuild from a log that
threw it away.

## What was built

- **The state document carries the list** (the plan's C4): the bridge's
  state builders gained `tasks` — the log tail-scanned (2 MiB window,
  lossy at the seam) for the last `tasks` record, the same shape
  `goal`/`goal_done` already ride. Every load rebuilds the card from
  the document; the live events keep it current after.
- **The conversation is remembered** (C1): `fleet-open-session` written
  wherever a real conversation binds (`openWork`, the New mint,
  dispatchTask's create-bind), cleared by `detachToWelcome` and by the
  home walk's own detach.
- **The chat address re-attaches** (C2): the boot's chat branch runs
  `openWork(stored, {keepHash: true})` — the same seed, the address
  stays `#/chat`. A deliberate detach cleared the key, so the chosen
  welcome stays chosen (C6).
- **The pane returns** (C5): `fleet-pane` and `fleet-pane-tab` persist
  beside the width, applied at boot before the first paint — a reload
  mid-goal re-opens the pane with the card showing.
- **The chip never lies** (C7): the bare state read happens only on a
  true welcome; the shown conversation's read names it.
- **Two clippy nits in the burn-down's `reload.rs`** — collapsible ifs
  from the co-tenant's C6, which rides `face-plugin-host` and thus this
  branch's base — fixed mechanically (a match guard, one collapsed
  condition) so the branch's gate runs green. Their own worktree copy
  is untouched; noted here so the merge is not a surprise.

## The verification

- **Gate on the branch** (`935d148`): fmt 0, clippy 0, release build 0,
  `cargo test` **2,124 passed / 0 failed** (the branch carries the
  burn-down's tests beside mine).
- **The harness:** **962 checks / 110 cases** — the five RE cases pin
  the memory (bind/clear), the chat re-attach (the resume names the
  stored conversation, the address stays `#/chat`), the chip and card
  rebuilding from the state document, the pane's open state and tab,
  the deliberate detach surviving, and the refused id's named welcome.
- **The bridge pin:** `session_tasks_reads_the_last_tasks_record_from_the_log_tail`
  — the last record wins, an interposing record does not confuse the
  scan, a conversation with no tasks reads as empty.
- **The live proof** (C9, the owner's real face at 3080, their own
  burn-down session `20261106T033200Z-14f5a714` — 497 checkpoints): the
  chip read `implement plan as designed`, the card held **6 rows**; the
  pane opened; **the reload** — the conversation returned, the chip
  returned, all **6 rows** rebuilt (rows the capped event log had long
  dropped — the state-document leg proving itself in the wild), the
  pane **open at the run tab**. Then New + reload: the welcome stands.
  One wrinkle named, not fixed: the welcome's chip shows the launch
  session's standing goal — the bare read on a true welcome, the
  face's pre-existing standing-context behavior, which C7 permits.
- **The delivery:** the branch binary installed through a new inode
  (351456307), the service restarted (port 3080 answering 200, the
  served `app.js` carrying `storeOpenSession`), and the dmg rebuilt
  with the fix verified inside.

## Found on the way

- **The drive hit 100% again** — `~/.fleet/checkpoints` at **69 G**
  (the KI burn-down's long runs snapshot per turn; the prevention is
  still the open item). The owner-authorized procedure re-ran: 821 turn
  dirs pruned to the last 20 per session, **55 GiB freed**. The
  retention leg in daily-maintenance remains the real fix.
- **The plan-gate's classifier** flagged the renumbered C10/C11 after
  the state-doc criterion inserted — the body commits to both by name;
  dispositioned (the proven command-led shape).

## Where it stands, honestly

- The work is on **`reload-reattach`** (origin, `935d148` + the docs
  commit) — `master` and the burn-down's refs untouched; the merge is
  the owner's call when the burn-down lands.
- The face at 3080 and the dmg run the branch's binary — the fix is
  live before the merge, by the owner's own report of the bug being
  live.
- KI-451, KI-452, and the checkpoint-retention leg remain open.
