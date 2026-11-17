---
name: settings-writes-hand-the-caller-the-write-reply
description: "The settings surface's silent-feedback class: writeSettings handed onDone whatever readSettings resolved (undefined), so every reply-reading caller took its empty path; fixed by returning the document, passing the write reply through, and giving the provider card its own remembered answer line."
type: project
date: 2026-11-17
basis: inferred
tier: durable
shape: [rendering, drops-the-row, resolved]
phrases: [writeSettings onDone reply, readSettings resolves undefined, settings-status lives in the board column, provider-answer role alert, face.providerAnswer remembered per provider, a mock answering empty cannot tell success from silence]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

KI-561's root cause: `writeSettings` handed its `onDone` the result of `readSettings(true)`, and `readSettings` resolved `undefined` (its fetch chain assigned `face.settings` and returned nothing). So every reply-reading caller in the settings surface silently took its empty path — the provider reload always said "the reload did not answer", and the plugin install/promote handlers did the same.

The fix, in `src/gui/assets/app.js`:
- `readSettings` returns `doc` from its final `.then`.
- `writeSettings` re-reads first, then calls `onDone(reply)` with the WRITE's reply (`{status, body}`, so `body.wrote` is readable). A non-200 returns before `onDone`, because `onDone` is the caller's success path — running it painted "saved" over the writer's refusal and the harness's U1 case caught exactly that.
- The provider card owns an answer line (`data-role="provider-answer"`, `role=alert`) rendered beside the press, because `el.settingsStatus` lives in the settings board column while the card renders into the pane's `dockObjectBody` — two different columns.
- The answer is remembered in `face.providerAnswer` keyed by provider id, because a successful reload repaints the card from the home's own read; a sentence held only in a local node is wiped by the very repaint that shows the new catalog.
- The press disables itself and says `reloading…` before the request, with a re-entrancy guard.

The pin that matters: four face checks assert the answer line exists, a scripted success reports the listing's own count, a refusal names the writer's words in the card, and the press is not left disabled. Red-proof: reverting only the `onDone(reply)` plumbing reproduces `the reload did not answer` on both paths. A mock that answers `{}` cannot tell a success path from a silent one, so the success case scripts a real reply.

KI-562 files the reload-all card's version of the same class, still open.
