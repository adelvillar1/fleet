---
name: the-served-face-is-the-installed-copy
description: "Rebuilding target/release/fleet does not change the console the owner browses — the LaunchAgent runs ~/.fleet/bin/fleet, and the deploy is a separate manual step"
type: feedback
date: 2026-11-05
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 33
superseded-by:
valid-until:
---

The owner asked why the console had no huddles rail link when the code
had it. The code was fine: the LaunchAgent `com.fleet.gui` runs
`/Users/alejandrodelvillar/.fleet/bin/fleet`, the **installed copy**, and
nothing keeps it current with `target/release/fleet`. That copy was built
2026-11-03 13:56 — before the huddle work — so the served `/app.js` had
zero `huddles` strings while the source and the fresh build had them all.

**Why:** "rebuilt the app" and "the owner sees the change" are two
different acts. A green build, a fresh dmg and a pushed branch all leave
the running console exactly as it was, and the owner reads the running
console. KI-408 closed this class with a two-command runbook; the
runbook is a manual step, which hard rule 12 names a design defect (now
KI-412, open).

**How to apply:** when a change is user-visible, the delivery is part of
the work — `cargo build --release && cp target/release/fleet
~/.fleet/bin/fleet`, then restart the agent
(`launchctl bootout gui/$(id -u)/com.fleet.gui`, wait, then
`launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/com.fleet.gui.plist`
— the first bootstrap after a bootout can fail with `Input/output error`;
retry it), then **prove the served asset**: fetch `/app.js` and grep for
the change, and check the endpoint the page reads. The Tauri dmg is a
second, separate artifact (`cargo tauri build`) and the `.app` is
cleaned from `bundle/macos/` by the dmg step. Related:
[[a-relay-join-secret-lives-outside-the-dotenv]],
[[huddle-plan-completed-2026-11-04]]
