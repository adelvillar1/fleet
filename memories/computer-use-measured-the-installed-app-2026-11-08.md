---
name: computer-use-measured-the-installed-app-2026-11-08
description: "feedback 2026-11-08 — the Computer Use accessibility tree is a working live-behaviour probe for the INSTALLED desktop app (paste, focus, notices), and the owner prefers the four candidate cases measured rather than asked"
type: feedback
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

The owner's ask — "we now need to implement pasting from the clipboard into
the composer" — named four plausible cases (Cmd+V in the desktop app, image
paste, a paste button, text paste in another surface). I asked which one; the
owner answered **"why don't you test the 4 questions yourself?"** — and
measuring them was the right move: one case was already working, one was a
silent no-op with unreachable refusal code, and the other two were
surface-specific. The measurement found the real gap in one pass.

**The method that worked.** Bind the installed app by bundle id
(`agent.computerUse.getApp({ bundle_id: "dev.fleet.shell" })` — the bundle id
is in `Contents/Info.plist`, not the display name; `listApps()` does not list
the app because its display name is lowercase `fleet` and the list is
capitalized), then drive the accessibility tree:

- `getAXState({ disableDiffing: true })` for a full tree (the composer is a
  `textarea the task (editable focused)`; the tree is small, ~94 elements).
- `app.typeText(...)` to prove the focus before testing a paste — a paste test
  that fails for lack of focus reads as a broken paste.
- `app.setValue(index, "")` to clear the composer between cases (the stub's
  `setValue` is the reliable clear; `elements()` gives the index).
- `app.pressKey("super+v")` for the paste, then re-read the tree and grep the
  composer's row for the marker.
- `app.getScreenshot({ emit: false })` for a visual read (a notice, a layout).

**Two traps.** (1) The first Cmd+V right after launching the app can be
swallowed — the app is not the key window yet; re-test after the composer has
genuinely been focused. (2) The clipboard must be set from the shell
(`printf 'MARKER' | pbcopy`; `screencapture -c` for an image) and confirmed
(`pbpaste`, `osascript -e 'clipboard info'`) — the probe reads the real
clipboard, so a stale one reads as a broken paste.

**Why:** the owner's correction is the same discipline as
[[say-only-what-you-have-read]] and the measure-and-read mockup rule — do not
present an assumption as a blocker, and do not ask a question the tooling can
answer. A live-behaviour probe over the accessibility tree is available
in-session and beats both asking and theorizing.

**How to apply:** for a live-behaviour question about the installed app, bind
it by bundle id, prove the focus, drive the action, and re-observe — then
report what each case did. Related: [[composer-paste-measured-live-2026-11-08]]
(the measurement this enabled), [[gui-client-verified-by-a-vm-harness]] (the
face's other verification paths), [[measuring-a-mockup-by-reading-the-png]].
