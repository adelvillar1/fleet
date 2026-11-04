---
name: docs-gallery-refresh-2026-10-30
description: "project 2026-10-30 — the documentation refresh after the four rail revamps: all 13 gallery images re-shot with Playwright from the live face (they predated even the settings/bots revamps), README names the idiom, surfaces.md's four stale passages rewritten; commits 6699961 + 1274634"
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

Owner direction 2026-10-30: *"make sure that all documentation is updated
including the README.md file with the corresponding new screenshots related
to the new rail-enabled pages. use playwright."* Landed as `6699961` (the
gallery + README) and `1274634` (surfaces.md).

**What the check found:** the README gallery's twelve surface screenshots
all dated 2026-10-27 — stale for SIX pages, not the four from that day
(settings and bots had been revamped 2026-10-29 too). All thirteen images
(`docs/images/surface-*.jpg` + `settings-tools.png`, which
`docs/features/tools.md` embeds) were re-shot in one Playwright pass at
1440×900 from the live face, each rail page at its arrival section, four
key shots read as the visual review. The functional spec's picker images
were left alone — the picker did not move.

**README.md:** the gallery intro now names the idiom the long pages share
(rail → sections → one-line rows → the pane), and the skills bullet
carries the category rail + the approve's home-side landing. A stale-claims
sweep across README, both specs, and every feature doc found nothing else.

**`docs/architecture/surfaces.md`** was the one doc the revamps had not
reached — its per-surface What/What-you-do/Why/State passages for memory,
skills, workspace history, and calibration described the pre-rail shapes
and stale counts (24 facts → 114 live/34 retired/13 archived; 442 → 449
skills). All four rewritten in its own voice with today's measured states.

**How to apply:** a surface revamp's doc pass is not done until the
GALLERY is re-shot (screenshots are documentation too, and they age like
prose) and `docs/architecture/surfaces.md` is checked — the per-surface
atlas is easy to miss because no plan lists it as a linked artifact.

Related: [[session-recap-2026-10-30-knowledge-group-on-the-rail]],
[[deliverables-for-github-live-in-the-repo]],
[[the-screenshot-is-the-visual-review]],
[[fleet-docs-revamp-direction]].
