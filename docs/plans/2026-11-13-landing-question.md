---
title: the landing is the question — "What do you want to work on?"
date: 2026-11-13
status: completed (2026-11-13; the landing is the question - pinned by the reworked OB/N1 cases and the deleted LAND family; /api/home and bridge.home() retired with their last consumer; the gate green)
---

# The landing is the question

The owner's ruling (2026-11-13): the face's landing keeps only
"What do you want to work on?" — everything else adds unnecessary
cognitive load. The landing (`#empty`) carried eleven surfaces of
explanation (eyebrow, lede, greet, machine stats, since, next-from-graph,
method strip, lanes, offers, recent, meta) over one question.

The surviving landing is the h1 (now the question) plus the onboard door
and its own flow — the door is functional first-run machinery, not
landing chrome. The graph, stats, offers and history live on their own
surfaces, one pivot away.

## Acceptance criteria

- [x] C1: `#empty` carries the h1 ("What do you want to work on?"), the
  onboard door whole, and `#onboard-cards` (the door's post-door flow) —
  nothing else.
- [x] C2: the painters and wiring retire (`paintGreet`, `paintRecent`,
  `paintHome`, `paintOffers`, `paintSince`, the home-stats/next painters,
  the method/lane wiring, the copy-override eyebrow/lede lines);
  `paintEmpty` keeps only its `#rail-where` duty; the el map drops every
  removed id; `readHome` and the `fleet-last-visit` writes go; the boot
  pairing-refusal path re-points to the h1.
- [x] C3: the landing-only CSS blocks are gone; the shared classes stay
  (`.offers*` for the door and later panel, `.empty-eyebrow/.empty h1/
  .empty p/.empty-meta` for pwWelcome and the huddles card, `.home-stat*`
  for the skills board, `.method/.method-label` for the door's
  disclosure, all `.onboard-*`).
- [x] C4: `readHome` loses every consumer — the `GET /api/home` route,
  `bridge.home()`, and its Rust test retire; surfaces.md's landing
  section reads the question.
- [x] C5: the harness — `LAND the landing greets` deleted; `N1 the home
  is always reachable` and `OB the door is the welcome` reworked; the
  door/capability/strategy/judgment cases stay.
- [x] C6: U49 in the UI ledger; the gate green; commits on master, push;
  local deploy (rm-then-cp → face relaunch); no release steps.

## Verification

The harness and gate, then live: launch → the landing is the question
over the composer; the first-run door still drives key setup and its
capability/strategy steps; the rail header still names the workspace.
