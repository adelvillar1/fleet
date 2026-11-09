---
name: rebuild-is-not-commit-push-or-ship
description: "feedback 2026-10-20 → 2026-11-03 — the owner has asked about the delivery state EIGHT times (commit/push/rebuild/restart, and since the Tauri shell: the .app/dmg too, which the gate never covers and which silently ages); say the delivery state (committed/pushed/rebuilt/ restarted/dmg) in every turn's closing line, and when the owner says go, ship the whole thing in one turn"
type: feedback
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 11
superseded-by:
valid-until:
---

After building and live-verifying the pairing card plus the PWA plus a new
app icon, I closed each turn with what was verified and a passing gate —
and the owner had to ask "did you commit, push and rebuild the app?".
The honest answer was: rebuilt (release binary, face restarted on it,
launcher re-signed with the new `.icns`), **not committed, not pushed**.
`HEAD` and `origin/master` were both still at the pre-session commit and
36 entries sat dirty in the tree.

**Why it matters.** A green gate, a live audit and a running face read as
"done" — but the work lived only in one working tree. Only the owner can
say whether a push is wanted, so "uncommitted" is a legitimate state to
stop in; leaving it **unstated** is what is not legitimate, because it
makes local verification look like a delivery and leaves a whole session
one `git checkout` away from gone.

**"Has the application been rebuilt?" has three answers (2026-10-20/21).**
The owner asked twice; the answer that landed separated the binary on disk
(rebuilt at T, no source newer), the process serving the port (launched
before T, so it serves the old build — identify it by
`lsof -nP -iTCP:<port> -sTCP:LISTEN` plus `ps -p <pid> -o lstart`), and
what the build actually contains (a dirty tree builds the other session's
WIP too). Never answer with a bare "yes"; give the restart command and say
what it will and will not pick up. See
[[verify-the-binary-is-newer-than-the-source]].

**How to apply.** Every recap that ends in a working change names its
delivery state in one explicit line — rebuilt / restarted / committed /
pushed — and the recap offers a commit split when the tree holds more than
one coherent change. Prefer naming the split (one commit per increment)
over one sweeping commit; note which paths are the owner's, not the work
(`.zcode/`, `tmp/`). Do not infer that "the app works on my screen" means
the owner has it. Related: [[fleet-memory-corpus-dirties-the-tree]] (why
a clean-tree check is not automatic here),
[[verify-the-binary-is-newer-than-the-source]] (the rebuild half of the
same question).

**The third ask, three days of work deep (2026-10-30).** The owner asked
"was all of this push to the remote?" — and **17 commits were unpushed**,
reaching back through yesterday evening's settings/bots revamps to
`41573c0`; the day's four revamps, the fixes, and the doc gallery were
all local-only. The new dimension: the session-recap skill's "do NOT
commit" (which is about the recap file itself, leaving the owner the
review) had spread in my framing into leaving the WHOLE day unpushed and
unstated — the recap never named the delivery state this lesson
mandates. The close: committed the recap + the four plan completions as
`7c1eaef`, pushed `41573c0..7c1eaef` (18 commits), master even with
origin. **How to apply:** the delivery-state line is owed at every turn's
end, not just the recap's — and "the skill says don't commit" scopes to
the skill's own artifact, never to the day's verified work; when the
owner asks about push state, the answer is the ahead-count and the
push, not an apology.

**The close of the loop (2026-10-20):** the owner said "let's push it, also
reload the process so that we get the changes in". What that meant in one
turn, and what "ship" should mean by default from now on: stage the work
paths (never `.zcode/` or `tmp/`), commit with a body that names each
concern, `git push origin master` and read the push line, then **restart
the face** on the new binary and confirm the served output (the PWA hash
grep is the cheap proof). Check the face is idle before killing it — the
run-in-flight check is part of the restart.

**The plan split is not a commit split.** I offered five commits and made
one: the concerns interleaved inside the same files (`app.js`, `app.css`,
`app/server.rs`, the three docs), so a true split needed hunk-level surgery
and would have produced intermediate commits that neither built nor
behaved. Verify the split's feasibility (do the concerns share files?)
before promising it; when they do, one coherent commit whose body names the
parts is the honest unit, and say that is why.

**The push does not finish the delivery either.** After the push the
owner still needed the process reload to *see* it, and installed copies
(phone PWA, a Dock icon) keep the old assets and icon until they are
re-added. Delivery state therefore has four legs — rebuilt, restarted,
committed+pushed, and every client the owner actually looks at — and the
last one can be the only one that is missing.

**An inspection step is not a stopping point (2026-10-20).** The owner said
"commit and push all changes"; I ran one diff-inspection command and ended the
turn there, and the owner had to ask "and why did you stop?". An ask that ends
in a push carries through every step in one turn — inspect, commit, push,
report — and if the tree turns out to be moving under you (a tandem session
committing concurrently, see
[[shared-file-commits-can-sweep-another-sessions-work]]), that is narration
inside the turn, not a reason to end it.

**The same stop, mid-investigation (2026-10-22).** The owner asked "why did
you stop?" again, and this time nothing outward was pending: I was tracing how
a run picks its session, had just read the code, and ended the turn between the
read and the fix. A trace that ends in a question is not a finished turn — the
reading is the middle of the work, and stopping there leaves the owner holding
a half-answered report.

**Commit-message findings are the sixth thing a delivery turns on (2026-10-20,
adfd353).** A commit body is the only place several decisions survive — the
`resolved now` column is per tier and not the run's route; lane defaults merge
under the model's arguments; `lanes.json` is fleet-owned and the lane repos are
not rewritten; a parallel session's WIP is excluded from this commit. Put
those in the body, not only in the recap, because the next reader finds the
commit and not the conversation.

**The fifth leg is not static; it can restart underneath you (2026-10-20).** Once the face
runs under `com.fleet.gui`, "did you restart it?" stops being answerable from your own
actions: the agent's KeepAlive and a parallel session both restart the same job. The owner
asked "did you restart the app that is running on port 3080?" and the only honest answer
was "it was restarted — by the agent, at 16:39:45, run #2 — not by me, and that build
predates two later commits". Read `launchctl print gui/$(id -u)/com.fleet.gui` (state,
runs, pid), `ps -p <pid> -o lstart`, and `git log -1 --format=%ci` together; never answer
a restart question with a bare yes.

**The fourth ask, and the answer that closes it (2026-11-02).** The owner
asked *"did you push to the remote and reloaded the app?"* — push yes
(0 ahead/0 behind, `3ebf1fe` the tip), reload **no**, because the
closeout work had grown past the last restart and the face still served
the pre-closeout build. The reload is its own four-step pass and it is
owed whenever the tree changed since the last restart: build in the
**master worktree** the plist points at, `launchctl kickstart -k`, then
prove the **served bytes** (`curl -s 127.0.0.1:3080/app.js | grep -c
<symbol>`), not the process list — the assets are `include_str!`'d, so a
rebuild without a restart serves nothing new. See
[[fleet-gui-is-a-launchagent]] for the full recipe. The delivery-state
line now has four legs to name: committed, pushed, rebuilt, restarted —
and "the app" means the process, not the source.

**The fifth leg: what keeps running (2026-10-20).** The owner asked "is that
something that you can do for me?" about two follow-ups I had named — leaving
the face running and the tailnet mapping. Do the parts that are local and
reversible without asking (a LaunchAgent is one plist and one `bootout`);
stop at the parts that change a setting on the owner's account (enabling
Tailscale Serve is tailnet-wide — hand over the admin link, do not click it
for them). For a persistent face, a LaunchAgent beats `nohup`: install
`~/Library/LaunchAgents/com.fleet.gui.plist` running the release binary
`gui --no-open --remote` with `RunAtLoad`, `KeepAlive.SuccessfulExit=false`,
`ThrottleInterval` 10, and both stdio paths at
`~/Library/Logs/fleet-gui.log`; `launchctl bootstrap gui/$(id -u) <plist>`
(after killing the manual face so the port is free), then verify with
`launchctl print gui/$(id -u)/com.fleet.gui` that `state = running` and the
log shows both listeners. That makes the Dock/PWA icon a client with a server
behind it at login, which is the difference between an app icon and a
bookmark that usually says "fleet is not reachable".

**The fifth ask, and the answer shape it settled (2026-11-02).** The owner
asked *"did you push to the remote and reloaded the app?"* and then, minutes
later, *"has this work been committed?"* — the same question twice, from
two angles, because the first answer had named push but not the reload and
the second wanted the commit state. The shape that closes it in one line:
**committed, pushed (with the ahead/behind count and the tip), rebuilt,
restarted** — and when the tree holds other sessions' work, name those
files as not mine. A bare "yes" to any of the four legs is the failure
mode; the count is what makes the push claim checkable, and the restart
claim is only true if the served bytes were re-proven.

**The sixth ask, and the measurement that caught a 2.5-hour-stale face
(2026-11-02, late).** The owner asked *"is there anything pending to merge,
the app rebuilt, and relaunched?"* — three legs in one question, and the
answer was: merges none, rebuild **no**, relaunch **no**. The face's
LaunchAgent binary was built at 18:16 from `3ebf1fe`, **before all of that
day's GUI work**, and the served `app.js` had **zero** occurrences of a
symbol the new source carried (`grep -c plugins` → 0; 950,592 bytes served
vs 961,837 in the source). The fix in one pass, and the measurement that
proved it: `git -C ~/Projects/fleet-master-wt merge --ff-only
origin/master` → `cargo build --release` there → `launchctl kickstart -k
gui/$(id -u)/com.fleet.gui` → `curl -s 127.0.0.1:3080/app.js | grep -c
plugins` = **36** and the byte count **exactly the source's** (961,837),
plus `/` → 200 and both relays rejoined in `fleet-gui.log`. **How to
apply:** the three-leg question is the standard close-out ask — answer
each leg separately with its measurement, and never let a green gate stand
in for the served bytes: the gate proves the tree, the `grep -c` of a
new symbol proves the **process**. A stale face is invisible from the
source and from the process list; only the served asset and the binary's
mtime against the last commit expose it. See
[[fleet-gui-is-a-launchagent]] for the recipe and
[[verify-the-binary-is-newer-than-the-source]] for the artifact half.

**The eighth ask: the delivery is two artifacts and two refs (2026-11-03,
late).** The owner asked *"is everything built including the tauri app and
push to the remote?"* — and the Tauri dmg had gone **stale** (built
2026-11-02 21:41, before the provider-connect and composer-actions fixes);
it is not part of the gate, so nothing red when it fell behind. The push
shape was new too: the day's work sat on `face-plugin-host` while the
local `master` worktree (`fleet-master-wt`) held an older checkout —
`git push origin face-plugin-host:master face-plugin-host` pushed the
branch tip onto origin/master **without touching the held worktree**,
which then needs a pull. **How to apply:** the Tauri dmg is a second
delivery artifact that silently ages — rebuild it (`cd src-tauri && cargo
tauri build`) whenever face assets or the core changed, not only when
asked; and a held local branch never blocks the push — push the ref, then
name the worktree the pull will land in.

**The seventh ask: "the app" is two artifacts, and the owner supplies
the rebuild command (2026-11-03).** The owner asked *"did you rebuild the
web app and the tauri app, push to remote, and reloaded the web app?"* —
four legs, and a new dimension: **"the app" is two delivery artifacts**,
the web face (the LaunchAgent's binary) and the **Tauri app** (the
.app/dmg, which embeds the face — see
[[tauri-native-shell-implemented-2026-11-02]]). I had rebuilt the web
face and pushed but not touched the Tauri app, and my rebuild attempt
used plain `cargo build --release` — the owner cancelled it and supplied
the command: **`cd src-tauri && cargo tauri build`** (the bundle step is
what ships the .app/dmg). The proof the new face is embedded:
`strings target/release/fleet-desktop | grep -c pluginBuiltIns` (2) —
the served-bytes discipline applied to an embedded binary.
`/Applications/fleet.app` was stamped by the build itself. **How to
apply:** the delivery-state line now names the Tauri artifact whenever
face assets changed, and the Tauri rebuild command is the owner's, not a
guess — a plain release build of the crate builds the binary but ships
no bundle.
