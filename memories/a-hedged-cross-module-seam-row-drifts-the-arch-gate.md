---
name: a-hedged-cross-module-seam-row-drifts-the-arch-gate
description: "feedback 2026-11-02 — an arch-gate DRIFT on a plan's Files section was caused by a hedged seam row reaching into a module whose documented contract is narrow (judge.rs = the route-grading verb only); cite the source PLAN for a seam reuse, not the file, and distinguish stable drifts (fix the wording) from one-run variance (re-run before treating as a verdict)"
type: feedback
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

The usage plan's Files section carried "`src/judge.rs` (the session seam reuse, if the shape differs from the beats')" — a hedge reaching into a module whose TECHNICAL-DOCUMENTATION.md contract is one verb ("the routing judgment beat: `fleet judge` grades a day's route decisions through sys1. The grader sees redacted route rows only"). The arch gate DRIFT'd on exactly that section, **stably across three runs** (documented≈0.76, conflict≈0.64), while every individual file row read CONFORMS — the drift is the section-level claim, not a row.

**Why:** the gate keys a Files section against the documented module contracts; a row that says a module will host work outside its documented contract is a real architecture claim, and hedging ("if the shape differs") does not soften it — it makes the claim unfalsifiable.

**How to apply:**
- A seam reused from another feature should cite its **source plan** (the artifact that owns the contract), not the file that happens to host the implementation today: "the lane-ask seam is reused from the judgment beats' implementation per `2026-10-23-judgment-beats-one-lane.md`, wherever it lives" — the rewrite flipped the verdict to **PASS, stable**. Dispositioned `fixed`.
- Before treating an arch-gate DRIFT as real, **re-run it twice**: the session-parity-remainders plan's single DRIFT never reproduced (model variance — one run's 0.64 conflict, next run CONFORMS, dispositioned waived); the usage plan's did (three identical verdicts). Variance gets waived with the variance named; a stable drift names a real wording problem.
