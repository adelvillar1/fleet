---
name: ki-431-delivery-restart-leaves-face-unloaded
description: "KI-431 — the binary delivery's service restart can leave the face's LaunchAgent unloaded (bootout without re-bootstrap); verify post-state and re-bootstrap by name"
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

KI-431 (filed 2026-11-04, committed e14b9c5): the binary delivery's
service restart can leave the face's LaunchAgent unloaded — the
bootout made for the inode swap was not followed by a re-bootstrap, so
the face and both public relays stayed down until a manual load. The
owner experienced it as "fleet is down."

**Why:** the diagnosis pattern is reusable — the agent log
(~/Library/Logs/fleet-gui.log) stopped mid-startup with no panic and
no bind failure (a crash-looping KeepAlive keeps writing; a stopped
log means removed), `launchctl print gui/501/com.fleet.gui` answered
"could not find service in domain", and the binary's fresh mtime
completed the picture. A manual
`launchctl bootstrap gui/501 ~/Library/Agents/...plist` restored it.

**How to apply:** the delivery/restart step verifies the agent's
post-state — `launchctl print` shows a pid and the port answers HTTP —
and re-bootstraps by name when the domain lacks the service; a service
outage is named by the step that causes it, never assumed away. When
the agent IS loaded, `launchctl kickstart -k gui/501/com.fleet.gui` is
the working restart (verified 2026-11-04 and 11-05: pid changes, port
answers, relays rejoin). See also
[[fleet-gui-is-a-launchagent]] and
[[the-served-face-is-the-installed-copy]].
