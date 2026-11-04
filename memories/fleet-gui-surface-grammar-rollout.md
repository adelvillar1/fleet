---
name: fleet-gui-surface-grammar-rollout
description: "owner 2026-10-20/21 — the Atlas surface grammar applied to every remaining page; ALL SIX done (memory/skills, sessions, router, delegation, bots, calibration, settings) and #/tower retired with its feed moved to the Atlas. Bots/calibration inner column+rail composition is the open tail; attach-files shipped, a per-chat working folder is designed but NOT built"
type: project
date: 2026-11-04
basis: confirmed-by
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

**The standing pattern, as the owner asked for it repeatedly** ("let's follow
the same revamp pattern for bots, settings, and calibration", then "let's keep
going"): every surface takes the grammar proven by the Atlas
([[fleet-second-brain-visual-redesign-awaiting-pick]]) — a `surface-head` with a
compact instrument, a `surface-strip` of figures, a dense left table
(`.surface-column`, span 8) whose rows carry their own actions, and a
`.surface-rail` (span 4) of secondary cards. Composition, not rewrite: existing
charts, filters and reads stay.

**Shipped under this rule:** memory + skills (`e7e36c2`,
[[fleet-memory-and-skills-knowledge-tables]]); sessions (`f6593d6`) and router +
delegation (`7353073`) ([[fleet-sessions-archive-groups-by-project]]); the
composer's run bar (`48cd01e`,
[[fleet-composer-needs-permissions-and-a-route-pin]]); **tower retired
(`05fd1ec`)**; bots, calibration and settings (`30788ba`); **attachments
(`f51906c`)**.

**Tower retirement — the resolved answer.** The owner said "given that second
brain is fantastic it seems like the tower page is redundant." Mostly true: the
Atlas covers projects, attention, conflicts, pulse and coverage; only three
facts were unique. The report feed (newest twelve filings) and each harness's
trust-precision line moved into the Atlas as "the report feed" card below the
commit ribbon; the directory scan was superseded and dropped; `#/tower` now
falls back to the conversation. **The reusable pattern:** before retiring a
surface, enumerate the facts that live only there, move the ones worth keeping,
and name which one was superseded rather than silently dropping it. Leftover:
the tower's renderer helpers (`readTower`/`paintTower`/`twoSeries`) are still in
`app.js`, unreachable, because `projectDrawer` inside that block is shared with
the related index.

**Bots, calibration, settings (`30788ba`) — head+strip only.** Bots kept its
start/stop, token seal, door and pairing rows; calibration kept its charts and
gate review; settings kept its section index and every catalog-driven form
([[fleet-config-surface-is-the-settings-page]],
[[no-form-asks-for-a-value-fleet-already-has]]). **The inner column+rail half
of the plan was NOT applied to bots and calibration** — their cards still stack
below the strip — so the row-level actions remain the open tail.

**The trap from that pass, worth carrying:** swapping a tile strip for a
`surfaceStrip` deleted a local (`waitingSeats`) that was still used later in the
same function — the page threw and rendered **zero cards** while every other
check looked fine. After any head/strip swap, load the page and assert the card
count is non-zero, not just that the title paints.

**Attachments (`f51906c`).** The composer's Attach control reads text files in
the browser (five max, 256 KB each, 512 KB together), shows removable chips, and
frames them ahead of the task in the run body; **the server owns the bounds and
refuses a name that is a path**, and the framed text is what the session log
records the owner asking — an attachment is evidence in the conversation, not
hidden context.

**Working folder — built, but not by me.** After the plan was committed I did
not implement it; a **concurrent session** committed `f554a59` "a working folder
per chat" (same git identity, different authoring session) and flipped the plan
to `done`, noting in its body that my empty-state fix `543ff95` had swept some
of its `index.html` markup. The owner then asked **"are you executing the plan
without my approval?"** — the right answer was no, with the evidence: `git show
-s --format='author/date/subject' f554a59` names the session and time, and the
implementation's scope (run bar third control, `<FLEET_HOME>/sessions-workdirs.json`,
tool root, refusals, checkpoint capture, subagent inheritance) matched the plan
but was not mine. **Lesson: with two sessions sharing one git identity, "who
wrote this" is answered by the commit body's own disclosures and the file
mtime trail — and never by assuming your own history.** The one correction that
is mine: the plan's terminal status is `completed`, not `done`.

**Order, done:** tower → bots → calibration → settings → attach → (working
folder open).

**Two open plans now cover the rest of the composer's work**, both drafted
2026-10-21 after the owner's correction that plans are repo files gated by
`dev-decisions` ([[plan-is-a-file-not-prose]]) and both `plan-gate` PASS at
12/12: `docs/plans/2026-10-21-working-folder.md` (`4077099`,
[[fleet-chat-working-folder-plan]]) and
`docs/plans/2026-10-21-document-perception.md` (`d1d5267`,
[[fleet-document-perception-plan]]). Neither is built. The rule to carry: say
"planned, not built" with the commit sha, never let a committed plan read as
delivery ([[a-green-gate-covers-what-was-built-not-what-was-left-out]]).

Related: [[plan-brainstorming-before-development]],
[[fleet-second-brain-is-a-tool-plugin]], [[rebuild-is-not-commit-push-or-ship]],
[[a-compacted-transcript-reads-as-lost-work]].
