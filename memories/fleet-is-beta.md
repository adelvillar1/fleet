---
name: fleet-is-beta
description: "Fleet moved from alpha to beta on 2026-10-30 by the owner's declaration — 0.3.0-beta, the wanted surfaces are all there, and the shakeout is the work mode now."
type: project
date: 2026-10-30
basis: confirmed-by
shape: [change]
recall-count: 0
superseded-by:
valid-until:
---

The owner's words (2026-10-30): "I think we can call fleet in beta
stage. What I wanted in the app is there and looks and works fine. Time
to find bugs." The crate version is `0.3.0-beta`; CLAUDE.md's status
line carries the declaration.

The wanted surfaces are all there and working — the eight rail pages
(settings, bots, skills, memory, workspace history, calibration, plus
the conversation and the second brain), the landing's greet, dynamic
conversation names, the remote relay with both faces, the substrate's
reads and writes from the browser.

The work mode is the **shakeout**: adversarial passes over the
surfaces, the flows, and the seams, with every find filed in
`docs/known-issues.md` — the ledger discipline, every error logged, not
ignored. Fixes land before additions; new features wait for a bug
budget.
