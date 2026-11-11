---
name: updater-fix-landed-2026-11-09
description: "The updater works on BOTH channels, proven live (0.6.5 -> 0.6.6); the plan is completed; three field lessons (unprefixed app permission, cp-in-place SIGKILL, GPU-composited window invisible to capture)"
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

The plan `docs/plans/2026-11-09-updater-that-installs.md` is COMPLETED
(2026-11-09): v0.6.5 carries the fix, v0.6.6 is the release it installs, and
**both channels are proven live**.

- CLI: `fleet update` on a 0.6.5 copy installed 0.6.6 and kept `fleet.prev`;
  the About pane's press in a browser did the same to `~/.fleet/bin/fleet`.
- Shell: the installed 0.6.5 app's About pane was reached with `⌘K` then
  `about`, the apply press ran, the plugin installed the 0.6.6 bundle, the app
  relaunched (pid 43242 → 43461), and both `/api/version` on the shell's own
  port and the bundle's `CFBundleShortVersionString` answered 0.6.6.

**Why it matters:** the updating path is now the way releases land — a cut
publishes the feed and the machine's app + CLI install it themselves. The
channel was then used for real the same day: **0.6.7** (KI-495, the send
button with a queue) was installed by `fleet update` from 0.6.6, which keeps
`fleet.prev`, and is what 3080 serves (the desktop bundle lagged at 0.6.6 —
one About press moves it). See [[the-send-button-with-a-queue-ki-495]] and
[[the-release-is-the-owner-s-no-op]].

**How to apply (the field lessons, each paid for):**

- **Tauri app permissions are referenced unprefixed** in a capability
  (`allow-restart-app`), not `fleet-desktop:allow-restart-app`.
- **Never `cp` a binary over a running one.** The write truncates the inode,
  macOS's code-signature cache is keyed to it, and the next exec is SIGKILLed
  (exit 137) over bytes that hash correctly. `rm` + `cp`, or the updater's
  staging rename (KI-491).
- **A GPU-composited window can be invisible to capture while alive.** The
  press had to be aimed blind: the page's tab order was read in a browser at
  the same route (the About pane's controls are its last two focusables, the
  apply press second-from-last), then `⌘K` → `about` → Return → neutral click
  into the pane → Shift-Tab ×2 → Return in the window. `lsof` on the server's
  port is the liveness proof, not the picture.
- The release ships the CLI channel's artifact as a **raw binary** and the cut
  reads its own feed back anonymously. See [[updates-come-from-github-never-locally]].
