---
name: ernie-evolve-deep-dive-2026-11-10
description: "project 2026-11-10 — Baidu's ERNIE Evolve read against fleet's self-improvement loop AS BUILT; landed as docs/architecture/deep-dive-2026-11-10-ernie-evolve.md (3059ba8, docs-only); four improvement areas led by the evaluator-inside-the-tree gap; the companion to the 2026-11-07 DoubleAI dive, read against the completed C0–C9 program"
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

The owner asked (2026-11-10) for a deep dive on
https://ernie.baidu.com/research/posts/ernie-evolve/ and a comparative
analysis of fleet's self-improvement mechanics — *"what we learn and
improve?"* Landed as
`docs/architecture/deep-dive-2026-11-10-ernie-evolve.md`, committed
`3059ba8` (docs-only — the gate skipped per hard rule 2), tower task
`t-20261110-112541-3910` done.

**The article.** Software R&D as a recursive self-improvement loop with
five roles: the **Director** (picks direction + starting version from an
Experience Bank + Knowledge Base), the **Selector** (picks a candidate
lane-tree node, states the hypothesis, weighs scores/failures/prior
decisions), the **Executor**, the **Evaluator** — a fixed program
**kept outside the optimized repo, invisible to agents**, accepting
gains only above a **noise threshold** — and the **Reviewer** (rejects
intent-departure and eval-loop exploitation). Parallel research lanes,
each a tree of candidate versions; failures are first-class experience
(unsuccessful approaches never retried without new justification).
Results: beats public bests on NanoChat/NanoGPT/SOL-ExecBench; 623.6%
of baseline generation speed in four production rounds.

**Companion framing.** The 2026-11-07 DoubleAI dive
([[self-improvement-deep-dive-2026-11-07]]) read the article against
fleet's loop *as designed*; its plan landed the same day (C0–C9
complete), so this dive reads ERNIE Evolve against the loop *as built*
— demand map, clustering, twin/sham race, analyze, session beats,
eval --gaps, capability library all verified at their lines.

**The four improvement areas** (doc body, each with fleet file:line):

1. **The evaluator lives inside the tree a proposal can edit** — the
   structural one. ERNIE's core rule is the evaluator outside the
   optimized repo; fleet's `patch` op writes any repo-relative path
   (`stage_patch` `src/improve.rs:2241` refuses unsafe paths,
   `memories/`, dirty targets — never `src/eval.rs` or its tests). The
   gate passes (a weakened scenario IS the test), the corpus step
   grades against the softened instrument, and a "measured gain" is
   indistinguishable from an instrument rewrite by score alone; the
   race only helps when run, and an instrument rewrite raises the
   applied arm exactly like a real gain. Fix in fleet's idiom: a
   **protected-path set in `stage_patch`** (corpus sources, fixtures,
   pinning tests, the improve gate itself) refused by name — the
   `memories/` refusal's shape — with `fleet eval --promote` left as
   the one legitimate door to grow the corpus. The single repo cannot
   be literal "invisible to agents" (hard rule 12); honest form is
   *unwritable through the self-modification door*.
2. **Declared target vs patched path never meet** — a skill proposal's
   name mismatch is refused (`improve.rs:1969`) but a `patch` can
   declare `target-name: src/agent.rs` and write `src/eval.rs`; the
   drift guard stamps the patch's OWN path (staleness, not scope).
   ERNIE's Reviewer role; fix is strict equality for
   `target: source` proposals (few lines, in-repo precedent).
3. **Refusals are invisible to the next filing** — lessons are written
   **only on apply** (`finish` `improve.rs:1826`, lesson at `:1835`);
   a fresh proposal targeting a file with two prior refusals gets the
   applied history only. `last_refusal` rides its OWN proposal's file
   (`:1875`, card `:2670`). ERNIE refuses retrying failures without
   new justification. Fix: the filing confirmation (where
   `lessons_for:745` already runs) also lists recent refused/waiting
   proposals for the same target with their `last_refusal`.
4. **An applied proposal is never settled against reality** — measured
   once on the deterministic corpus at apply, the declared `effect`
   never read back; `measured` means *measured on the corpus*
   (`Lesson::from_proposal:630`). ERNIE closes every candidate through
   real workloads. Fleet owns the pattern: `grade_judgments`
   (`calib.rs:1085`) grades a verdict by the row that FOLLOWED it —
   apply the same shape to proposals (demand trend of the targeted
   class, N days later; no outcome = no rate). Open question: the
   target→demand-class mapping (some proposals honestly grade
   unmeasurable).

Plus: **the demand map drops `propose` rows** (`demand.rs:200` reads
five families, not OP_PROPOSE) — the loop's own refusal rate has no
bad-rate/trend/thin flag; areas 3 and 4 both want it.

**Named as ahead:** the twin/sham race (causal control ERNIE lacks),
promotable gaps (ERNNIE's roadmap item, shipped), determinism as
perfect noise control (DEMAND_TREND_EPSILON is where fleet will need
ERNIE's epsilon — the real-workload readback), one artifact read by
five instruments. **Non-adopted deliberately:** the autonomous
direction pick (the owner's ruling — the Director's job stays theirs;
fix the printed evidence, don't automate the decision) and parallel
lanes/version trees (the article's answer to ~50k candidates; fleet's
sequential gate-plus-git IS the tree at this scale).

**Why:** ERNIE Evolve's evaluator-isolation problem is *worse* for
fleet than for ERNIE — fleet optimizes itself, so instrument and
artifact are the same file — and the other three areas are the loop's
own evidence not reaching its own next decision.
**How to apply:** the dive is the design record. **Areas 1–2 now ride
a drafted plan as its C7** (the evaluator wall in `stage_patch`,
refused for every mode including `--approve`, `--promote` the corpus's
one door) — see [[improvement-modes-inbox-plan-drafted-2026-11-10]];
areas 3–5 stay queued as their own plan when the owner asks. Area 1
was the right first pick: "every other measure in the loop reads
through the instrument area 1 would let a proposal rewrite."
Related: [[self-improvement-deep-dive-2026-11-07]],
[[self-improvement-program-merged-and-delivered-2026-11-07]],
[[the-resource-contract-admit-work-settle-curtail]] (settlement is the
pattern area 4 reuses).
