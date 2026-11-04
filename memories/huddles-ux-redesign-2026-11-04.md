---
name: huddles-ux-redesign-2026-11-04
description: "project 2026-11-04 — the owner called the shipped huddle UI "horrible" and asked for a Slack-level interface in the app's own rail grammar; the plan is docs/plans/2026-11-04-huddles-ux.md (C1–C14), Phase 0's five mockups are drawn, measured and committed (a60713b), and the code phases await the owner's read"
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

The huddle MODEL was complete ([[huddle-plan-completed-2026-11-04]]); the
SURFACE was scaffolding. The owner's verdict: *"What the previous model did
can only be called horrible… plan first and leverage the design skills you
have to get us to a slack level of ux/ui"* — with one constraint, *"remain
consistent with the rail experience we have in sessions and throughout the
entire app."*

**What the audit of the shipped UI found** (all four are real scaffolding
defects, not taste):

- `#huddles-rail` is a 188px `<nav>` bound in `app.js:666` and **never
  filled** — the split layout reserves a gutter for nothing.
- `.board` is a 12-column grid but `.hsection` has no `grid-column` rule,
  so every section and the new-room button auto-place into one ~93px track
  while `.hrow-name` demands 210px.
- `.press`, `.dot`, and `.chip.work|wait|idle|ended` have **no CSS rules
  anywhere**, so the primary button renders as a UA button and every state
  reads identically.
- A row press navigates to `#/sessions/<id>` — opening a huddle **drops the
  owner into Sessions**. There is no rooms rail, no last-line preview, no
  presence, no place for the deliverables.

**The design the plan locks** (direction recommended, the picker returned
no answer, so nothing is labelled user-approved):

1. **The split** — `#/huddles` is the surface: a 320px **rooms column** (the
   work column's exact mechanics: absolute, `--work-width`, stage margin,
   48px folded strip, sheet at ≤1180), the room, the app's own right pane.
   A row on `.work-row`: 3px state bar, member stack, name, the **last line**
   (`designer: the hero copy is tightened…`), time, `needs you` badge,
   `2 files · 3 facts`.
2. **One room head** (not two): serif name + state chip + `break out ·
   distil · archive`, the goal as a quiet line, members with voices and a
   presence ring on the speaker, then the **deliverable shelf** — artifact
   cards whose press opens the pane's preview.
3. **The transcript**: an assistant turn gets the member's mark in the
   gutter and `designer · 14:02`; a mention is a system line
   (`you → scholar · review requested`); a deliverable is a `.card-row`
   artifact card; a capability gap is an inline proposal.
4. **The composer** keeps its mechanics, gains a `to` chip (`to designer`,
   else `to the room`) and a send that names it.
5. **The seat stays INLINE above the composer** — the console has never used
   a modal for it (amended during Phase 0, when the mockup drew one).
6. Threads/reactions/typing dots stay OUT: model work, a separate plan.

**Seams the build needs:** `/api/huddles` rows gain `preview` + `from` (the
last owner/assistant text, read in the fold the endpoint already walks);
`#/huddles/<id>` must keep the huddles panel and the rooms column.

**Phase 0 is committed (a60713b):** five screens at
`docs/plans/mockups/2026-11-04-huddles-ux/` (rooms, room folded, deliverable
with the pane open, empty, seat) plus `rooms-console.png`, rendered at
1440×968 and measured over the DevTools protocol in all four themes —
0 overflow, 0 clipping, rows 72.8px, rail rows 34px, mark ink 5.7–7.0:1
(the first pass caught a 4.3:1 lane-0 mark in `contrast`; the ink mix moved
from 82% to 68% tint). Details: [[measuring-a-mockup-by-reading-the-png]].

**State at the close of the turn:** the code phases (1–5) are NOT started.
The mockups were handed back for the owner's read as the gate, with the
offer to redraw rather than build. **The owner read them and loved it**
("love it!"), then deferred the code to an icon pass first — every icon on
every rail, plus the rail toggle reduced to its chevron alone
([[every-rail-icon-audited-2026-11-04]]). Next move: land the icons plan,
then return here and build P1→P5 (seams, rooms rail, room head/shelf/
transcript, states, then harness cases + docs + `design-system.md` handoff
packet + deploy + dmg + push).
