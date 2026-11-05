---
name: curator-arc-recap-2026-10-30
description: "The second arc of 2026-10-30 closed — recap at docs/recaps/2026-10-30-the-curator.md, pushed 7c1eaef..e099d40 to origin/master; ledger 377/33, suite 1,838, and the open debts (vision, TABPFN, the failed corpus job)"
type: project
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

The curator arc of 2026-10-30 (the day's second arc, after [[session-recap-2026-10-30-knowledge-group-on-the-rail]]) is closed and pushed: recap at `docs/recaps/2026-10-30-the-curator.md`, committed e099d40, pushed `7c1eaef..e099d40` to origin/master. Ledger 377 rows / 33 open; suite 1,838 passed / 0 failed across all targets.

The arc: the router/dev-decisions/lanes deep dive → the surface-curation plan (7b96232) → the curator landed (a7007a4, KI-375/376/377) → the K5 real-session replay (babb109) → the curator-judgment plan (50e807f) → landed advisory (1731faf) → the owner's award (b16597a) → the three-judge panel (caa5f2f).

**Open debts carried out of the arc, none covered by a plan yet:**
- The **vision workload always errors** — `Client::routed` + the sync `Client::chat`, which has no router branch (`src/api.rs:603` vs `:628`); the routed client's `base_url` is empty so the sync path POSTs nowhere.
- **`measure` is starved of `TABPFN_API_KEY`** while blend1's tabular leg works — the two key universes (`~/.config/sdm1/env` vs `~/.fleet/.env`; only blend1 reads its siblings' files).
- **`corpus-refresh-dev-decisions` failed its 2026-10-30T00:50Z run** (runs 7, ok 6) — the only corpus job to fail that night; the store itself was current, so the cost was nothing, but the schedule is not reliable.
- The seat's production behavior is the calib `curate` rows' to judge — the J6 replay showed the panel under-confident on glide (3/4 votes under the 0.6 gate), so most steers still let the cosine decide; if the gate never passes, the panel's confidence calibration is what to revisit, and `CURATE_JUDGED_DECIDES = false` is the one-line rollback.

Related: [[curator-judgment-plan-2026-10-30]], [[surface-curation-plan-2026-10-30]], [[router-lanes-deep-dive-defects-2026-10-30]].
