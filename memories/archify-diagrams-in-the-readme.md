---
name: archify-diagrams-in-the-readme
description: "reference 2026-11-16 — how fleet's README architecture and turn drawings are made: Archify JSON authored from the repo at a pinned revision with file:line SRC badges, gated by `archify finalize`, HTML in docs/architecture/ and PNGs linked from the images"
type: reference
date: 2026-11-16
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The README's "A look" section carries two diagrams, both
[Archify](https://github.com/tt-a1i/archify) (`~/.zcode/skills/archify`,
v3.0.1 installed; 3.2.0 was available and NOT installed on my own
initiative):

- `docs/architecture/fleet-architecture.html` ← `docs/images/architecture-fleet.png`
  — 16 components: owner → console / native shell / paired phone → face
  server → agent core → router → providers and the house lanes, with tools
  and the plugin host beside the core; above, the home's stores (memory,
  session log + archive, calibration stream, and the home's own vault /
  boards / roles / cleanup); below, the machine (layerfs worker, cron,
  health, the resource contract). 18 connections, two regions ("the home —
  files you can read", "one Rust binary — every face serves it"), four
  cards.
- `docs/architecture/fleet-turn.html` ← `docs/images/process-turn.png`
  — the **sequence** (8 participants: owner, face, agent core, router,
  provider, tools, approval, calibration), not a workflow: assemble → judge
  and walk (the head's sha256, the cache read) → act (gated, folded at 85%)
  → answer and record.

Both are `--quality showcase`, pinned to `meta.repository` at a 40-char
revision with `link_mode: "local-only"`, and every node carries `sources`
`file:line` that render as the `SRC` badges. **The authored repository URL
must match the local `origin`** — since 2026-11-16 that is
`https://gitea-gitea.opsh.io/synapticmomentum/fleet.git`, not the github
release remote; the evidence gate refuses a mismatch by name.
Candidates live under `.archify/<type>-<slug>-<stamp>/candidate.json`,
which is a tracked `.gitignore` rule (`/.archify/`, commit `4448d4e`).

**Delivery:** the 2026-11-02 originals shipped as `de2d42b` (12
components, 7 participants). The 2026-11-16 refresh pins `d4ad779` (the
0.8.9 cut plus the cache-record correction): four tracked artifacts —
`docs/architecture/{fleet-architecture,fleet-turn}.html` (~750 KB each,
self-contained) and `docs/images/{architecture-fleet,process-turn}.png`.
The architecture PNG is a **header-and-stage crop** (1980x1320) because a
candidate with boundaries makes the node index a single-column grouped
list and the full page 2077 px tall; the turn PNG is the full page
(2880x3184).

**How to apply:** author the candidate, finalize once, then copy the HTML
over `docs/architecture/` and shoot the PNG. The capture path used since
the playwright `tmp/diagram-shoot.js` disappeared: `obscura serve
--allow-file-access --allow-private-network`, then a CDP script on the
browser socket (`Target.createTarget` + `Target.attachToTarget` with
`flatten: true`), `Emulation.setDeviceMetricsOverride` at 1440 wide and
`deviceScaleFactor: 2`, `Page.captureScreenshot`. The stage is a 960 px
column at 1440; `magick -crop` trims to the title and stage.

**Gotchas learned:**
- Architecture `connections` take no `role` field (workflow edges do).
- A boundary label near a route trips `composition/container-border-run`
  when an edge runs along the border; give the edge truthful sides or drop
  the boundary.
- A passing finalize reports `visualReviewRecommendation.signals.resolvedCrossovers`.
  The protocol is ONE positions/sizes-only reflow and one rerun with
  `--out-dir <folder>/review-2`; accept a lower count, restore the copy and
  go to review-3 if it fails or gets worse. The 2026-11-16 refresh went
  5 → 3 and stopped there.
- A re-finalize with an existing `*.browser-check.json` is refused
  ("evidence-path-conflict"); pass `--out-dir <folder>/review-N`.
- Sequence limits: `segments` take only from/to/label; messages need ≥28px
  spacing and y ≥ 160; the viewBox width must be ≥ the participants' need
  (~105 px each) and ≤ **945** for a 6.1px sublabel to project above 6px at
  the 930px desktop budget; content must end at least 98px above the
  viewBox height (the legend); and the ratio must stay ≥1.55 or the desktop
  reader cannot narrow it. One-word segment labels — long ones draw under
  the participant boxes.
- A **workflow** (lane × column grid) could not route the turn's
  tool/model fork back to the agent without crossings after four attempts —
  a sequence draws a fork-and-return naturally; pick the diagram by the
  shape of the answer, not by "process".
