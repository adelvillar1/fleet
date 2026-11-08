---
name: owner-interrupt-kills-a-blocked-waited-build
description: "An owner interrupt kills a turn blocked in `process wait`, and a build riding the face's session dies with the face's restart — long artifact builds must be double-forked at birth, and a turn must never end inside a blocking wait."
type: feedback
date: 2026-11-08
basis: inferred
tier: durable
shape: [state-sync, fail-open]
phrases: [run_in_background, process wait, cargo tauri build, nohup python3 os.fork os.setsid, kickstart -k gui/$(id -u)/com.fleet.gui, /tmp/tauri-build-051.log]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

The 2026-11-08 ship turn left `cargo tauri build` as the turn's last tool call blocked in `process wait` — the owner's next message interrupted the turn, and the interrupt took the build with it (0-byte background log, tracker id gone, src-tauri/target never created). A build process inherits the session and dies with it regardless of run_in_background; running_in_background only detaches the LOG, not the lifetime, inside a live wait. The recipe that ships without this trap: (1) never end a turn blocked in process wait — kick the artifact build, report, let the waiter be nohup+double-fork (python os.fork ×2 + setsid, execvp) so it reparents to launchd and survives both the turn and the face's own restart; (2) the face's launchd relaunch (kickstart -k) kills everything under the face pid, so a build riding the gui session dies at the reload even if it would outlive the turn; (3) verify the artifact in the NEXT turn from its own log file (/tmp/tauri-build-051.log), never from the tracker.
