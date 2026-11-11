---
name: the-send-button-with-a-queue-ki-495
description: "KI-495 fixed in 0.6.7 — the composer's send button did nothing with a message queued (idle press with empty composer returned; Stop no-oped without a process card); shipped through the updater itself"
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

Owner report 2026-11-09: "when a message is queued the send button doesn't
work." Two faults under one control, both SILENT, both in
`src/gui/assets/app.js`:

1. **The idle press.** With nothing typed and a message waiting, the click ran
   `send()`, which — queue present, composer empty — took its queue branch,
   pushed the empty string nowhere and returned. No run, no queue change, no
   line anywhere. A queue can outlive the run that made it (a state read says
   idle while the queue still waits), so that press was the only way to move
   it. Fix: idle + empty composer + a waiting queue → `flushQueue()` (a run
   working in another conversation still keeps the queue waiting).
2. **The dead Stop.** A face attached to a running chat (a reload, a chat
   opened mid-run) learns `running` from the state read but has NO process card
   until a process event lands. `stopRun`'s guard returned whenever the card
   was absent/inactive — a no-op under a live run. Fix: it fires whenever the
   face believes a run is in flight; the card only decides what the reply
   paints.

**Why it matters:** both are the owner's-own-hands shape — the screen offers a
control that does nothing and says nothing. Red-proof first (with the change
stashed the harness failed: no `/api/run`, no `/api/stop`, queue still
`queued · 1`); fixed, 1036 checks / 125 cases; gate green; **v0.6.7 cut and
installed by `fleet update` itself** (0.6.6 → 0.6.7, `fleet.prev` kept) and
serving on 3080. Recapped at `docs/recaps/2026-11-09-the-send-button.md`.

**How to apply:** by design the button reads **Stop** while a run is in flight
and Enter queues — changing that is a product call, not a bug. The desktop
bundle lagged at 0.6.6 (one About press moves it). See
[[updater-fix-landed-2026-11-09]].
