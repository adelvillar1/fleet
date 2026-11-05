---
name: fleet-gui-is-a-launchagent
description: "fleet gui is supervised by com.fleet.gui (KeepAlive) — the daemon runs the canonical installed binary ~/.fleet/bin/fleet (KI-408); restart by killing the PID (kickstart's specifier failed 2026-11-03) and bootout + bootstrap after any plist edit"
type: project
date: 2026-11-05
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

2026-10-28 (fleet): `fleet gui` runs under the LaunchAgent
`~/Library/LaunchAgents/com.fleet.gui.plist` (`com.fleet.gui`, KeepAlive,
ProgramArguments → `target/release/fleet gui --no-open --remote`). Killing the
process directly does not stop it — launchd respawns it within seconds and the
respawn binds 3080 first, so a `nohup … &` attempt dies with "Address already in
use", and repeated kill/relaunch attempts produce a storm of one-off
session logs (a respawn creates empty launch sessions and each restart's cron
catch-up refires missed windows).

**How to apply:** CHECK WHICH WORLD YOU ARE IN FIRST. On 2026-10-29 ~11:48
the gui ran as a PLAIN process (PID 79857, `fleet gui --no-open --remote`,
no FLEET_/LAYERFS_/DEEPSEEK env vars — credentials come from `~/.fleet/.env`
at runtime), `kill 79857` did NOT respawn it, and a plain `nohup …
target/release/fleet gui --no-open --remote >> ~/.fleet/gui-3080.log 2>&1 &`
relaunch bound 3080 first try. The plain world held all day 2026-10-29 —
six more kill+nohup relaunches (96252, 22518, 36341, 39201, 41891, 44950)
each bound 3080 first try with no launchd interference, so in this world the restart recipe
is simply kill the PID, confirm no run in flight (newest session mtime),
nohup the same args, verify the port + the served asset. So the LaunchAgent
supervision is not always
in force — before restarting, look: `ps aux | grep "fleet gui"` (args + whether
a launchd parent or a nohup one) and `launchctl list | grep fleet.gui`. If
launchd owns it, restart with `launchctl kickstart -k gui/$(id -u)/com.fleet.gui`
(one clean respawn, prints nothing but binds) —
`kickstart -k` worked fine on 2026-10-29 after an earlier observation that
it exited 78, so try it first and fall back to `bootout` + `bootstrap`.
**BUT `kickstart -k` does NOT re-read an EDITED plist** (measured
2026-10-29 evening: after adding `--bot-relay` to ProgramArguments and
kickstarting, the respawned process still ran the OLD args — launchd
held its cached definition). After any plist edit the reload is
`launchctl bootout gui/$(id -u)/com.fleet.gui` then `launchctl
bootstrap gui/$(id -u) ~/Library/LaunchAgents/com.fleet.gui.plist`;
bootstrap can fail once with `5: Input/output error` racing launchd's
cleanup — wait a beat and retry (or `launchctl load <plist>`). Also
2026-10-29: the plist now carries StandardOutPath/StandardErrorPath →
`~/Library/Logs/fleet-gui.log`, so the launch's own words (the relay
entry URLs with their `#k=` secrets, the bot face line) are readable
there instead of dying in launchd's void, and ProgramArguments is
`fleet gui --no-open --remote --relay https://fleet.opsh.io --bot-relay
https://fleetbot.opsh.io`.
Before any kill, confirm no run in flight: newest `~/.fleet/sessions/*.jsonl`
mtime against the clock (2026-10-29 the newest write was 2 h old — safe).
Because the face serves `include_str!`/`include_bytes!`-embedded assets,
a commit is invisible until `cargo build --release` runs first; the proof
that the new code is live is the SERVED asset
(`curl -s 127.0.0.1:3080/app.js | grep -c <new symbol>`), not the source.
Check `/api/state` for `running`/a standing seat before kicking.

**2026-11-03, KI-408 — the serving binary has ONE home now, decoupled from
every worktree.** The owner reported the PWA still showing turn-count
indicators the browser face had lost, and the cause was not the retirement
regressing but the daemon serving a STALE binary: `ProgramArguments`
hardcoded the **fleet-master-wt worktree's own** `target/release/fleet`
(a second build tree no session rebuilds), built 09:31 that morning —
before the day's composer-actions and provider fixes — so the face the
owner browses and the face the PWA reaches both lagged the pushed master
(the served app.js was 962,880 bytes vs the current 967,393, a 107-line
diff). The trap: the browser talks to the SAME stale server, so locally
everything looks consistent — the discrepancy surfaces only against a
second viewer (the phone's PWA), which is why it read as a
pwa-vs-browser bug. The fix is structural: the serving binary has ONE
home, `~/.fleet/bin/fleet`, and the plist's ProgramArguments points
there; the reload is two commands and a restart, recorded in
`docs/features/install.md`'s release-binary section (29b5187). Measured
this day: `launchctl kickstart -k gui/$(id -u)/com.fleet.gui` FAILED with
"Unrecognized target specifier" (it had worked on 2026-10-29 — do not
trust the specifier), so the restart that works is kill the PID and let
KeepAlive re-exec, and a PLIST EDIT still needs bootout+bootstrap (a
third confirmation). The proof the reload landed stays the served bytes:
diff the served `/app.js` against `src/gui/assets/app.js` —
byte-identical is the pass.
**2026-11-02 the agent was REPOINTED** from
`/Projects/fleet/target/release/fleet` (the other session's checkout on
`relay-proof-fix`) to `/Projects/fleet-master-wt/target/release/fleet`,
because the owner asked for the settings work to be "part of the app".
The edit was `/usr/libexec/PlistBuddy -c "Set :ProgramArguments:0
<path>" ~/Library/LaunchAgents/com.fleet.gui.plist` (backup
`com.fleet.gui.plist.bak-20261102`), then `launchctl bootout
gui/$(id -u)/com.fleet.gui` + `launchctl bootstrap gui/$(id -u)
<plist>` — again, `kickstart -k` respawned the OLD binary (same PID
family, `grep -c` of the new symbol in the served app.js = 0) until
bootout+bootstrap — the second measured confirmation that a plist EDIT is
invisible to kickstart, so after any ProgramArguments change read the
served bytes, not the process list. The plist itself is the file-level
record (`/usr/libexec/PlistBuddy -c "Print :ProgramArguments:0"`), and it
carries the launch-time environment (`EnvironmentVariables`) — which is why
secrets placed there are still secrets in the plist even after the vault
seals its `.env` copies. Also: the plist stores `FLEET_RELAY_JOIN_SECRET`
and `FLEET_BOT_RELAY_JOIN_SECRET` in PLAINTEXT under
EnvironmentVariables (and the owner's `.env` has them too) — worth
moving when the owner says so. The face on 3080 now SERVES the
settings flow: the model list, and the accounts card carrying
`connect a provider` and `reload every provider` (verified
`curl -s 127.0.0.1:3080/app.js | grep -c "reload every provider"` = 1).
The other fleet
LaunchAgent is `local.fleet-maintenance` (09:20). Two ports to expect after
2026-10-28: 3080 = the console (loopback + the machine's LAN address) and
3081 = the bot face (`--bot-port`). Note for the catch-up interaction:
[[gate-the-staged-tree-not-the-working-tree]].

**2026-11-02, the closeout reload — the full recipe that worked.** The
work had grown past the last restart, so the face still served the
pre-closeout build. The reload in one pass: (1) `git -C
~/Projects/fleet-master-wt merge --ff-only origin/master` (the plist's
ProgramArguments[0] points at the **master worktree's** binary, so the
build happens there, not in the session's checkout); (2) `cargo build
--release` in that worktree; (3) `launchctl kickstart -k
gui/$(id -u)/com.fleet.gui` — no plist edit, so kickstart is correct
here (the bootout+bootstrap rule above is for an EDITED plist); (4) prove
the served bytes, not the process: `curl -s 127.0.0.1:3080/app.js | grep
-c <new symbol>` (the assets are `include_str!`'d, so a rebuild without a
restart serves nothing new), plus `curl -s .../api/state` for a standing
seat and `grep "relay — joined" ~/Library/Logs/fleet-gui.log` for both
relays. Verified: 4 hits for the closeout symbols in the served app.js,
`/site/sample-board/` and `/api/settings` both 200, both relays joined.

**2026-11-03 late — KI-408's fix confirmed live, and the reload gained a
copy step.** After KI-408 the plist's ProgramArguments points at
`~/.fleet/bin/fleet` (the installed copy) — verified by `ps` on the
restarted pid (`…/.fleet/bin/fleet gui --no-open --remote --relay …`), so
**a worktree rebuild alone no longer reloads the face**: the fresh binary
must be COPIED to the installed path first
(`cp ~/Projects/fleet-master-wt/target/release/fleet ~/.fleet/bin/fleet`),
then `launchctl kickstart -k gui/$(id -u)/com.fleet.gui` — which **worked
repeatedly this day** (three reloads: the closeout pass, the flat-shape
fix, the plugins-ux landing; each proven by the served bytes). The
earlier "Unrecognized target specifier" failure did not recur; the
standing recipe is now three commands — ff-merge the worktree, build
there, copy to `~/.fleet/bin/fleet`, kickstart, prove the served bytes —
and the binary-path check (`ps -p <pid> -o command`) belongs at the top,
because a rebuild aimed at the wrong home looks exactly like a build that
did not take.

**2026-11-04 — kickstart failed again; the recipe that worked was
bootout+bootstrap, and bootstrap needed ONE RETRY.** The owner reported no
huddles rail link; the installed `~/.fleet/bin/fleet` was a day old (zero
`huddles` strings). `launchctl bootout gui/$(id -u)/com.fleet.gui`
succeeded, and the immediate `launchctl bootstrap …` printed `Bootstrap
failed: 5: Input/output error` — but by then the service was gone from
`launchctl list` and `kickstart` answered "Could not find service"; a SECOND
`bootstrap` exited 0 and the pid appeared running
`~/.fleet/bin/fleet gui --no-open --remote --relay …`. So the standing
recipe is: `cp target/release/fleet ~/.fleet/bin/fleet` → `bootout` →
`bootstrap` (retry once on the 5/IO error) → prove `/app.js` over
`curl` carries the change. Since KI-408 the installed copy is what runs, so
a rebuild alone reloads nothing ([[the-served-face-is-the-installed-copy]],
KI-412 open: the copy is a manual step hard rule 12 calls a defect).

**2026-11-04 evening — the failure mode the recipe itself creates: a
bootout whose re-bootstrap never ran left fleet DOWN for hours.** The
owner asked "check why fleet is down". Findings: the plist intact on
disk (updated 2026-11-03), but `launchctl print
gui/$(id -u)/com.fleet.gui` answered "Could not find service … in
domain" — the agent was NOT LOADED, and KeepAlive cannot act on an
unloaded agent. The agent's own log (`~/Library/Logs/fleet-gui.log`)
stopped mid-normal-startup at 15:48 with NO panic and NO bind failure —
a crash-looping KeepAlive keeps writing, so a stopped log means the
agent was REMOVED, not dying. The deployed binary landed 15:53–54
through a new inode: the signature is a bootout-for-the-swap whose
re-bootstrap never happened (the same afternoon delivery that produced
that binary). Meanwhile a MANUAL face was alive the whole time on port
3098 (checkout build, started 10:34, `--no-open`, NO relay flags) — so
"down" was precise: no established face, and nothing behind
fleet.opsh.io / fleetbot.opsh.io (the phone's door), while a test face
answered on a drifting port. **How to apply:** the face's port is
whatever its invocation said, not a constant — find the real listener
with `lsof -nP -iTCP -sTCP:LISTEN | grep fleet` before curling an
assumed port. The diagnosis order that worked: ps (args + which binary)
→ curl the assumed port → lsof for the actual listener → `launchctl
list | grep fleet.gui` → `launchctl print` for the loaded/not-loaded
verdict → the plist on disk → the agent log tail (stops mid-startup =
booted out; bind failures = port war; panic = crash). The fix is one
command — `launchctl bootstrap gui/$(id -u)
~/Library/LaunchAgents/com.fleet.gui.plist` — then verify PID, bound
port, and both relays joined in the agent log. **Confirmed run the same
evening (2026-11-04 ~16:00):** the owner said yes; bootstrap exited 0,
pid 32862 on the fresh 15:53 binary, 3080 answered HTTP 200 on BOTH
127.0.0.1 and the LAN address (the face binds both in --remote mode)
plus 3081 for the bot face, the agent log showed "fleet relay — joined"
for BOTH fleet.opsh.io and fleetbot.opsh.io, and the public relay
answered 200 from outside — the phone's door was what "down" meant.
The find is filed as **KI-431** (committed e14b9c5): a delivery
restart's bootout must end with a verified bootstrap — `launchctl
print` shows a pid and the port answers — never a copied file. Any
delivery recipe that bootouts MUST end with a verified bootstrap (PID
visible in `launchctl print`), not a copied file.
