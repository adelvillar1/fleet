---
name: the-drawer-is-a-pane-tab-and-its-main-component
description: "The drawer is now a pane tab and the pane's main component — an inspected object opens the pane on `detail` with the run's view standing down, instead of the old card that hid every tab to make room for itself."
type: change
date: 2026-11-18
basis: confirmed-by
tier: durable
shape: [rendering, drops-the-row, resolved]
phrases: [the collapsible pane should be part of the drawer, "main component of it, otherwise the drawer serves no real purpose", tab-detail, pane-detail, data-object-card open pane-tabs display none]
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source:
---

The owner's 2026-11-18 ruling: "the collapsible pane should be part of the drawer and it should be main component of it, otherwise the drawer serves no real purpose."

WHAT WAS WRONG, diagnosed from the code. `openDrawer` rendered into `#dock-object` — a card stacked ABOVE the pane's tab strip — and `body[data-object-card="open"] .pane-tabs, body[data-object-card="open"] .pane-body { display: none }` hid every tab and every other panel to make room for it. So opening an object blanked the pane and left the drawer as the only thing on a surface it had just emptied. That is exactly "serves no real purpose": 47 callers used the drawer and the pane's own views were suppressed every time.

THE FIX. The drawer is now a PANE TAB, and the pane's main component:
- `index.html` gains a `tab-detail` button (leading the strip, right after `tab-context`) and a `pane-detail` panel holding the `dock-object` card. The card MOVED inside the panel rather than sitting as a sibling of the tab bodies.
- `openDrawer(title, sections)` calls `openPane()`, fills `dock-object-title`/`dock-object-body`, reveals `tab-detail`, then `showPaneTab('detail')`. The fixed `aside#drawer` remains only as the narrow fallback below the pane breakpoint.
- `showPaneTab` learns `detail`: it toggles the tab and panel, refuses the tab when nothing is open (a tab that leads nowhere is a door to a blank, so it falls back to `run`), and gained a `remember` flag.
- `closeDrawer` clears the body, retires `tab-detail`, and returns to `run` — but ONLY when the detail tab was the one showing (`if (face.paneTab === 'detail') showPaneTab('run', false)`), and never persisting that move.
- The CSS rule that blanked `.pane-tabs`/`.pane-body` is DELETED, replaced by `#pane-detail` taking the ordinary `.pane-body` column.
- `face.objectTitle` added to the store (it was being written but never declared).

THE BUG THE FIX INTRODUCED, caught by an existing case: `closeDrawer()` runs on EVERY route change (`showRoute` calls it), so an unconditional `showPaneTab('run', false)` reset the owner's restored pane tab on each arrival — `RE the pane returns at its tab` failed with `files=false run=true`. The guard on `face.paneTab === 'detail'` is what makes the reload-reattach preference survive.

A TEST TRAP: `dock-object.hidden` was the old signal for "an object is open". The card no longer hides — its PANEL does. Two cases were updated to assert `pane-detail.hidden`/`tab-detail.hidden`/`aria-selected` instead, and the "opens" case now also asserts `pane-run.hidden === true`, which is what actually proves the drawer is the pane's main component rather than a stack beside it.

Verified live on a scratch home: the served HTML carries `tab-detail`, `pane-detail`, the `dock-object` nested inside it, and no blanking rule. Gate: 2354 Rust tests, 1435 face checks across 200 cases, fmt clean. Assertions proved to bite by deleting the `showPaneTab('detail')` call, which fails the open case with `detail=true selected=false`.
