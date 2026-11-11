---
name: self-improvement-program-implemented-2026-11-07
description: "project 2026-11-07 — SUPERSEDED the same day by [[self-improvement-program-merged-and-delivered-2026-11-07]]: the self-improvement program (C0–C9) is COMPLETE, MERGED to master at f6f5f0d, and DELIVERED (dmg rebuilt, app installed, face relaunched). Survives as the mid-flight record of C0–C6's landing"
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

The owner's steer (2026-11-07): *"implement the plan as designed using a worktree to avoid collisions with other sessions"* — the program plan of [[self-improvement-deep-dive-2026-11-07]] is **ACTIVE** (`docs/plans/2026-11-07-self-improvement.md`, committed `1837902` on `face-plugin-host`), and the work runs on branch **`self-improvement`** in the worktree **`/Users/alejandrodelvillar/Projects/fleet-self-improvement`** (created from master + a merge of the branch, `7013a40`) — the owner's own plan-worktree convention, so the parallel sessions' trees are untouched.

**Landed, each live-proven on the real stream:**
- **C0 the demand map** (`c11ef8d`): `src/demand.rs` reads the calib stream into per-class demand (route workload×provider, eval scenarios, delegation outcomes, validate verdicts) with rate, row count, and a halves-based trend (improving/flat/worsening/thin under `DEMAND_MIN_ROWS`); `fleet demand` prints it and lands one `demand` row. Live: 25 classes, top demand `subagent-delegation` 36% bad and worsening.
- **C1 the clustering beat** (`4e35f67`): `fleet demand --cluster` groups the window's failures, surfaces each cluster's examples, and files a shape proposal per cluster at or over `CLUSTER_MIN_BAD` (5) — a source Patch joining the tag to `FAILURE_SHAPES` + `FAILURE_SHAPE_KEYWORDS`, owner-applied, the class-derived word as the draft name. Live: 7 clusters filed, and every example named the SAME root cause — "the mock-server port never arrived" wearing seven class masks.
- **C4 analysis before policy** (`ca86897`): `fleet improve analyze` measures the routing judgments per class — the failure rate and the confidence threshold that separates right from wrong (misclassification-minimising, strictest tie-break; one-sided classes carry no threshold); the `improve` tool's declaration names it as the first step.
- **C6 the gaps** (`0a17f5d` + `bba79b8`): `fleet eval --gaps` reports the stream's failure classes with no corpus scenario; `--promote <class>` writes a scenario seed under `<FLEET_HOME>/eval-gaps/` from the class's own examples. Live: the delegation outcomes (failed 4 of 4) and the validate verdicts carry no coverage.
- **C5 the beats' scopes** (`76ccaf5`): `fleet judge --run <input-hash>` grades one run's route rows instead of the day; `fleet delegate --session <id>` judges one session's delegations (native — the rows carry the session id). The judge's session scope would need a row link the schema does not carry; the run scope is the honest form there.

**In flight:** C7 the capability library (the applied proposals rendered as capabilities through `fleet improve --capabilities`, and the skill drafter's evidence naming the capabilities already established for the target kind) — the code is in, the tests were mid-fix.

**Open:** C2 the self-probe re-run, C3 the twin/sham race, C8 the gate, C9 the docs.

**Why:** the article's argument for the phantom gradient — every other instrument reads the demand map — is the program's own sequencing, and the worktree keeps a multi-session program landable one phase at a time without colliding with the parallel sessions.
**How to apply:** continue on `self-improvement` in the worktree; each phase lands as its own verified increment (scoped test runs, one gate per push — [[house-build-loop-sticky]]). The socket guards bit twice during C5's tests (a judged-row construction in eval.rs and improve.rs — the fixtures are hand-written JSON lines, the route.rs precedent; [[the-judgment-beat-socket-guard]]). Related: [[self-improvement-deep-dive-2026-11-07]], [[self-improvement-program-plan-drafted-2026-11-07]], [[memory-dreaming-plan-implemented-2026-11-07]].
