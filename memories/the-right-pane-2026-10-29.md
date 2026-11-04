---
name: the-right-pane-2026-10-29
description: "project 2026-10-29 — the right pane is ONE surface now (run/files/ preview/web tabs, one topbar icon, the dock retired with its four cards relocated), a grip owns its width (300 floor, persist, dblclick reset, arrow keys), and the web tab renders addresses live behind a framing check; d20c257, harness 344/37, Chromium-measured"
type: project
date: 2026-11-04
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

2026-10-29 the owner reported (with a topbar screenshot): two icons open
different versions of the right pane, the pane needs deep research on what a
harness right pane should be, a drag component for width, and preview +
browser capability "like zcode and deepseek harness have". Root cause of the
confusion was structural: the dock (`aside#dock`, the run's process + object
cards) and the pane (`aside#pane`, files/preview/web) SHARED grid column 3
with `body[data-pane="open"] .dock { display: none }` — each icon swapped
what lived in the one right slot.

**Landed as d20c257** (plan docs/plans/2026-10-29-the-right-pane.md,
completed):

- **P0 one surface**: the pane's tabs are now run/files/preview/web — the
  dock's FOUR cards moved in (process → the run tab body; dock-object → a
  card above the tab bodies, shown on object presses; face-card "this
  session" and trust-card → beneath the process card in the run body). The
  dock aside, `aside-toggle`, and all `data-aside`/`data-dock` machinery
  retired, INCLUDING the rail auto-fold that existed only to make the dock
  room (setAside/dockMode/syncDock/railBeforeDock/asideBeforeObject all
  gone). `openPane()` shows the remembered tab (default **run**);
  `openDrawer` above 1180px = openPane + the object card; below, the fixed
  drawer keeps the job. The pane's width rides `--pane-width` in the grid.
- **P1 the grip**: the pane's left edge is `role="separator"` `#pane-grip`
  — pointer drag writes `--pane-width` clamped [300, min(720,
  innerWidth−560)], persists `localStorage["fleet-pane-width"]`, double-click
  resets 440, arrow keys step ±24. Width state is self-tracked in an
  `applied` closure (the stub's getBoundingClientRect is a lie, and the var
  is the source of truth anyway). During a drag `body[data-pane-dragging]`
  stands the iframe's pointer events down.
- **P2 the browser**: `web.rs framable(url)` walks redirects reading ONLY
  headers — `x-frame-options` (DENY/SAMEORIGIN) and CSP
  `frame-ancestors` (no `*`/scheme token), body never read — bridge route
  `GET /api/web/frame`. Framable → the face renders a sandboxed iframe
  (`allow-scripts allow-same-origin allow-forms allow-popups`, NO
  allow-top-navigation so the page can't escape the pane, referrerpolicy
  no-referrer) with a back control over the tab's own history; refused → the
  refusal names the site's own header and offers the prose reader
  (`/api/web`) and open-in-owner's-browser doors.

**Research** (subagent, in the plan): the harness-pane canon is process
visibility, files/diffs with accept-reject, checkpoints, live preview,
resizable pane, steering — fleet's design matched; its two real gaps vs the
peers are named follow-ups **P3 (a Changes section — the diff review as a
pane section) and P4 (a plan card — plan mode's document visible in the
pane)**, deliberately not built.

**Measured, not asserted**: `tmp/pane-width-probe.mjs` (Chromium, real
stylesheet) proved the grid honors the var at 440/720/300 and the grip takes
the pointer — see [[a-layout-claim-is-measured-not-asserted]]. Harness green
at 344 checks / 37 cases (new case: run tab + object card + grip
drag/persist/reset/keys + frame/refusal doors); the gate ran whole.

Surgery lesson worth keeping: the dock held TWO more cards than its
comment named (face-card, trust-card) — the first relocation sliced mid-dock
and left stranded cards plus a stray `</aside>`; inventory every child of a
container you are dissolving before cutting it.

**Follow-up defect, fixed 2026-10-30 (KI-373, 1ce38a7):** the dock element
carried `class="dock rail"`, so its cards took `.rail .card`'s 16 px padding,
hairline, radius and ground. Moving them into the pane dropped that ancestor —
the run tab's cards rendered with ZERO padding, flush to the frame and under
the grip (Chromium: card left 1001 vs pane 1000, padding-left 0px). Fixed by
declaring the box for both homes (`.rail .card, .pane .card`) plus
`#pane-run { padding: 8px 12px; gap: 10px }`; verified card 1013–1428 inside
pane 1000–1440 with the grip ending at 1006, and no text leaf in any of the
four tabs starting left of the grip. The general lesson is its own memory:
[[a-relocated-child-loses-the-ancestor-that-styled-it]]. Harness 395/43.
