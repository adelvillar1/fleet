---
name: bot-pwa-dogfooded-live-01a9ab3
description: "2026-10-28 — the owner's \"we have not used persona (bot) mode because you just built it\" forced a live dogfood as a paired device; four defects found by using it (the mode-word alias, the remote run policy never parsed, the catch-up restart storm KI-350, the persona's voice) — all fixed and committed 01a9ab3, gate 1,771 green, the whole loop verified over HTTP"
type: project
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

2026-10-28, owner: **"we have not used persona (bot) mode because you just
built it."** The bot-PWA implementation ([[bot-pwa-implemented-2976098]])
was built and gate-green but never run. The dogfood drove the face end to
end as a paired device over HTTP and found **four defects the tests could
not catch** — all fixed and committed as **`01a9ab3`** (gate: fmt/clippy/
release/1,771 tests, exit codes read).

**The dogfood instrument** (reusable): mint a code on the console
(`POST 127.0.0.1:3080/api/pair/window`), redeem it on the remote origin
(`POST /api/pair` with `{"code","name"}` → `Set-Cookie: fleet_device=…`),
then curl everything with that cookie — the shell, `/bot.webmanifest`,
the 403 on `/app.css`, `/api/inbox`, `/api/session/new`, `/api/run`,
`/api/session/join`, `/api/goal`, `/api/approve`. A curl IS a paired
device. See [[fleet-pairing-qr-without-tailscale-serve]] for the
LaunchAgent restart.

**The four defects:**

1. **The mode word.** `run_policy_of` accepted `auto`/`yes-guarded` but
   not `guarded` — the bot client's posted word fell through to `Ask`.
   Parser takes the alias; the client posts the canonical word.
2. **The real one: the remote face never parsed the mode at all.** Its
   `/api/run` called `bridge.run()` — the mode-less constructor — so
   **every bot run ran in Ask mode, asking on every call**, whatever it
   posted. C7 was never live where it mattered. Fixed: the remote route
   reads the policy (mode/pin/budget/folder) through the console's own
   parsers, made `pub(crate)` — see
   [[a-second-face-must-share-the-firsts-parsers]].
3. **The catch-up restart storm (KI-350).** `record_run` wrote
   `last_run` only when a run ENDED, so a catch-up run killed mid-flight
   was indistinguishable from a window never served, and every gui
   restart re-served every window whose run died — four rapid restarts
   multiplied the ~14 jobs that missed today's 00:15–00:50 window into
   **89 open scheduled logs**, all queued on the bridge's serial command
   loop, starving the interactive run. Fixed:
   `Store::mark_dispatched` stamps `last_run` at dispatch
   (`fire_one`'s entry); the outcome write still lands at the end.
   Pinned by `a_window_stamped_at_dispatch_is_not_refired_by_a_restart`.
   To clear a flood: disable all 15 jobs (`fleet cron disable --name …`),
   one clean restart, dogfood, re-enable.
4. **The persona's voice.** The run said "Ran as Fleet" — the log named
   tower but the model never heard it. The lead persona's overlay now
   rides every face run (`role_overlay` from `roles::load` +
   `overlay_prompt`, the same seam a delegation's role uses). Second
   run: *"I am **tower**, fleet's lead persona."*

**Verified live after the fixes:** the guarded run appended to
`tmp/bot-dogfood.md` with **no seat** (non-destructive runs); the replay
card carried `persona: tower`; auditor joined as a log row and the inbox
read `personas: ['tower','auditor']`; the owner's goal change read back
in the row; the destructive seat fired flagged
(`write_file · destructive: true · allowlistable: false`) and **held
under denial** (`approved: false` in the log, the file intact).

**Operational notes:** the gui is supervised by LaunchAgent
`com.fleet.gui` with KeepAlive — `kill`/`pkill` are respawned within
seconds; `launchctl kickstart -k gui/$(id -u)/com.fleet.gui` is the only
clean restart (recorded in
[[fleet-pairing-qr-without-tailscale-serve]]). Restarting fires cron
catch-up — quiet a restart by disabling the jobs first. The KI ledger
grew underneath the session (automated rows landed as KI-341..349) and
the new row's ID collided three times — take the highest free ID at
write time.

Related: [[one-goal-many-personas-destructive-is-the-only-gate]],
[[bot-pwa-followups-9b4ef7b]],
[[a-second-face-must-share-the-firsts-parsers]].
