---
name: composer-paste-measured-live-2026-11-08
description: "project 2026-11-08 — the composer's paste paths measured live in the installed desktop app: text paste WORKS (Cmd+V), image paste is a SILENT NO-OP whose refusal notice never fires, the bot face has no paste handler, and the TUI's is already handled (KI-026)"
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

The owner asked 2026-11-08 for "pasting from the clipboard into the composer".
Before building anything, the four candidate cases were MEASURED live in the
installed desktop app (`/Applications/fleet.app`, bundle `dev.fleet.shell`,
driven by Computer Use over the accessibility tree) rather than assumed:

- **Text paste works.** A marker on the clipboard (`PASTE-PROOF-1729`) landed
  in the composer's textarea on Cmd+V — after typing proved the focus
  (`TYPE-PROOF-42` landed) and clearing it. The desktop app's Tauri shell uses
  Tauri's default macOS menu, which already ships Edit ▸ Cut/Copy/Paste/Select
  All, so Cmd+V has a menu item wired.
- **Image paste is a silent no-op.** With a PNG on the clipboard
  (`screencapture -c`), Cmd+V left the composer empty AND the refusal notice
  the code promises — *"an image cannot ride the composer yet … an image goes
  through the document surface, which escalates it to Perception"* —
  **never appeared**. The composer's image branch (`app.js:9806`) is
  unreachable in the webview: a native image paste into a `<textarea>` is a
  no-op, so the `paste` event either does not fire or arrives with empty
  `clipboardData`. The code's notice is decoration, not a behaviour.
- **The first Cmd+V right after launching the app was swallowed once** — the
  app was not the key window yet. It works as soon as the composer has
  genuinely been focused (the click-the-composer-then-paste path worked).
- **The bot/remote face has no paste handler at all** (`bot.js`) — text
  pastes natively, files and images do not attach. The TUI's paste is already
  handled (KI-026: `App::paste` normalizes `\r\n`/`\r` to `\n`).

**The face's existing paste design** (`app.js:9806`, the composer's
`<textarea id="task">`): a paste carrying FILES attaches them through the
picker's own bounds (`attachFiles`: ≤5 files, ≤256 KB, text extensions only);
a paste carrying TEXT falls through to the browser default; a paste carrying
an IMAGE is refused with the notice. So the unmet case is **pasting an image
— a screenshot — into the composer**, which is the common "paste a screenshot"
workflow and currently a silent no-op.

**Why:** the owner's ask was ambiguous across four cases, and the obvious
reading (Cmd+V) was already working — building it would have been a no-op
feature. Measuring first exposed that the image path is the real gap, and
that its promised refusal is unreachable code.

**How to apply:** image paste needs a clipboard READ (the webview will not
hand the image to the paste event) — either the Tauri clipboard plugin or
`navigator.clipboard.read()` behind a user gesture — plus an image-attachment
path in the send payload. That is a DESIGN change, not a bug fix: the
composer's current design deliberately refuses images (the notice says an
image goes through the document surface to Perception). The owner approved
drafting the plan, and it is now DRAFTED and gated — see
[[composer-clipboard-paste-plan-drafted-2026-11-08]]. The plan's transport
decision changed twice on the owner's steers: the first draft carried the
image as a written path (the 65,536-byte request cap could not carry it), then
the owner ruled the cap should rise to allow screenshots and then that an
artificial cap makes no sense with today's context windows — so the cap is
removed and the image rides as bytes. Related:
[[gui-client-verified-by-a-vm-harness]]
(the face's verification paths), [[measuring-a-mockup-by-reading-the-png]]
(the measure-and-read discipline this followed),
[[hybrid-retrieval-delivered-2026-11-08]] (the app this was measured in).
