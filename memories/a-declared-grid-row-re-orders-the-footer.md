---
name: a-declared-grid-row-re-orders-the-footer
description: "feedback 2026-10-23 (KI-302) — giving one child an explicit grid row made CSS grid auto-place every later item from row 1, so a hidden head left the composer at the top of the conversation; a column of head + growing content + pinned footer is a flex stack, and the owner found it before any test did"
type: feedback
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

The owner: *"the composer as soon as you go to the main conversation page shows
pinned to the top, but for consistency it should be at the bottom."* It was: I
had added a `.conv-head` and given the content `.empty, .log { grid-row: 2 }`
so a hidden head would not hand the `1fr` track to whatever came first. CSS
grid packs **auto-placed** items starting at row 1, so with the head `hidden`
(hidden children generate no boxes) the composer, the process strip, the seat
and the action cards were all placed *above* the content's row. The mark-up
order was right the whole time; the layout stopped reading it.

**Why:** pinning one child to an explicit track changes the flow for every
other child, and a `display: none` sibling silently changes which row "first"
means. The failure only appears in the state the owner meets first — the front
door, where the head is hidden — while an open conversation looked right, which
is why every check I had passed.

**How to apply:** when a column is *head, content that grows, footer that keeps
its height*, write it as a flex column — `flex: 0 0 auto` on the ends,
`flex: 1 1 auto; min-height: 0` on the content — so DOM order is the layout and
no sibling's visibility can move the others. Use explicit grid rows only when
every item in the grid is placed by row, and then give *all* of them a row.
Verify geometry by measurement, not by reading CSS: the pin is the stylesheet's
rules in the face harness
([[gui-client-verified-by-a-vm-harness]]) **plus** a live `getBoundingClientRect`
pass on the state the owner reported (here: composer bottom === panel bottom,
gap 0, at both 1440 and 800, on both the front door and an open conversation).
A layout bug the owner can see is worth a KI row with its mechanism, as
[[defects-the-work-closes-belong-in-the-ledger]] asks.
