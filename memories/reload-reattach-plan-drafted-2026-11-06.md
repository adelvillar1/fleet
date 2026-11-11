---
name: reload-reattach-plan-drafted-2026-11-06
description: "project 2026-11-06 — the reload-reattach plan IMPLEMENTED 11/11 on branch `reload-reattach` (worktree ~/Projects/fleet-reload-reattach) and MERGED to master by the owner-approved fast-forward a1502af..a3a5437 (the burn-down's history rides the branch's base, zero conflicts, pushed); the decisive amendment: the owner's reload happened MID-RUN — the resume refuses in flight and the fallback replays the CAPPED in-memory event log (52,413 dropped) — so the state document itself carries the task list (`session_t…"
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

The owner reported 2026-11-06, right after confirming the goal card works
([[goal-task-pane-plan-drafted-2026-11-05]]): "when the app is reloaded the
goals don't load again" — steered "plan the fix for now", then GO via the
goal: "implement plans you just created but on a separate branch because the
KI burn down is still ongoing."

**The defect's code facts** (all read, line refs in the plan): the work-row
open seeds properly (`openWork` → `POST /api/resume` → `cards_from_records`
→ `load(true)`); `#/chat` never re-attaches (its detach block guards on
`face.session`, empty on a fresh page); a bare `/api/state` read names the
LAUNCH session's goal (wrong, not just missing); nothing persisted the open
conversation (the `fleet-*` keys hold surface choices, never the session
id); and the pane's width was persisted but its open/closed state and active
tab were not — a reload closed the pane over the just-rebuilt card. The
owner widened the ruling: **a reload restores what was showing, on every
surface.**

**The decisive amendment before implementing:** the owner's reload happened
MID-RUN (91 calls, 52,413 dropped events, 497 checkpoints on their own
burn-down session). `openWork`'s resume REFUSES for a session whose run is
in flight (the composer-actions F2 semantics), and the fallback replays the
bridge's **capped in-memory event log** — whose early `tasks` cards a long
run had dropped long ago. The card could not rebuild from a log that threw
it away. So the plan gained its fourth leg (→ 11 criteria, C4 inserted and
the rest renumbered): **the state document itself carries the list.**
`session_tasks()` (`bridge.rs`, beside `session_goal_done`) tail-scans the
session log — 2 MiB window, `from_utf8_lossy` at the seam, last
`{"type":"tasks"}` record wins (the tool's own full-replacement contract) —
and both state builders gained `"tasks"`. The replay sets `face.tasks` from
it and paints, so EVERY path (boot, reload, the in-flight refusal fallback)
rebuilds cap-proof; the live events keep it current after.

**The branch discipline (the owner's ruling):** the work went to branch
`reload-reattach` in its own worktree `~/Projects/fleet-reload-reattach`
(based on face-plugin-host 5ee8544) — the KI burn-down session is active in
`/private/tmp/fleet-c13-gate` / `fleet-c13-old` worktrees and pushes to BOTH
refs (a1502af), so a shared checkout would have yanked their branch. Pushed
`935d148` (the feature) + `a3a5437` (the close-out) to
origin/reload-reattach only. **Then the owner asked "would it be okay to
merge it?" — and the merge was a pure fast-forward**: origin/master sat
exactly at the branch's base (a1502af, the burn-down's own latest) with
zero master-only commits, so nothing could conflict; verified by fetch +
merge-base before answering, then ff'd in `fleet-master-wt` and pushed
`a1502af..a3a5437 master -> master`. The face at 3080 had been running
that exact binary since the delivery, so master and the deployment
agreed at the merge. The safe-merge procedure when parallel sessions are
active: fetch, read the merge base, count master-only commits — a FF
needs no gate re-run beyond the branch's own evidence; face-plugin-host
now sits behind the merged state (optional tidying).

**Landed:** `storeOpenSession`/`storedOpenSession` helpers + four bind sites
(openWork, the New mint, dispatchTask's create-bind) + cleared by
`detachToWelcome` AND the home walk's own detach block (which must stay a
LIGHT detach — it keeps the run state by N1's design; only New's
detachToWelcome clears running/driving/seat); the chat branch re-attaches
via `openWork(stored, {keepHash: true})` BEFORE readHome — the ordering
works without a booting flag because a deliberate detach clears the key
first; `fleet-pane`/`fleet-pane-tab` written by openPane/closePane/
showPaneTab and applied at boot before `showRoute()` (the tab read before
openPane, so openPane's showPaneTab uses the stored tab); the bridge pin
`session_tasks_reads_the_last_tasks_record_from_the_log_tail` (the last
record wins, an interposing record does not confuse the scan, no record =
the honest absence). The three N2-era tests were already re-pointed by
KI-453's work.

**Verification:** gate on the branch — fmt 0, clippy 0, release build 0,
`cargo test` **2,124/0** (the branch carries the burn-down's tests beside
mine); harness **962 checks / 110 cases** (five RE cases: the memory, the
re-attach, the pane, the deliberate detach surviving, the refused id's
named welcome). **The live proof on the owner's REAL face and their own
497-checkpoint session** (`20261106T033200Z-14f5a714`): the chip read the
goal, the card held 6 rows, the reload returned the conversation + chip +
all 6 rows (rows the capped log had dropped — the state-document leg
proving itself in the wild) with the pane open at the run tab; New + reload
→ the welcome stands. **One wrinkle named, not fixed:** the welcome's chip
shows the LAUNCH session's standing goal — the bare read on a true welcome,
pre-existing behavior C7 permits; quote it, don't silently inherit it.

**Delivered:** the branch binary installed through a new inode
(351456307), the service restarted (3080 → 200, `storeOpenSession` in the
served shell), the dmg rebuilt with `detachToWelcome` verified inside.

**Found on the way:**
- **The drive hit 100% AGAIN mid-gate** — `~/.fleet/checkpoints` at 69 G
  (the burn-down's long runs); the owner-authorized prune re-ran (821 turn
  dirs removed, kept 20/session → **55 GiB free**). See
  [[checkpoints-unbounded-72g-disk]] — the prevention (the session-close
  hygiene plan's C2) is still not built.
- **Two clippy nits in the burn-down's `reload.rs`** (collapsible ifs at
  149/478) — their C6 rode face-plugin-host, which is this branch's base,
  so the gate could not run green without them; fixed mechanically (a match
  guard, one collapsed condition, brace rebalance) with the note in the
  commit + recap so the merge holds no surprise. Their own worktree copy
  untouched.
- **The Edit-parameter escape trap:** an old_string containing a literal
  `\u0000` (backslash-u as SOURCE text) must be passed `\\u0000` — the
  parameter unescapes `\u0000` into a NUL character and the match silently
  fails while the co-tenant's stale-buffer write made it look like a
  landed-then-reverted edit; verify the on-disk region, not the tool's
  success report (see [[a-parallel-session-co-edits-your-file]]).
- **The unit tests live in the fleet LIB target** (src/lib.rs, 1,918
  tests) — `cargo test --lib` alone runs fleet-desktop's 6 first and
  `--bin fleet` filters to 0; filter bare (`cargo test <name>`) and read
  the Running lines.

**The post-merge push check ("was this pushed to the remote?"):** fetch,
then read the remote tips directly — origin/master carried the merge
(a3a5437) plus the co-tenant's `24ad44b` readme commit on top, and
origin/reload-reattach sat at the branch tip. `face-plugin-host` was the
one ref NOT pushed: 5 local commits ahead (all the burn-down session's —
their C12 close-out, their readme docs, their own merge of master into the
branch), left for them because they were mid-flight in the shared checkout.
Pushed `1259e91..3bd0c08` (the memories churn) earlier the same day, then
the burn-down's commits accumulated on top — a shared checkout's ref state
is whoever committed last, so answer "was it pushed" from a fresh fetch,
never from memory.
