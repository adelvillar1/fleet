---
name: self-improvement-program-plan-drafted-2026-11-07
description: "project 2026-11-07 — the program plan for the eight self-improvement areas is DRAFTED and gated (docs/plans/2026-11-07-self-improvement.md, status draft, uncommitted); arch-gate PASS, plan-gate judgment-band flags waived by disposition; awaiting the owner's approval before C0 starts"
type: project
date: 2026-11-11
basis: inferred
tier: durable
shape: []
phrases: []
missing: []
premises: []
recall-count: 0
superseded-by:
valid-until:
source: zcode:fleet:content:285983102772d99aaed2c496d57acdf6f320aad7347d3891e9bb9e2378839d34
---

The owner's steer (2026-11-07): *"create a plan to implement your 8 suggestions"* — the program plan for the eight improvement areas of [[self-improvement-deep-dive-2026-11-07]], at `docs/plans/2026-11-07-self-improvement.md`. **Status: draft, uncommitted, awaiting the owner's approval** (the skill's step 8 — not marked active, not started).

**The shape:** ten criteria C0–C9, one per area plus the gate and docs, sequenced in dependency order so each phase lands as its own verified increment. C0 the **demand map** first and alone (new `src/demand.rs` reader aggregating the calib stream into per-class demand — rate + trend — with `fleet demand` printing it and one `demand` calib row); C1 repeated failures → shape proposals (the clustering beat); C4 analysis-before-policy (`fleet improve analyze` — thresholds measured from the stream before a proposal's criteria are written); C2 the self-probe acting per decision (a confidently-wrong delegation re-run, logged and bounded); C3 the twin/sham race (`fleet improve race <proposal>` — applied vs reverted vs scrambled, placebo refused by name); C6 the gaps (`fleet eval --gaps` — classes the stream shows but the corpus doesn't cover, promotable into scenarios); C5 the beats' session scope (timing); C7 the capability library (applied proposals rendered as capabilities, feeding the skill drafter); C8/C9 the gate and the docs.

**Direction lock:** the demand map first (the others read it); fleet's interventions stay owner-gated — the one exception is C2's re-run seam, which re-dispatches a child the owner already approved, logged as its own row and bounded by the same `max-turns`; measured-before-acted (each phase carries its bar); no new store (the calib stream, the eval corpus, and the proposal store are the only substrates).

**Gate status, honestly:** arch-gate **PASS** with 2 conventions gaps (the demand row vocabulary and the face section — offered as doc updates). Plan-gate: two flags standing on the gate and docs criteria (p=0.34 / 0.44, stable in the judgment band with the assertions carried verbatim in the Approach and the Verification section) — waived by disposition, same as the memory-dreaming plan ([[plan-gate-structure-and-association-traps]]). The plan-surface's fuzzy map (C6→src/delegate, C0→src/judge) was overridden by disposition; its one strong edge (C0 before C5) matches the plan's stated order.

**Why:** the article's argument for the phantom gradient — every other instrument reads the demand map — is the plan's own sequencing, and the phasing makes a multi-session program landable one phase at a time.
**How to apply:** when the owner approves, flip the plan to `active` and start C0 alone (it is the others' reader). The socket-guard shape applies to the new modules (the lane-owning wrappers live in `src/demand.rs` and a sibling, never named from the scanned synchronous paths — [[the-judgment-beat-socket-guard]]); the preflight names the guards ([[house-build-loop-sticky]]). Related: [[self-improvement-deep-dive-2026-11-07]], [[memory-dreaming-plan-implemented-2026-11-07]].
