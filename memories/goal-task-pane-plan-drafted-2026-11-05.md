---
name: goal-task-pane-plan-drafted-2026-11-05
description: "project 2026-11-05 — the goal task-pane plan COMPLETED 13/13 and WHOLE-DELIVERED: 85fa835 + f50e971 + fff6dc3 (the KI-453 New-control fix) + 1259e91 (recap) + 3bd0c08 (memories churn) pushed to face-plugin-host AND master, both refs at 3bd0c08 with a clean tree; the face relaunched on the new binary (3080, 200), the Tauri dmg rebuilt and mounted-verified carrying the new face; the todo list became Record/UiEvent::Tasks + the pane card + GoalDone, the live proof quoted the frames and the restart rebuild; the…"
type: project
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

The owner asked 2026-11-05: while a goal runs, show a running list of tasks
with each one's completion status in the right-side collapsible pane.
Plan at `docs/plans/2026-11-05-goal-task-pane.md` — **COMPLETED 13/13, status
completed**. **The decisive find:** the `todo` tool already was the run's task
list — full-list replacement, fold-surviving — but invisible to the face and
unlogged typed. The plan surfaced it; it built no tracker.

**Landed (85fa835 on face-plugin-host):** C0 mockup (32e2086, 48/48 measured
+ eye-read — the read caught the done-state process card still showing `act`
as current); `TaskItem` + `Record::Tasks`/`Record::GoalDone` (both
non-replaying); `UiEvent::Tasks`/`GoalDone` emitted at the phase-3 dispatch
site beside the ToolResult — **`run_work` is a self-less free fn, so
emissions land where `self` lives, and GoalDone reads the goal tool's own
output JSON contract** (`completed:true/goal/note` built by run_goal); bridge
describe + cards_from_records arms; TUI ignore arms; the `gp-` card in the
pane's run tab (app tokens, process-stepper marks), auto-open on the
set-goal reply's `started:true` (popover and /goal command paths), the
goal-prompt one-liner (C7), Rust pins, 4 harness GP cases via the new
`opts.goalReply` seam.

**Key semantics caught:** on `goal_done` the face CLEARS `face.goal` — the
tool removed the goal from the store (a done goal is a fact, not an
objective) and `paintGoal`'s done branch shows only when the objective is
gone; the card keeps the line via the completion's own text. **C3's rebuild
rides the existing seed path with zero new plumbing:** `attached.events` is
filled from `cards_from_records` on resume, so the log's cards flow through
apply() on any reload.

**The live proof (C10, completed 2026-11-06):** scratch home
`tmp/goal-pane-home`, release binary :3099, the owner's real routing.json
COPIED IN (the first attempt walked the compiled defaults — see
[[hard-chain-leader-walk-and-the-400-no-failover]]); mode auto rides
POST /api/run and persists in `state.permissions` (full refuses without a
working folder; the folder is set per-session via POST /api/folder — the
first goal run's todo/write calls were refused "no working folder"); goal →
`started:true`; the SSE stream carried THREE `tasks` frames evolving live
(in_progress → completed → all complete) then `goal_done` with the note;
`tmp/goal-pane-proof/notes.md` held exactly the required line; the model
called todo first unprompted beyond the C7 sentence. Seats were answered by
hand and the reason quoted: the scratch home's sys1 lane is unbound, so
every destructive judgment fails closed to the standing class. After a FULL
server restart, `/api/resume` seeded 3 tasks cards + the goal_done card from
the log alone. No fold occurred (three steps, well under budget) — **C10's
fold clause was AMENDED before ticking** (the fold carry is C8's pin; the
fold itself was live-proven 2026-11-04), per the plans' own rule.

**Delivered whole on the owner's "you can do and should do all that"** (one
turn after their go): merged to master by pushing the branch ref
(`de5d643..f50e971 face-plugin-host -> master`), `fleet-master-wt` ff'd,
release rebuilt at f50e971, new inode installed at `~/.fleet/bin/fleet`
(347553359), `launchctl kickstart -k` → new pid 50913, port 3080 → 200, the
served app.js carries the gp markers. KI-451 (account-400 no failover) +
KI-452 (blend1 doctor tests load-sensitive) filed. Gate 2,077/0, harness 933
checks / 103 cases.

**The dmg leg, closed after the owner's "continue":** `cargo tauri build`
re-ran (the cancelled run was NOT left open — see
[[rebuild-is-not-commit-push-or-ship]]) → `fleet_0.3.0-beta_aarch64.dmg`
10.90 MiB (11,433,334 B) mounted read-only, `fleet-desktop`
(25,651,424 B) carries the face (`gp-goal-text` ×3, `goal_done` ×4). A
timestamp that looked stale was the CLOCK: local is CST (UTC−6), the tower-log
ids are UTC — the whole session is local **2026-11-05**, which is why the
recap was renamed from `2026-11-06-…` to
`docs/recaps/2026-11-05-the-goals-task-pane.md` (00d69a7, pushed to both
refs; the plan's `updated:` and KI-451/452's evidence dates corrected the
same day).

**The recap write-back itself:** the owner asked "write a session recap,
commit and push" when the recap already existed but had STOPPED BEFORE THE
close-out — it said "not yet deployed … the delivery pass is the next
step's opening move", which was false by then. The lesson: a recap written
mid-session goes stale the moment the close-out lands; the same ask means
"bring the existing recap up to the actual state", not write a second one.
Also found on the way: a parallel session shares this checkout and committed
its own close-out (`edf5274`, `2026-11-05-extraction-through-sys1.md`) to
BOTH refs mid-way — different file, no collision, my push fast-forwarded on
top.

**Post-close-out continuation (same session):** the owner's New-control
regression was found, fixed and fully shipped as KI-453 —
`fff6dc3` pushed to both refs, face relaunched on the new binary
(pid 94983, 3080 → 200, `detachToWelcome` in the served shell), dmg
rebuilt and mount-verified carrying it; harness moved 933/103 →
**941/105**, gate stayed **2,077/0**. See
[[new-mints-at-the-press-ki-453]] for the ruling and the three doors.
The dmg is current at `fff6dc3`; `/Applications/fleet.app` was
byte-current at `de5d643` as of the third rebuild — its install status
after `fff6dc3` is unverified (the dock app ages independently, see
[[tauri-dmg-rebuilt-2026-11-05]]).

**The recap's final write-back (`1259e91`, pushed to both refs):** the
owner asked "write a session recap" twice — the second time after the
KI-453 fix, when the recap existed but its last section stopped before
the fix's own delivery and "Where it stands" still named `f50e971`. The
ask means **bring the existing recap up to the actual state**, not write
a second one: the KI-453 section gained its delivery paragraph (commit,
push, new inode 347645925, pid 94983, dmg 10.89 MiB with `detachToWelcome`
verified inside), and "Where it stands" was corrected to `fff6dc3`. The
session's close-of-record is
`docs/recaps/2026-11-05-the-goals-task-pane.md` — one file, three commits
deep (f50e971 → fff6dc3 → 1259e91), covering the plan, the live proof,
the close-out, and the regression fix.

**The final close-out (`3bd0c08`, pushed to both refs):** after the
recap, the only uncommitted work was the runtime's `memories/` recall
churn plus two repro scratch files the KI-453 CDP drive had left in the
repo root (`repro-hold.txt` + its `.lock`). The debris was removed and
the churn committed as `chore(memories): the corpus' recall counters` —
**both refs now at `3bd0c08`, working tree clean**, which is the state
the session closes in. The memories churn is the runtime's own recall
counters (the daily-maintenance pattern), never a session's work; a
clean-tree check is not automatic here.

**The session's last beat:** the owner sent a screenshot of the live
goal card with "The goal card worked like a charm" — the shipped surface
seen working in their own app, which is the proof the mockup and the
harness were built toward. The card's design intent (the model's own
declared steps, statuses moving on their own, nothing fabricated) is
what the screenshot shows.
