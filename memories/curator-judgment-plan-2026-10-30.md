---
name: curator-judgment-plan-2026-10-30
description: "The curator-judgment plan (C4) LANDED 1731faf; the owner AWARDED the seat (b16597a, CURATE_JUDGED_DECIDES=true) and made the vote a PANEL (caa5f2f): CURATE_JUDGES glide/drex/jev fanned out in one call, best of 3 wins, tie = no verdict"
type: project
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

The owner took the landed surface-curation plan's criterion (b) on 2026-10-30 ("once completed create a plan for the deferred curator") — plan written at `docs/plans/2026-10-30-curator-judgment.md` (50e807f) and **IMPLEMENTED 1731faf, status completed** — with the J6 contingency firing: the replay (real session 997a54b4's four steers through a real sys1 child, glide ~800ms/judgment) showed IDENTICAL decisions to cosine-only, zero mis-fires, but no domination — three of four votes under the 0.6 gate (0.174/0.536/0.571), no early fire, no focus fix. The owner then AWARDED the seat (b16597a: "award the seat and we roll it back if needed") — `CURATE_JUDGED_DECIDES = true` and the vote is a PANEL (caa5f2f: CURATE_JUDGES = glide/drex/jev fanned out in one classify call via provider list, best of 3 wins — majority vote, winning side's MEAN confidence meets the 0.6 gate, focus/staleness strict majorities, a tie is no verdict → fail-open; the row/cache carry the judges/yes/no split), a confident vote fires and holds, the cosine stays safety-net + fail-open, the advisory behavior stays pinned for the one-constant rollback, and the calib `curate` rows are the evidence base; the focus hint still acts (known_shape-validated assemble_with_shape hint). Use case A (the early fire) is delivered by the award, not the replay. Gate 1,649 + 189 green.

The shape: one `curate` task in the vendored sys1 (heads `needs_recall` yes/no, `focus` over the 7 structure + 6 failure shapes + none, `staleness` fresh/stale), cached in route-cache.json under a second namespace with the same producer key, composed in `maybe_curate` over a four-row table: first steer fires; confident yes fires (early — before cosine would); confident no holds UNLESS cosine < floor (cosine = safety net, lane = policy); unavailable/no-keys/low-confidence = cosine decides, byte-identical fail-open (the `sys1_judges` gate checked before any spawn, KI-044's lesson). `apply_steer` went async. Focus acts through `assemble_with_shape` (a hint that beats `classify_shape`) or it is dead weight. `Record::Curated` gained `trigger` + `focus_shape`. Calib `curate` rows (the `explain` row's shape) make it gradeable.

The honesty spine: the K5 replay showed the cosine floor mis-firing on ZERO real pairs, so criterion (a) was NOT met — the judgment buys policy, not repair; J6's contingency shipped advisory and the owner's award flipped it. The composition is the pure `curate_decision(first, judged, cosine, judged_decides)` — both paths pinned row by row, so the rollback is one flip of the constant.

Text contract names BOTH ends (last-curated instruction + new steer) — a judgment of the MOVE, not a topic classifier. Cache: second map in route-cache.json, one rekey clears both namespaces.

Increments J0-J3 (task, verdict+cache, composition+focus, measurement). Related: [[surface-curation-plan-2026-10-30]] (the landed plan), [[router-lanes-deep-dive-defects-2026-10-30]], [[owner-awards-safe-upgrades-with-a-rollback]].
