---
name: harness-compositional-generalization-paper-vs-fleet
description: "The Zhang/Khattab harness-as-compositional-generalizer paper validates Fleet's delegation-as-RLM shape; the one adoptable idea is trajectory-shape instrumentation on the calibration stream, not new machinery."
type: design
date: 2026-11-02
basis: inferred
tier: durable
shape: [docs]
phrases: [compositional generalization, locally in-distribution, RLM, recursive language model, context rot, context offloading, trajectory shape, harness paper, alexzhang13]
missing: [the trajectory-shape reader (unbuilt; extends the usage plan's rows)]
superseded-by:
valid-until:
---

Source: https://alexzhang13.github.io/blog/2026/harness/ (Zhang & Khattab, MIT CSAIL, Jul 2026). Thesis: harnesses that keep every LM call locally in-distribution (LID) induce equivalence classes over tasks, so RL on short tasks transfers 8–32× longer; the RLM testbed = context offloading + programmatic sub-calls in a REPL; context rot (appended tool outputs bloating history) is the named failure mode; the paper itself warns hand-coded strategies run afoul of the bitter lesson.

Fleet mapping (2026-11-02): Fleet already embodies the paper's two mechanisms independently — delegation to depth 2 IS the RLM's recursive sub-call with a fresh locally-clean context (the strongest LID move in the codebase), and windowed reads + capped outputs + the memory substrate + skills progressive disclosure ARE offloading; compaction answers context rot. The Mismanaged Geniuses Hypothesis is the owner's own delegation rationale. Same validates-don't-threatens verdict as the offloop article ([[super-agent-article-vs-fleet]]).

Adoptable (ranked 1, the only one): TRAJECTORY-SHAPE INSTRUMENTATION. The usage plan (2026-11-02-usage-in-calibration-and-stream-anomalies, draft) lands per-request token accounting; nothing records the per-run shape — token growth across turns, compaction events, delegation fan-out. A `fleet calib shape`-style reader over those rows makes "is Fleet's context rotting" measurable per KI-249's law before anyone builds preventive compaction or an offload convention. Small, extends the usage plan's rows, measures first.

Rejected, with reasons: giant-input offloading replacing the 64KB wire cap (a deliberate spend bound, KI-378's class; revisit only if calibration shows real tasks hitting it); a first-class REPL orchestration (bash+filesystem already give variable-like state; an overhaul the inference-time evidence doesn't demand); the decomposition nudge (bitter-lesson-adjacent at runtime; the paper's hint was a training aid); the RL apparatus itself (Fleet doesn't train; the 8–32× claims are about trained RLMs, not harness swaps on routed mid-tier models).
