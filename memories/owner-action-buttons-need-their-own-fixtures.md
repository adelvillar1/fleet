---
name: owner-action-buttons-need-their-own-fixtures
description: "An idle-fixture button sweep proves nothing about owner-action surfaces — the approval seat (two hosts), the inbox's five classes with their row menus, the service-repair verbs and the frame action bar each need their own fixture, and a fixture missing the server's row key makes a working menu look dead."
type: project
date: 2026-11-17
basis: inferred
tier: durable
shape: [rendering, drops-the-row, resolved]
phrases: [approval seat decisions once always deny remember, "both seat hosts #seat-actions #design-seat", inbox row key is class:id from inbox_rows, "row menu ] read as dead on a keyless fixture", window.open recorded like clipboardWrites, repair verbs only when a service is down]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

The owner asked whether the dead-button review covered buttons for items needing *their* approval — surfaces that normally would not appear. Honest answer: the first pass did not.

**The trap.** Both sweeps booted a lean `emptyState` with `approval: null` and an empty inbox. So every owner-action surface rendered nothing, and the sweep "proved" 373 lens buttons and 40 settings buttons clean while never touching the class the owner cares about most. A sweep over one idle state says nothing about the surfaces that appear when work is waiting.

**The class, enumerated by grep rather than guessed:**
- the approval seat — `DECISIONS` = once / always / deny / remember, in TWO hosts (`#seat-actions` over the composer, `#design-seat` on the canvas, same state, added for the KI-380 class)
- the inbox's five row classes (`INBOX_CLASSES`: seat, onboard, ask, cron, doctor), each with row verbs *and* a `⋯` row menu
- the service-repair verbs when a service is down: `Repair`, `Install kiri and start the container`, `Rebuild the lanes`
- the design frame's action bar: tweaks / review / versions / export / copy / duplicate / open / delete, plus `fix the drift` on an incoherent set

**Two harness blind spots surfaced only because these buttons were driven:**
1. **`⋯` menus looked dead with a bad fixture.** `bridge.rs::inbox_rows` stamps `key = "{class}:{id}"` and a signature on every row; a fixture without keys makes `face.inboxOpen === row.key` compare `null` to `undefined`, so the menu never opens. The fixture was wrong, not the button — I nearly filed this as a defect.
2. **`window.open` was discarded by the stub.** `open the frame` leaves no other trace, so it read as dead. The stub now records `windowOpens`, exactly as it records `clipboardWrites`, and the fingerprint counts both.

**A pin that could not fail:** the first seat pin pressed `rows('button').find(...)`, which returns the *first* match — so breaking the design-seat copy left it green. Now each host is driven by selector (`#seat-actions button`, `#design-seat button`), and breaking one turns exactly those three checks red.

Counts: 29 inbox buttons, 3 repair verbs, 9 frame acts, 6 seat decisions across both hosts — all live. Permanent case `everyButtonOnAnOwnerActionSurfaceDoesSomething`, red-proofed per host and per menu.

KI-565 names what is still **not** driven, rather than implying full coverage: a failing lane's own repair card, a refused write's retry, a session row's rewind/restore, and the huddle room's awaiting-you verbs.
