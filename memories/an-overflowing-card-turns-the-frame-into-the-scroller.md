---
name: an-overflowing-card-turns-the-frame-into-the-scroller
description: "feedback 2026-10-30 (owner-reported) — an object card taller than its bounded pane painted past it, making `.face` (overflow hidden) the scrollable ancestor that openDrawer's scrollIntoView then dragged — the top bar left the viewport when a memory's card opened; the fix is the card scrolling ITSELF"
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

Owner report 2026-10-30: *"the memories page swallows the top bar when a
memory is clicked and the side pane opens."* Diagnosis (measured, not
guessed): the object card (~1,460 px) is taller than the pane's bounded
grid row (~840 px); `.pane` has `overflow: visible`, so the content
painted past it and **`.face` — `overflow: hidden` — became the
scrollable ancestor**, holding scrollTop 109. `openDrawer`'s
`scrollIntoView({block:'nearest'})` then scrolled THAT ancestor, dragging
the top bar out of the viewport. `window.scrollY` stayed 0 — the scroll
happened inside a clipped ancestor, invisible to every scroll-position
check aimed at the window.

**Why:** an `overflow: hidden` box still scrolls programmatically (and
by keyboard/focus); hidden overflow does not mean unscrollable. Content
that exceeds a bounded flex/grid child must be given its own scroller or
it silently promotes an ancestor to the role.

**The fix:** the open card scrolls itself —
`body[data-object-card="open"] .dock-object { overflow-y: auto;
min-height: 0; flex: 1 1 auto; }` — so the pane never overflows, the
grip stays put, and scrollIntoView's nearest scrollable ancestor is the
card itself. Every object card on every surface benefits (committed
`9fe5f60`).

**How to apply:** when a pane/panel is height-bounded, any card that can
exceed it needs its own `overflow-y: auto` + `min-height: 0`; verify
with a rect measurement of the chrome elements (`top.t === 0`) after
opening a card, not by eye and not by `window.scrollY`.

Related: [[workspace-history-rail-plan-2026-10-30]],
[[memory-rail-plan-and-the-substrate]], [[a-layout-claim-is-measured-not-asserted]].
