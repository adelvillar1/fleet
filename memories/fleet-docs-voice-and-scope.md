---
name: fleet-docs-voice-and-scope
description: "owner rulings on documentation (2026-10-21) — README and FUNCTIONAL speak product-manager, TECHNICAL speaks technical-program-manager; docs carry the what and the why, never the how (that is the code's job); goal/intent/solution, never growing pains"
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

Owner corrections on the documentation revamp, 2026-10-21:

- **Registers:** README.md and FUNCTIONAL-SPECIFICATIONS.md speak the
  language of a **product manager** — what the product does for the user,
  the jobs, what is shipped versus in flight. TECHNICAL-DOCUMENTATION.md
  speaks the language of a **technical program manager** — systems,
  interface contracts, environments, runbooks, risk, sequencing.
- **Scope:** "the what and the why. the how, it's in the code." Do not
  transcribe implementation into the docs.
- **Content:** "the goal, the intent, and the solution is what matters" —
  no vendoring history, no drift, no unfinished work as narrative. And
  "focus on the functionality, not on the mechanics": understand and
  describe surfaces by what the user lands on and does, not by internal
  machinery (the cron trail was cut mid-investigation for this).
- **Evidence:** understand the user-facing surfaces by landing on them
  for real — the owner directed Playwright against the running face and
  screenshots "as needed" — and quote each surface's own sentence of
  purpose (they were written to be the docs' language).

**Why:** the docs are the product's front door; the owner reads them
first, and a doc that narrates the build process reads as immaturity,
not candor.

**How to apply:** when writing or revising docs here, ask what the
surface is *for* and which principle it makes visible, quote its own
copy, and leave mechanisms to the code and the architecture deep-dive.
The substrate produced 2026-10-21: docs/architecture/surfaces.md (the
what and why of every rail destination) and docs/architecture/lanes.md
(the five-lane case). See also [[plan-brainstorming-before-development]]
(approve the voice before rewriting) and [[the-harness-voice-is-best-of-both]].
