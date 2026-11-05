---
name: a-relay-join-secret-lives-outside-the-dotenv
description: "project 2026-11-02 — both FLEET_*_RELAY_JOIN_SECRET values are in the plist EnvironmentVariables and ~/.fleet/.env; never echo a plist read that can print them, and mistaking one relay's file for the other manufactures a false non-ff"
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

State after the 2026-11-01 rotation: `com.fleet.gui.plist` carries
`FLEET_RELAY_JOIN_SECRET` and `FLEET_BOT_RELAY_JOIN_SECRET` in its
`EnvironmentVariables`, and names values only via the OpenShip project
`fleet-relay` (services `relay`, `bot`) → `~/.fleet/.env`. No relay
secret is in this session's process env, so OpenShip CLI calls here
cannot authenticate (they error "not signed in").

**How to apply:**
- **2026-11-04 reinforcement — a whole-file `cat` of the plist prints
  BOTH secrets into the session.** Diagnosing "fleet is down" I read
  the plist with plain `cat` to see ProgramArguments and both join
  secrets landed in the conversation. Extract only the keys the
  question needs (`/usr/libexec/PlistBuddy -c "Print
  :ProgramArguments"`, or `grep` the specific key), and if a
  whole-file read happens anyway, name it once and never repeat the
  values into any log, tower-log line, or repo file.
- Anything I read that can print them: restart is
  `launchctl kickstart -k gui/$(id -u)/com.fleet.gui`, and
  `~/Library/Logs/fleet-gui.log` ends with `fleet relay — joined
  https://fleet.opsh.io` / `…fleetbot.opsh.io`. Proof the served assets
  are the new build: fetch `/app.js` (the index names
  `/app.js?v=<hash>`; `/assets/app.js` is the 43-byte 404) and grep for
  the change.
- Config edits made in a SIBLING task's worktree
  (`~/Projects/dev-decisions/.worktrees/<task>`) are invisible to the
  fleet launch — the join secret fails the same way every time; the
  running face reads the installed/home config, not another repo's tree.
- When the task is "reload the app", do the full chain or say which leg
  is yours: build → kickstart → verify served bytes → state which worktree's
  config the face reads.
Related: [[relay-join-secret-rotation-2026-11-01]],
[[openship-env-set-apply-pattern]], [[fleet-gui-is-a-launchagent]].
