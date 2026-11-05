---
name: archify-diagrams-in-the-readme
description: "reference 2026-11-02 — how fleet's README architecture and turn drawings are made: Archify JSON authored from the repo at a pinned revision with file:line SRC badges, gated by `archify finalize`, HTML in docs/architecture/ and full-page PNGs linked from the images"
type: reference
date: 2026-11-05
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
recall-count: 0
superseded-by:
valid-until:
---

The README's "How it is built" section carries two diagrams, both
[Archify](https://github.com/tt-a1i/archify) (`~/.zcode/skills/archify`,
v3.0.1 installed; 3.2.0 was available and NOT installed on my own
initiative):

- `docs/architecture/fleet-architecture.html` ← `docs/images/architecture-fleet.png`
  — 12 components: owner → console → face server → agent core → router →
  lanes, with canvas, memory, vault above and cron / session log / tool
  surface below.
- `docs/architecture/fleet-turn.html` ← `docs/images/process-turn.png`
  — a **sequence** (7 participants), not a workflow: assemble → sys1
  judgment → chain walk → approval seat → tool call → stream + record.

Both are `--quality showcase`, pinned to `meta.repository` at a 40-char
revision with `link_mode: "local-only"` (the origin is SSH), and every
node carries `sources` `file:line` that render as the `SRC` badges.
Candidates live under `.archify/<type>-<slug>-<stamp>/candidate.json`,
  which is now a tracked `.gitignore` rule (`/.archify/`, commit `4448d4e`,
  beside `/.runs/ /​.job-artifacts/ /​.corpus-refresh/ /​services/browser/generated/`
  — the dirs the binary writes wherever it runs).

**Delivery:** shipped 2026-11-02 as `de2d42b` (on top of the gallery's
`572852b`), with `origin/master` in sync. The four artifacts are
tracked: `docs/architecture/{fleet-architecture,fleet-turn}.html` (~750 KB
each, self-contained) and `docs/images/{architecture-fleet,process-turn}.png`
(~400 KB each). The owner followed with "commit and push to remote" when
nothing of mine was left uncommitted — state the delivery status in the
closing line ([[rebuild-is-not-commit-push-or-ship]]). When only
runtime/other-session files remain untracked, list them and offer a
`.gitignore` line rather than committing them unasked — the owner said
yes, so those ignores are now in place (`4448d4e`, pushed): only the
parallel session's files (`memories/super-agent-*.md`,
`scripts/refresh-cruiserintelligence-blend1.*`) should ever appear
untracked after a clean session.

**How to apply:** re-run the same `finalize` on the candidate and copy
the HTML over `docs/architecture/`; re-shoot the PNGs with
`tmp/diagram-shoot.js` (playwright-core from hermes, chromium-1234 shell,
deviceScaleFactor 2, `fullPage: true`).

**Gotchas learned:**
- Architecture `connections` take no `role` field (workflow edges do).
- Fix the leading empty band by shifting every `pos[1]` (and each `via`
  y) up — the canvas is intrinsic.
- A re-finalize with an existing `*.browser-check.json` is refused
  ("evidence-path-conflict"); pass `--out-dir <folder>/review-2`.
- Sequence limits: `segments` take no `variant`; messages need ≥28px
  spacing and y ≥ 160; viewBox width must ≥ the participants' need
  (~105 px each) AND ≤ ~1007 px for 6.5px text to project above 6px at
  1440; omit `viewBox` only to let height grow, never width.
- A **workflow** (lane × column grid) could not route the turn's
  tool/model fork back to the agent without crossings after four
  attempts — a sequence draws a fork-and-return naturally; pick the
  diagram by the shape of the answer, not by "process".
