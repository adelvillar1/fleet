---
name: self-improvement-deep-dive-2026-11-07
description: "project 2026-11-07 — the DoubleAI \"can LLMs redesign themselves as they train?\" article read against fleet's self-improvement machinery; the comparison landed as docs/architecture/deep-dive-2026-11-07-self-improvement.md (4c820fb); eight improvement areas led by the demand map; the program plan is ACTIVE and being implemented on branch `self-improvement` in its own worktree (see [[self-improvement-program-implemented-2026-11-07]])"
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

The owner's steer (2026-11-07): *"we are going to repeat the same pattern we did but with the self improving nature of fleet and this article"* — the second article-vs-fleet deep dive, after the Anthropic memory one ([[memory-dreaming-comparison-2026-11-07]]). The article: DoubleAI's "Can LLMs redesign themselves as they train?" (2026-11-06) — an agent inside a training run doing **mechanistic intervention**: the **phantom gradient** (∂L/∂h per token/layer, a map of unmet demand; centred it encodes what the context asked for), **code layers grown behind zero-init gates** (a discrete edit made continuous; useful layers open in ~100 steps), **twin/sham races** (identical run with the layer off; scrambled information — rules out luck and raw capacity), **evolution is mandatory** (the same layer inserted halfway beats present-from-step-0, 72% vs 55% recall), **jaggedness** (CLRS-Text: code layers raise in-between-length accuracy 33%→79%; twice the training does not close the gap), and the **self-probe** (DeepSeek-V4.1-Flash fitted a linear probe on its own residual stream predicting wrong answers, then resampled them — 64%→71% on hidden contests; Qwen3-8B wrote its own stop/restart/reject policy — 59%→66%, and the critical ingredient was writing ANALYSIS CODE over its own graded attempts before writing the policy).

**Where fleet already lives the principles:** alignment-through-code is fleet's founding ruling (mechanism-constrained writes, gated proposals); the eval corpus is the objective test the article says interpretability lacks; the failure-shape taxonomy is the article's "name the failure mode" step; the dream pass is the evidence-validated intervention.

**The eight improvement areas** (the doc's body, each with fleet file:line): (1) the **demand map** — the phantom gradient's fleet analog; the calib stream already records per-decision verdicts (route_judged, delegate, eval, validate rows) but nothing aggregates which classes stay high-error across runs; (2) **failure clustering → shape proposals** — the taxonomy grows by hand; the judge's verdicts and the dream pass's contradictions are the un-clustered raw material; (3) **the self-probe acting per decision** — fleet's probes are pre-decision (route) or post-hoc (judge/eval); the curate judgment is the one that acts per-steer; (4) **twin/sham races for proposals** — the gate rules out breakage, not placebo; the eval corpus can race apply-vs-no-op; (5) **analysis-before-policy** — Qwen's critical ingredient; fleet's proposals carry a justification but no machinery measures thresholds first; (6) **timing** — the article's mid-training insertion beats from-scratch against the beats' day-end batching; (7) **jaggedness** — fleet never tests the in-between scenarios; (8) **the growing tool library** — landed proposals aren't mined for reusable capabilities. Area 1 is the one the other seven would read from.

**Why:** the article's mechanisms map onto fleet's self-grading/self-modifying problem more directly than the memory article's did — both are about when and how a system may edit itself while it runs.
**How to apply:** the doc is the design record; the program plan for the eight areas is ACTIVE and being implemented on branch `self-improvement` in the worktree — [[self-improvement-program-implemented-2026-11-07]]. The owner-ruling tensions are named in the doc: the article's agent acts autonomously inside the loop while fleet's ruling is proposals-filed/owner-applies/destructive-waits, and the zero-init gate is a mechanism for a system whose optimizer can open it — fleet's discrete gate is the honest form at this autonomy level. Related: [[memory-dreaming-comparison-2026-11-07]], [[memory-dreaming-plan-implemented-2026-11-07]], [[the-judgment-beat-socket-guard]].
