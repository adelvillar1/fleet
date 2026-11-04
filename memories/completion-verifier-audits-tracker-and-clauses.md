---
name: completion-verifier-audits-tracker-and-clauses
description: "feedback 2026-10-30 — the completion verifier refused a landed implementation for two things the artifacts didn't show — a stale todo list contradicting the transcript, and a criterion checked [x] while one of its own clauses ("at least one real session replayed") was unmet; close the tracker out and never check a criterion whose every clause isn't evidenced"
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

feedback 2026-10-30 — the surface-curation plan landed green (a7007a4,
gate 1,645 + 188, KI rows filed) and the completion verifier still
refused it, on two grounds that had nothing to do with the code:

1. **The todo list went stale.** C3/docs/gate were verifiably done but
   the tracker still showed one in_progress and two pending — the
   tracker contradicted the transcript.
2. **An overstated `[x]`.** K5 read "`DRIFT_FLOOR` is chosen from
   measured pairs (the `gui/related` bands, **plus at least one real
   session replayed**) and recorded here" — the floor was measured
   against purpose-built pairs, the real-session clause was unmet, and
   the checkbox was checked anyway.

**Why:** a criterion is a conjunction of clauses, and a verifier reads
them all; a proxy signal (passing tests, a landed commit) covers the
artifact, not the tracker or the sentence. Same family as
[[a-plan-marked-met-can-hide-missing-behaviour]] and
[[a-green-gate-covers-what-was-built-not-what-was-left-out]].

**How to apply:**

- Mark every todo completed the moment its work is verified — the list
  is part of the deliverable's state, not scratch.
- Before checking any plan criterion, re-read it clause by clause and
  ask what evidence each named clause has; uncheck and finish the
  clause (the K5 replay ran against real session `997a54b4`'s four
  steers, babb109) rather than let the check stand.
- When a landed implementation makes a use case the *deferred seat's*
  promise (the curator-judgment plan's early-fire use case A died with
  the advisory verdict), uncheck it in the same landing and say so —
  an honest unchecked box is the record; a checked one is a lie the
  next audit re-finds.
